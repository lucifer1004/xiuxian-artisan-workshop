use arrow::record_batch::RecordBatch;
use serde_json::Value;
use xiuxian_vector::attach_record_batch_metadata;
use xiuxian_wendao_core::{
    capabilities::{ContractVersion, PluginCapabilityBinding},
    repo_intelligence::{RegisteredRepository, RepoIntelligenceError, RepositoryPluginConfig},
    transport::{PluginTransportEndpoint, PluginTransportKind},
};
use xiuxian_wendao_runtime::transport::{
    DEFAULT_FLIGHT_BASE_URL, DEFAULT_FLIGHT_TIMEOUT_SECS, FLIGHT_SCHEMA_VERSION_METADATA_KEY,
    NegotiatedFlightTransportClient, negotiate_flight_transport_client_from_bindings,
    normalize_flight_route, validate_flight_schema_version, validate_flight_timeout_secs,
};

use super::capability_manifest::discover_julia_graph_structural_binding_from_manifest_for_repository;
use super::graph_structural::{
    GraphStructuralRouteKind, validate_graph_structural_filter_request_batch,
    validate_graph_structural_filter_response_batch,
    validate_graph_structural_rerank_request_batch,
    validate_graph_structural_rerank_response_batch,
};
use crate::compatibility::link_graph::julia_graph_structural_provider_selector;

const JULIA_PLUGIN_ID: &str = "julia";
const GRAPH_STRUCTURAL_TRANSPORT_KEY: &str = "graph_structural_transport";
const STRUCTURAL_RERANK_TRANSPORT_KEY: &str = "structural_rerank";
const CONSTRAINT_FILTER_TRANSPORT_KEY: &str = "constraint_filter";
const DEFAULT_JULIA_HEALTH_ROUTE: &str = "/healthz";

/// Build a Julia Flight transport client for one graph-structural route kind.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the repository config contains an
/// invalid graph-structural transport block or cannot be negotiated into a
/// Flight client.
pub fn build_graph_structural_flight_transport_client(
    repository: &RegisteredRepository,
    route_kind: GraphStructuralRouteKind,
) -> Result<Option<NegotiatedFlightTransportClient>, RepoIntelligenceError> {
    let Some(binding) = build_graph_structural_flight_transport_binding(repository, route_kind)?
    else {
        return Ok(None);
    };

    negotiate_flight_transport_client_from_bindings(&[binding]).map_err(|error| {
        RepoIntelligenceError::ConfigLoad {
            message: format!(
                "failed to build Julia graph-structural Flight transport client for repo `{}` and route `{}`: {error}",
                repository.id,
                route_kind.route(),
            ),
        }
    })
}

/// Validate one staged graph-structural request batch set.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when any batch violates the staged
/// graph-structural request contract for the selected route kind.
pub fn validate_graph_structural_request_batches(
    route_kind: GraphStructuralRouteKind,
    batches: &[RecordBatch],
) -> Result<(), RepoIntelligenceError> {
    for batch in batches {
        validate_graph_structural_request_batch(route_kind, batch)
            .map_err(|error| graph_structural_contract_error(route_kind, "request", error))?;
    }
    Ok(())
}

/// Validate one staged graph-structural response batch set.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when any batch violates the staged
/// graph-structural response contract for the selected route kind.
pub fn validate_graph_structural_response_batches(
    route_kind: GraphStructuralRouteKind,
    batches: &[RecordBatch],
) -> Result<(), RepoIntelligenceError> {
    for batch in batches {
        validate_graph_structural_response_batch(route_kind, batch)
            .map_err(|error| graph_structural_contract_error(route_kind, "response", error))?;
    }
    Ok(())
}

/// Send graph-structural Arrow batches to one remote Julia Flight transport.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the request violates the staged
/// contract, the Flight roundtrip fails, or the response violates the staged
/// graph-structural response contract.
pub async fn process_graph_structural_flight_batches(
    client: &NegotiatedFlightTransportClient,
    route_kind: GraphStructuralRouteKind,
    batches: &[RecordBatch],
) -> Result<Vec<RecordBatch>, RepoIntelligenceError> {
    validate_graph_structural_request_batches(route_kind, batches)?;
    let request_batches = batches
        .iter()
        .map(|batch| attach_graph_structural_schema_version_metadata(batch, route_kind))
        .collect::<Result<Vec<_>, _>>()?;
    let response_batches = client
        .process_batches(&request_batches)
        .await
        .map_err(|error| RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "Julia graph-structural Flight request for route `{}` failed: {error}",
                route_kind.route()
            ),
        })?;
    validate_graph_structural_response_batches(route_kind, response_batches.as_slice())?;
    Ok(response_batches)
}

/// Resolve the repository's graph-structural Julia Flight transport client,
/// perform the remote Flight roundtrip, and validate the staged response
/// contract.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the repository does not declare a
/// usable graph-structural client, the roundtrip fails, or the response
/// violates the staged contract.
pub async fn process_graph_structural_flight_batches_for_repository(
    repository: &RegisteredRepository,
    route_kind: GraphStructuralRouteKind,
    batches: &[RecordBatch],
) -> Result<Vec<RecordBatch>, RepoIntelligenceError> {
    let client =
        build_graph_structural_flight_transport_client(repository, route_kind)?.ok_or_else(
            || RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "repo `{}` does not declare an enabled Julia graph-structural Flight transport client for route `{}`",
                    repository.id,
                    route_kind.route()
                ),
            },
        )?;
    process_graph_structural_flight_batches(&client, route_kind, batches).await
}

fn build_graph_structural_flight_transport_binding(
    repository: &RegisteredRepository,
    route_kind: GraphStructuralRouteKind,
) -> Result<Option<PluginCapabilityBinding>, RepoIntelligenceError> {
    let Some(options) = resolve_graph_structural_transport_options(repository, route_kind)? else {
        return discover_julia_graph_structural_binding_from_manifest_for_repository(
            repository, route_kind,
        );
    };
    build_graph_structural_flight_transport_binding_from_options(repository, route_kind, options)
}

fn build_graph_structural_flight_transport_binding_from_options(
    repository: &RegisteredRepository,
    route_kind: GraphStructuralRouteKind,
    options: GraphStructuralTransportOptions,
) -> Result<Option<PluginCapabilityBinding>, RepoIntelligenceError> {
    if let Some(false) = options.enabled {
        return Ok(None);
    }

    let route = match options.route {
        Some(route) => {
            normalize_flight_route(route).map_err(|error| RepoIntelligenceError::ConfigLoad {
                message: format!(
                    "repo `{}` Julia graph-structural route `{}` is invalid: {error}",
                    repository.id,
                    route_kind.route()
                ),
            })?
        }
        None => route_kind.route().to_string(),
    };
    let health_route = match options.health_route {
        Some(route) => {
            normalize_flight_route(route).map_err(|error| RepoIntelligenceError::ConfigLoad {
                message: format!(
                    "repo `{}` Julia graph-structural health_route for `{}` is invalid: {error}",
                    repository.id,
                    route_kind.route()
                ),
            })?
        }
        None => DEFAULT_JULIA_HEALTH_ROUTE.to_string(),
    };
    let schema_version = match options.schema_version {
        Some(schema_version) => validate_flight_schema_version(&schema_version).map_err(
            |error| RepoIntelligenceError::ConfigLoad {
                message: format!(
                    "repo `{}` Julia graph-structural schema version for `{}` is invalid: {error}",
                    repository.id,
                    route_kind.route()
                ),
            },
        )?,
        None => route_kind.schema_version().to_string(),
    };
    let timeout_secs = match options.timeout_secs {
        Some(timeout_secs) => validate_flight_timeout_secs(timeout_secs).map_err(|error| {
            RepoIntelligenceError::ConfigLoad {
                message: format!(
                    "repo `{}` Julia graph-structural timeout for `{}` is invalid: {error}",
                    repository.id,
                    route_kind.route()
                ),
            }
        })?,
        None => DEFAULT_FLIGHT_TIMEOUT_SECS,
    };

    Ok(Some(PluginCapabilityBinding {
        selector: julia_graph_structural_provider_selector(),
        endpoint: PluginTransportEndpoint {
            base_url: Some(
                options
                    .base_url
                    .unwrap_or_else(|| DEFAULT_FLIGHT_BASE_URL.to_string()),
            ),
            route: Some(route),
            health_route: Some(health_route),
            timeout_secs: Some(timeout_secs),
        },
        launch: None,
        transport: PluginTransportKind::ArrowFlight,
        contract_version: ContractVersion(schema_version),
    }))
}

fn attach_graph_structural_schema_version_metadata(
    batch: &RecordBatch,
    route_kind: GraphStructuralRouteKind,
) -> Result<RecordBatch, RepoIntelligenceError> {
    attach_record_batch_metadata(
        batch,
        [(
            FLIGHT_SCHEMA_VERSION_METADATA_KEY,
            route_kind.schema_version(),
        )],
    )
    .map_err(|error| RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "failed to attach Julia graph-structural schema metadata for route `{}`: {error}",
            route_kind.route()
        ),
    })
}

fn validate_graph_structural_request_batch(
    route_kind: GraphStructuralRouteKind,
    batch: &RecordBatch,
) -> Result<(), String> {
    match route_kind {
        GraphStructuralRouteKind::StructuralRerank => {
            validate_graph_structural_rerank_request_batch(batch)
        }
        GraphStructuralRouteKind::ConstraintFilter => {
            validate_graph_structural_filter_request_batch(batch)
        }
    }
}

fn validate_graph_structural_response_batch(
    route_kind: GraphStructuralRouteKind,
    batch: &RecordBatch,
) -> Result<(), String> {
    match route_kind {
        GraphStructuralRouteKind::StructuralRerank => {
            validate_graph_structural_rerank_response_batch(batch)
        }
        GraphStructuralRouteKind::ConstraintFilter => {
            validate_graph_structural_filter_response_batch(batch)
        }
    }
}

fn graph_structural_contract_error(
    route_kind: GraphStructuralRouteKind,
    direction: &str,
    message: impl Into<String>,
) -> RepoIntelligenceError {
    RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "Julia graph-structural {direction} contract `{}` for route `{}` violated: {}",
            route_kind.schema_version(),
            route_kind.route(),
            message.into()
        ),
    }
}

#[derive(Debug, Default)]
struct GraphStructuralTransportOptions {
    enabled: Option<bool>,
    base_url: Option<String>,
    route: Option<String>,
    health_route: Option<String>,
    schema_version: Option<String>,
    timeout_secs: Option<u64>,
}

fn resolve_graph_structural_transport_options(
    repository: &RegisteredRepository,
    route_kind: GraphStructuralRouteKind,
) -> Result<Option<GraphStructuralTransportOptions>, RepoIntelligenceError> {
    for plugin in &repository.plugins {
        let RepositoryPluginConfig::Config { id, options } = plugin else {
            continue;
        };
        if id != JULIA_PLUGIN_ID {
            continue;
        }

        let Some(transport) = options.get(GRAPH_STRUCTURAL_TRANSPORT_KEY) else {
            continue;
        };
        let transport = object_option(
            transport,
            GRAPH_STRUCTURAL_TRANSPORT_KEY,
            route_kind,
            repository,
        )?;
        let route_override = transport.get(route_kind.option_key());
        let route_override = match route_override {
            Some(value) => Some(object_option(
                value,
                route_kind.option_key(),
                route_kind,
                repository,
            )?),
            None => None,
        };

        return Ok(Some(GraphStructuralTransportOptions {
            enabled: route_override
                .map(|value| bool_option(value, "enabled", repository))
                .transpose()?
                .flatten()
                .or(bool_option(transport, "enabled", repository)?),
            base_url: route_override
                .map(|value| string_option(value, "base_url", repository))
                .transpose()?
                .flatten()
                .or(string_option(transport, "base_url", repository)?),
            route: route_override
                .map(|value| string_option(value, "route", repository))
                .transpose()?
                .flatten()
                .or(string_option(transport, "route", repository)?),
            health_route: route_override
                .map(|value| string_option(value, "health_route", repository))
                .transpose()?
                .flatten()
                .or(string_option(transport, "health_route", repository)?),
            schema_version: route_override
                .map(|value| string_option(value, "schema_version", repository))
                .transpose()?
                .flatten()
                .or(string_option(transport, "schema_version", repository)?),
            timeout_secs: route_override
                .map(|value| u64_option(value, "timeout_secs", repository))
                .transpose()?
                .flatten()
                .or(u64_option(transport, "timeout_secs", repository)?),
        }));
    }

    Ok(None)
}

fn object_option<'a>(
    value: &'a Value,
    field: &str,
    route_kind: GraphStructuralRouteKind,
    repository: &RegisteredRepository,
) -> Result<&'a Value, RepoIntelligenceError> {
    if value.is_object() {
        return Ok(value);
    }

    Err(RepoIntelligenceError::ConfigLoad {
        message: format!(
            "repo `{}` Julia graph-structural field `{field}` for route `{}` must be an object",
            repository.id,
            route_kind.route()
        ),
    })
}

fn string_option(
    value: &Value,
    field: &str,
    repository: &RegisteredRepository,
) -> Result<Option<String>, RepoIntelligenceError> {
    let Some(raw) = value.get(field) else {
        return Ok(None);
    };
    let Some(string) = raw.as_str() else {
        return Err(plugin_config_type_error(repository, field, "a string"));
    };
    Ok(Some(string.to_string()))
}

fn bool_option(
    value: &Value,
    field: &str,
    repository: &RegisteredRepository,
) -> Result<Option<bool>, RepoIntelligenceError> {
    let Some(raw) = value.get(field) else {
        return Ok(None);
    };
    let Some(boolean) = raw.as_bool() else {
        return Err(plugin_config_type_error(repository, field, "a boolean"));
    };
    Ok(Some(boolean))
}

fn u64_option(
    value: &Value,
    field: &str,
    repository: &RegisteredRepository,
) -> Result<Option<u64>, RepoIntelligenceError> {
    let Some(raw) = value.get(field) else {
        return Ok(None);
    };
    let Some(number) = raw.as_u64() else {
        return Err(plugin_config_type_error(
            repository,
            field,
            "an unsigned integer",
        ));
    };
    Ok(Some(number))
}

fn plugin_config_type_error(
    repository: &RegisteredRepository,
    field: &str,
    expected: &str,
) -> RepoIntelligenceError {
    RepoIntelligenceError::ConfigLoad {
        message: format!(
            "repo `{}` Julia plugin field `{field}` must be {expected}",
            repository.id
        ),
    }
}

impl GraphStructuralRouteKind {
    fn option_key(self) -> &'static str {
        match self {
            Self::StructuralRerank => STRUCTURAL_RERANK_TRANSPORT_KEY,
            Self::ConstraintFilter => CONSTRAINT_FILTER_TRANSPORT_KEY,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use arrow::array::{
        BooleanArray, Float64Array, Int32Array, ListArray, ListBuilder, StringArray, StringBuilder,
    };
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;
    use tokio::runtime::Builder;
    use xiuxian_wendao_core::{
        repo_intelligence::{
            RegisteredRepository, RepositoryPluginConfig, RepositoryRefreshPolicy,
        },
        transport::PluginTransportKind,
    };
    use xiuxian_wendao_runtime::transport::FLIGHT_SCHEMA_VERSION_METADATA_KEY;

    use super::{
        build_graph_structural_flight_transport_client, validate_graph_structural_request_batches,
        validate_graph_structural_response_batches,
    };
    use crate::plugin::graph_structural::{
        GRAPH_STRUCTURAL_ACCEPTED_COLUMN, GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN,
        GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN, GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN,
        GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN, GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN,
        GRAPH_STRUCTURAL_CONSTRAINT_KIND_COLUMN, GRAPH_STRUCTURAL_DEPENDENCY_SCORE_COLUMN,
        GRAPH_STRUCTURAL_EDGE_CONSTRAINT_KINDS_COLUMN, GRAPH_STRUCTURAL_EXPLANATION_COLUMN,
        GRAPH_STRUCTURAL_FEASIBLE_COLUMN, GRAPH_STRUCTURAL_FILTER_ROUTE,
        GRAPH_STRUCTURAL_FINAL_SCORE_COLUMN, GRAPH_STRUCTURAL_KEYWORD_SCORE_COLUMN,
        GRAPH_STRUCTURAL_PIN_ASSIGNMENT_COLUMN, GRAPH_STRUCTURAL_QUERY_ID_COLUMN,
        GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN, GRAPH_STRUCTURAL_REJECTION_REASON_COLUMN,
        GRAPH_STRUCTURAL_REQUIRED_BOUNDARY_SIZE_COLUMN, GRAPH_STRUCTURAL_RERANK_ROUTE,
        GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN, GRAPH_STRUCTURAL_SEMANTIC_SCORE_COLUMN,
        GRAPH_STRUCTURAL_STRUCTURAL_SCORE_COLUMN, GRAPH_STRUCTURAL_TAG_SCORE_COLUMN,
        GraphStructuralRouteKind, JULIA_GRAPH_STRUCTURAL_SCHEMA_VERSION,
    };
    use crate::plugin::test_support::official_examples::{
        reserve_real_service_port, spawn_real_wendaosearch_demo_capability_manifest_service,
        wait_for_service_ready_with_attempts,
    };

    #[test]
    fn build_graph_structural_flight_transport_client_returns_none_without_config() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
            ..RegisteredRepository::default()
        };

        let client = build_graph_structural_flight_transport_client(
            &repository,
            GraphStructuralRouteKind::StructuralRerank,
        )
        .unwrap_or_else(|error| {
            panic!("missing graph-structural config should be ignored: {error}")
        });
        assert!(client.is_none());
    }

    #[test]
    fn build_graph_structural_flight_transport_client_reads_common_options() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Config {
                id: "julia".to_string(),
                options: serde_json::json!({
                    "graph_structural_transport": {
                        "base_url": "http://127.0.0.1:9101",
                        "health_route": "/ready",
                        "timeout_secs": 25
                    }
                }),
            }],
            ..RegisteredRepository::default()
        };

        let client = build_graph_structural_flight_transport_client(
            &repository,
            GraphStructuralRouteKind::StructuralRerank,
        )
        .unwrap_or_else(|error| panic!("graph-structural config should parse: {error}"))
        .unwrap_or_else(|| panic!("graph-structural client should exist"));

        assert_eq!(client.flight_base_url(), "http://127.0.0.1:9101");
        assert_eq!(client.flight_route(), GRAPH_STRUCTURAL_RERANK_ROUTE);
        assert_eq!(
            client.selection().selected_transport,
            PluginTransportKind::ArrowFlight
        );
    }

    #[test]
    fn build_graph_structural_flight_transport_client_reads_route_specific_overrides() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Config {
                id: "julia".to_string(),
                options: serde_json::json!({
                    "graph_structural_transport": {
                        "base_url": "http://127.0.0.1:9101",
                        "structural_rerank": {
                            "route": "graph/structural/rerank",
                            "schema_version": "v0-custom",
                            "timeout_secs": 30
                        },
                        "constraint_filter": {
                            "route": "/graph/structural/filter",
                            "timeout_secs": 12
                        }
                    }
                }),
            }],
            ..RegisteredRepository::default()
        };

        let rerank_client = build_graph_structural_flight_transport_client(
            &repository,
            GraphStructuralRouteKind::StructuralRerank,
        )
        .unwrap_or_else(|error| panic!("rerank config should parse: {error}"))
        .unwrap_or_else(|| panic!("rerank client should exist"));
        let filter_client = build_graph_structural_flight_transport_client(
            &repository,
            GraphStructuralRouteKind::ConstraintFilter,
        )
        .unwrap_or_else(|error| panic!("filter config should parse: {error}"))
        .unwrap_or_else(|| panic!("filter client should exist"));

        assert_eq!(rerank_client.flight_route(), GRAPH_STRUCTURAL_RERANK_ROUTE);
        assert_eq!(filter_client.flight_route(), GRAPH_STRUCTURAL_FILTER_ROUTE);
    }

    #[test]
    fn build_graph_structural_flight_transport_client_honors_enabled_false() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Config {
                id: "julia".to_string(),
                options: serde_json::json!({
                    "graph_structural_transport": {
                        "base_url": "http://127.0.0.1:9101",
                        "constraint_filter": {
                            "enabled": false
                        }
                    }
                }),
            }],
            ..RegisteredRepository::default()
        };

        let client = build_graph_structural_flight_transport_client(
            &repository,
            GraphStructuralRouteKind::ConstraintFilter,
        )
        .unwrap_or_else(|error| panic!("disabled route-specific config should parse: {error}"));
        assert!(client.is_none());
    }

    #[test]
    fn build_graph_structural_flight_transport_client_rejects_invalid_field_types() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Config {
                id: "julia".to_string(),
                options: serde_json::json!({
                    "graph_structural_transport": {
                        "constraint_filter": {
                            "timeout_secs": "fast"
                        }
                    }
                }),
            }],
            ..RegisteredRepository::default()
        };

        let Err(error) = build_graph_structural_flight_transport_client(
            &repository,
            GraphStructuralRouteKind::ConstraintFilter,
        ) else {
            panic!("invalid timeout type must fail");
        };
        assert!(
            error
                .to_string()
                .contains("Julia plugin field `timeout_secs` must be an unsigned integer"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn build_graph_structural_flight_transport_client_falls_back_to_capability_manifest() {
        let port = reserve_real_service_port();
        let base_url = format!("http://127.0.0.1:{port}");
        let _service = spawn_real_wendaosearch_demo_capability_manifest_service(port);
        Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap_or_else(|error| panic!("build wait runtime: {error}"))
            .block_on(wait_for_service_ready_with_attempts(
                &format!("http://127.0.0.1:{port}"),
                600,
            ))
            .unwrap_or_else(|error| {
                panic!("wait for real WendaoSearch capability-manifest service: {error}")
            });

        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            path: None,
            url: None,
            git_ref: None,
            refresh: RepositoryRefreshPolicy::Fetch,
            plugins: vec![RepositoryPluginConfig::Config {
                id: "julia".to_string(),
                options: serde_json::json!({
                    "capability_manifest_transport": {
                        "base_url": base_url,
                        "route": "/plugin/capabilities",
                        "schema_version": "v0-draft"
                    }
                }),
            }],
        };

        let rerank_client = build_graph_structural_flight_transport_client(
            &repository,
            GraphStructuralRouteKind::StructuralRerank,
        )
        .unwrap_or_else(|error| panic!("manifest fallback should parse rerank route: {error}"))
        .unwrap_or_else(|| panic!("manifest fallback rerank client should exist"));
        let filter_client = build_graph_structural_flight_transport_client(
            &repository,
            GraphStructuralRouteKind::ConstraintFilter,
        )
        .unwrap_or_else(|error| panic!("manifest fallback should parse filter route: {error}"))
        .unwrap_or_else(|| panic!("manifest fallback filter client should exist"));

        assert_eq!(rerank_client.flight_base_url(), base_url);
        assert_eq!(rerank_client.flight_route(), GRAPH_STRUCTURAL_RERANK_ROUTE);
        assert_eq!(filter_client.flight_base_url(), base_url);
        assert_eq!(filter_client.flight_route(), GRAPH_STRUCTURAL_FILTER_ROUTE);
    }

    #[test]
    fn validate_graph_structural_request_batches_accepts_staged_shapes() {
        let rerank = structural_rerank_request_batch();
        let filter = constraint_filter_request_batch();

        assert!(
            validate_graph_structural_request_batches(
                GraphStructuralRouteKind::StructuralRerank,
                &[rerank]
            )
            .is_ok()
        );
        assert!(
            validate_graph_structural_request_batches(
                GraphStructuralRouteKind::ConstraintFilter,
                &[filter]
            )
            .is_ok()
        );
    }

    #[test]
    fn validate_graph_structural_response_batches_accepts_staged_shapes() {
        let rerank = structural_rerank_response_batch();
        let filter = constraint_filter_response_batch();

        assert!(
            validate_graph_structural_response_batches(
                GraphStructuralRouteKind::StructuralRerank,
                &[rerank]
            )
            .is_ok()
        );
        assert!(
            validate_graph_structural_response_batches(
                GraphStructuralRouteKind::ConstraintFilter,
                &[filter]
            )
            .is_ok()
        );
    }

    #[test]
    fn validate_graph_structural_response_batches_rejects_wrong_shape() {
        let error = validate_graph_structural_response_batches(
            GraphStructuralRouteKind::ConstraintFilter,
            &[structural_rerank_response_batch()],
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Julia graph-structural response contract"),
            "unexpected error: {error}"
        );
    }

    fn structural_rerank_request_batch() -> RecordBatch {
        let batch = RecordBatch::try_new(
            Arc::new(Schema::new(vec![
                utf8_field(GRAPH_STRUCTURAL_QUERY_ID_COLUMN),
                utf8_field(GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN),
                int32_field(GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN),
                int32_field(GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN),
                float64_field(GRAPH_STRUCTURAL_SEMANTIC_SCORE_COLUMN),
                float64_field(GRAPH_STRUCTURAL_DEPENDENCY_SCORE_COLUMN),
                float64_field(GRAPH_STRUCTURAL_KEYWORD_SCORE_COLUMN),
                float64_field(GRAPH_STRUCTURAL_TAG_SCORE_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_EDGE_CONSTRAINT_KINDS_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN),
            ])),
            vec![
                Arc::new(StringArray::from(vec!["query-1"])),
                Arc::new(StringArray::from(vec!["candidate-a"])),
                Arc::new(Int32Array::from(vec![0])),
                Arc::new(Int32Array::from(vec![2])),
                Arc::new(Float64Array::from(vec![0.7])),
                Arc::new(Float64Array::from(vec![0.6])),
                Arc::new(Float64Array::from(vec![0.5])),
                Arc::new(Float64Array::from(vec![0.4])),
                Arc::new(list_utf8_array(vec![vec!["semantic", "dependency"]])),
                Arc::new(list_utf8_array(vec![vec!["symbol:foo", "symbol:bar"]])),
                Arc::new(list_utf8_array(vec![vec!["depends_on"]])),
                Arc::new(list_utf8_array(vec![vec!["n1", "n2"]])),
                Arc::new(list_utf8_array(vec![vec!["depends_on"]])),
            ],
        )
        .unwrap_or_else(|error| panic!("structural rerank request batch: {error}"));
        attach_schema_metadata(&batch)
    }

    fn constraint_filter_request_batch() -> RecordBatch {
        let batch = RecordBatch::try_new(
            Arc::new(Schema::new(vec![
                utf8_field(GRAPH_STRUCTURAL_QUERY_ID_COLUMN),
                utf8_field(GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN),
                int32_field(GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN),
                int32_field(GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN),
                utf8_field(GRAPH_STRUCTURAL_CONSTRAINT_KIND_COLUMN),
                int32_field(GRAPH_STRUCTURAL_REQUIRED_BOUNDARY_SIZE_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_EDGE_CONSTRAINT_KINDS_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN),
            ])),
            vec![
                Arc::new(StringArray::from(vec!["query-1"])),
                Arc::new(StringArray::from(vec!["candidate-a"])),
                Arc::new(Int32Array::from(vec![1])),
                Arc::new(Int32Array::from(vec![3])),
                Arc::new(StringArray::from(vec!["boundary-match"])),
                Arc::new(Int32Array::from(vec![2])),
                Arc::new(list_utf8_array(vec![vec!["semantic", "tag"]])),
                Arc::new(list_utf8_array(vec![vec!["symbol:foo", "tag:core"]])),
                Arc::new(list_utf8_array(vec![vec!["depends_on"]])),
                Arc::new(list_utf8_array(vec![vec!["n1", "n2"]])),
                Arc::new(list_utf8_array(vec![vec!["depends_on"]])),
            ],
        )
        .unwrap_or_else(|error| panic!("constraint filter request batch: {error}"));
        attach_schema_metadata(&batch)
    }

    fn structural_rerank_response_batch() -> RecordBatch {
        RecordBatch::try_new(
            Arc::new(Schema::new(vec![
                utf8_field(GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN),
                bool_field(GRAPH_STRUCTURAL_FEASIBLE_COLUMN),
                float64_field(GRAPH_STRUCTURAL_STRUCTURAL_SCORE_COLUMN),
                float64_field(GRAPH_STRUCTURAL_FINAL_SCORE_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_PIN_ASSIGNMENT_COLUMN),
                utf8_field(GRAPH_STRUCTURAL_EXPLANATION_COLUMN),
            ])),
            vec![
                Arc::new(StringArray::from(vec!["candidate-a"])),
                Arc::new(BooleanArray::from(vec![true])),
                Arc::new(Float64Array::from(vec![0.91])),
                Arc::new(Float64Array::from(vec![0.87])),
                Arc::new(list_utf8_array(vec![vec!["pin:entry", "pin:exit"]])),
                Arc::new(StringArray::from(vec!["structural rerank accepted"])),
            ],
        )
        .unwrap_or_else(|error| panic!("structural rerank response batch: {error}"))
    }

    fn constraint_filter_response_batch() -> RecordBatch {
        RecordBatch::try_new(
            Arc::new(Schema::new(vec![
                utf8_field(GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN),
                bool_field(GRAPH_STRUCTURAL_ACCEPTED_COLUMN),
                float64_field(GRAPH_STRUCTURAL_STRUCTURAL_SCORE_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_PIN_ASSIGNMENT_COLUMN),
                utf8_field(GRAPH_STRUCTURAL_REJECTION_REASON_COLUMN),
            ])),
            vec![
                Arc::new(StringArray::from(vec!["candidate-a"])),
                Arc::new(BooleanArray::from(vec![true])),
                Arc::new(Float64Array::from(vec![0.73])),
                Arc::new(list_utf8_array(vec![vec!["pin:entry"]])),
                Arc::new(StringArray::from(vec![""])),
            ],
        )
        .unwrap_or_else(|error| panic!("constraint filter response batch: {error}"))
    }

    fn attach_schema_metadata(batch: &RecordBatch) -> RecordBatch {
        let metadata = std::collections::HashMap::from([(
            FLIGHT_SCHEMA_VERSION_METADATA_KEY.to_string(),
            JULIA_GRAPH_STRUCTURAL_SCHEMA_VERSION.to_string(),
        )]);
        let schema = Arc::new(batch.schema().as_ref().clone().with_metadata(metadata));
        RecordBatch::try_new(schema, batch.columns().to_vec())
            .unwrap_or_else(|error| panic!("schema metadata batch: {error}"))
    }

    fn utf8_field(name: &str) -> Field {
        Field::new(name, DataType::Utf8, false)
    }

    fn bool_field(name: &str) -> Field {
        Field::new(name, DataType::Boolean, false)
    }

    fn float64_field(name: &str) -> Field {
        Field::new(name, DataType::Float64, false)
    }

    fn int32_field(name: &str) -> Field {
        Field::new(name, DataType::Int32, false)
    }

    fn list_utf8_field(name: &str) -> Field {
        Field::new(
            name,
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            false,
        )
    }

    fn list_utf8_array(values: Vec<Vec<&str>>) -> ListArray {
        let mut builder = ListBuilder::new(StringBuilder::new());
        for row in values {
            for value in row {
                builder.values().append_value(value);
            }
            builder.append(true);
        }
        builder.finish()
    }
}
