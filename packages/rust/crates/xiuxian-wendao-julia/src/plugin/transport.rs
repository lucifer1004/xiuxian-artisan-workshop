use arrow::record_batch::RecordBatch;
use serde_json::Value;
use xiuxian_vector::attach_record_batch_metadata;
use xiuxian_wendao_core::{
    capabilities::{ContractVersion, PluginCapabilityBinding},
    repo_intelligence::{RegisteredRepository, RepoIntelligenceError, RepositoryPluginConfig},
    transport::{PluginTransportEndpoint, PluginTransportKind},
};
use xiuxian_wendao_runtime::transport::{
    DEFAULT_FLIGHT_BASE_URL, DEFAULT_FLIGHT_SCHEMA_VERSION, DEFAULT_FLIGHT_TIMEOUT_SECS,
    FLIGHT_SCHEMA_VERSION_METADATA_KEY, NegotiatedFlightTransportClient,
    negotiate_flight_transport_client_from_bindings, normalize_flight_route,
    validate_flight_schema_version, validate_flight_timeout_secs,
    validate_plugin_arrow_response_batches,
};

use crate::compatibility::link_graph::{
    DEFAULT_JULIA_RERANK_FLIGHT_ROUTE, julia_rerank_provider_selector,
};

const JULIA_PLUGIN_ID: &str = "julia";
const FLIGHT_TRANSPORT_KEY: &str = "flight_transport";
const DEFAULT_JULIA_HEALTH_ROUTE: &str = "/healthz";
/// Baseline `WendaoArrow` response contract version enforced by this crate.
pub const JULIA_ARROW_RESPONSE_SCHEMA_VERSION: &str = DEFAULT_FLIGHT_SCHEMA_VERSION;

/// Build a Julia Flight transport client from repository plugin config.
///
/// The function looks for a `RepositoryPluginConfig::Config` entry whose `id`
/// is `julia`, and then reads either a nested `flight_transport` object or
/// direct transport keys from that plugin's `options`.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the plugin config uses invalid types
/// or cannot be converted into a valid Arrow Flight transport binding.
pub fn build_julia_flight_transport_client(
    repository: &RegisteredRepository,
) -> Result<Option<NegotiatedFlightTransportClient>, RepoIntelligenceError> {
    let Some(binding) = build_flight_transport_binding(repository)? else {
        return Ok(None);
    };

    negotiate_flight_transport_client_from_bindings(&[binding]).map_err(|error| {
        RepoIntelligenceError::ConfigLoad {
            message: format!(
                "failed to build Julia Flight transport client for repo `{}`: {error}",
                repository.id
            ),
        }
    })
}

/// Send Arrow batches to a remote Julia Flight transport and validate the `v1`
/// response contract before returning the decoded response batches.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the Flight roundtrip fails or the
/// decoded response violates the Julia Arrow response contract.
pub async fn process_julia_flight_batches(
    client: &NegotiatedFlightTransportClient,
    batches: &[RecordBatch],
) -> Result<Vec<RecordBatch>, RepoIntelligenceError> {
    let request_batches = batches
        .iter()
        .map(attach_schema_version_metadata)
        .collect::<Result<Vec<_>, _>>()?;
    let response_batches = client
        .process_batches(&request_batches)
        .await
        .map_err(|error| RepoIntelligenceError::AnalysisFailed {
            message: format!("Julia Arrow Flight request failed: {error}"),
        })?;
    validate_plugin_arrow_response_batches(response_batches.as_slice())?;
    Ok(response_batches)
}

/// Resolve the repository's Julia Arrow Flight transport client, perform the
/// remote Flight roundtrip, and validate the `v1` response contract.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the repository does not declare a
/// usable Julia Arrow Flight transport client, the Flight roundtrip fails, or
/// the decoded response violates the Julia response contract.
pub async fn process_julia_flight_batches_for_repository(
    repository: &RegisteredRepository,
    batches: &[RecordBatch],
) -> Result<Vec<RecordBatch>, RepoIntelligenceError> {
    let client = build_julia_flight_transport_client(repository)?.ok_or_else(|| {
        RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "repo `{}` does not declare an enabled Julia Flight transport client",
                repository.id
            ),
        }
    })?;
    process_julia_flight_batches(&client, batches).await
}

fn build_flight_transport_binding(
    repository: &RegisteredRepository,
) -> Result<Option<PluginCapabilityBinding>, RepoIntelligenceError> {
    let Some(options) = resolve_transport_options(repository)? else {
        return Ok(None);
    };

    if let Some(false) = bool_option(options, "enabled", repository)? {
        return Ok(None);
    }

    let base_url = string_option(options, "base_url", repository)?
        .unwrap_or_else(|| DEFAULT_FLIGHT_BASE_URL.to_string());
    let route = match string_option(options, "route", repository)? {
        Some(route) => {
            normalize_flight_route(route).map_err(|error| RepoIntelligenceError::ConfigLoad {
                message: format!(
                    "repo `{}` Julia flight_transport route is invalid: {error}",
                    repository.id
                ),
            })?
        }
        None => DEFAULT_JULIA_RERANK_FLIGHT_ROUTE.to_string(),
    };
    let health_route = match string_option(options, "health_route", repository)? {
        Some(route) => {
            normalize_flight_route(route).map_err(|error| RepoIntelligenceError::ConfigLoad {
                message: format!(
                    "repo `{}` Julia flight_transport health_route is invalid: {error}",
                    repository.id
                ),
            })?
        }
        None => DEFAULT_JULIA_HEALTH_ROUTE.to_string(),
    };
    let schema_version = match string_option(options, "schema_version", repository)? {
        Some(schema_version) => {
            validate_flight_schema_version(&schema_version).map_err(|error| {
                RepoIntelligenceError::ConfigLoad {
                    message: format!(
                        "repo `{}` Julia flight_transport schema version is invalid: {error}",
                        repository.id
                    ),
                }
            })?
        }
        None => DEFAULT_FLIGHT_SCHEMA_VERSION.to_string(),
    };
    let timeout_secs = match u64_option(options, "timeout_secs", repository)? {
        Some(timeout_secs) => validate_flight_timeout_secs(timeout_secs).map_err(|error| {
            RepoIntelligenceError::ConfigLoad {
                message: format!(
                    "repo `{}` Julia flight_transport timeout is invalid: {error}",
                    repository.id
                ),
            }
        })?,
        None => DEFAULT_FLIGHT_TIMEOUT_SECS,
    };

    Ok(Some(PluginCapabilityBinding {
        selector: julia_rerank_provider_selector(),
        endpoint: PluginTransportEndpoint {
            base_url: Some(base_url),
            route: Some(route),
            health_route: Some(health_route),
            timeout_secs: Some(timeout_secs),
        },
        launch: None,
        transport: PluginTransportKind::ArrowFlight,
        contract_version: ContractVersion(schema_version),
    }))
}

fn attach_schema_version_metadata(
    batch: &RecordBatch,
) -> Result<RecordBatch, RepoIntelligenceError> {
    attach_record_batch_metadata(
        batch,
        [(
            FLIGHT_SCHEMA_VERSION_METADATA_KEY,
            JULIA_ARROW_RESPONSE_SCHEMA_VERSION,
        )],
    )
    .map_err(|error| RepoIntelligenceError::AnalysisFailed {
        message: format!("failed to attach Julia Arrow Flight schema metadata: {error}"),
    })
}

fn resolve_transport_options(
    repository: &RegisteredRepository,
) -> Result<Option<&Value>, RepoIntelligenceError> {
    for plugin in &repository.plugins {
        let RepositoryPluginConfig::Config { id, options } = plugin else {
            continue;
        };
        if id != JULIA_PLUGIN_ID {
            continue;
        }

        if let Some(transport) = options.get(FLIGHT_TRANSPORT_KEY) {
            return object_option(transport, FLIGHT_TRANSPORT_KEY, repository).map(Some);
        }
        if contains_transport_keys(options) {
            return object_option(options, "options", repository).map(Some);
        }
    }
    Ok(None)
}

fn contains_transport_keys(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    [
        "enabled",
        "base_url",
        "route",
        "health_route",
        "schema_version",
        "timeout_secs",
    ]
    .iter()
    .any(|key| object.contains_key(*key))
}

fn object_option<'a>(
    value: &'a Value,
    field: &str,
    repository: &RegisteredRepository,
) -> Result<&'a Value, RepoIntelligenceError> {
    if value.is_object() {
        return Ok(value);
    }

    Err(plugin_config_type_error(repository, field, "an object"))
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

#[cfg(test)]
mod tests {
    use arrow::array::{Float64Array, StringArray};
    use arrow::record_batch::RecordBatch;
    use xiuxian_wendao_core::{
        capabilities::{ContractVersion, PluginCapabilityBinding},
        repo_intelligence::{
            JULIA_ARROW_ANALYZER_SCORE_COLUMN, JULIA_ARROW_DOC_ID_COLUMN,
            JULIA_ARROW_FINAL_SCORE_COLUMN, JULIA_ARROW_TRACE_ID_COLUMN, RegisteredRepository,
            RepositoryPluginConfig,
        },
        transport::{PluginTransportEndpoint, PluginTransportKind},
    };
    use xiuxian_wendao_runtime::transport::{
        NegotiatedFlightTransportClient, negotiate_flight_transport_client_from_bindings,
    };

    use super::{
        DEFAULT_JULIA_HEALTH_ROUTE, JULIA_ARROW_RESPONSE_SCHEMA_VERSION,
        build_julia_flight_transport_client, process_julia_flight_batches,
        process_julia_flight_batches_for_repository,
    };
    use crate::compatibility::link_graph::julia_rerank_provider_selector;
    use crate::julia_plugin_test_support::contract::{request_batch, request_batch_with_trace_id};
    use crate::julia_plugin_test_support::official_examples::{
        reserve_real_service_port, spawn_real_wendaoanalyzer_linear_blend_service,
        spawn_real_wendaoarrow_bad_response_service, spawn_real_wendaoarrow_metadata_service,
        spawn_real_wendaoarrow_service, wait_for_service_ready,
        wait_for_service_ready_with_attempts,
    };

    fn string_column<'a>(batch: &'a RecordBatch, name: &str) -> &'a StringArray {
        let Some(column) = batch
            .column_by_name(name)
            .and_then(|array| array.as_any().downcast_ref::<StringArray>())
        else {
            panic!("missing StringArray column `{name}`");
        };
        column
    }

    fn float64_column<'a>(batch: &'a RecordBatch, name: &str) -> &'a Float64Array {
        let Some(column) = batch
            .column_by_name(name)
            .and_then(|array| array.as_any().downcast_ref::<Float64Array>())
        else {
            panic!("missing Float64Array column `{name}`");
        };
        column
    }

    #[test]
    fn build_julia_flight_transport_client_returns_none_without_inline_config() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
            ..RegisteredRepository::default()
        };

        let client = match build_julia_flight_transport_client(&repository) {
            Ok(client) => client,
            Err(error) => panic!("expected missing inline config to be ignored: {error}"),
        };
        assert!(client.is_none());
    }

    #[test]
    fn build_julia_flight_transport_client_reads_nested_flight_transport_options() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Config {
                id: "julia".to_string(),
                options: serde_json::json!({
                    "flight_transport": {
                        "base_url": "http://127.0.0.1:8081",
                        "route": "/analysis",
                        "health_route": "/ready",
                        "timeout_secs": 30
                    }
                }),
            }],
            ..RegisteredRepository::default()
        };

        let client = match build_julia_flight_transport_client(&repository) {
            Ok(Some(client)) => client,
            Ok(None) => panic!("expected inline Julia Arrow Flight transport config"),
            Err(error) => panic!("expected nested config to build successfully: {error}"),
        };

        assert_eq!(client.flight_base_url(), "http://127.0.0.1:8081");
        assert_eq!(client.flight_route(), "/analysis");
        assert_eq!(
            client.selection().selected_transport,
            PluginTransportKind::ArrowFlight
        );
    }

    #[test]
    fn build_julia_flight_transport_client_rejects_invalid_field_types() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Config {
                id: "julia".to_string(),
                options: serde_json::json!({
                    "flight_transport": {
                        "timeout_secs": "fast"
                    }
                }),
            }],
            ..RegisteredRepository::default()
        };

        let Err(error) = build_julia_flight_transport_client(&repository) else {
            panic!("expected invalid timeout type to fail");
        };
        assert!(
            error
                .to_string()
                .contains("Julia plugin field `timeout_secs` must be an unsigned integer"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn build_julia_flight_transport_client_honors_enabled_false() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Config {
                id: "julia".to_string(),
                options: serde_json::json!({
                    "flight_transport": {
                        "enabled": false,
                        "base_url": "http://127.0.0.1:8081"
                    }
                }),
            }],
            ..RegisteredRepository::default()
        };

        let client = match build_julia_flight_transport_client(&repository) {
            Ok(client) => client,
            Err(error) => panic!("expected disabled config to be ignored: {error}"),
        };
        assert!(client.is_none());
    }

    #[tokio::test]
    async fn process_julia_flight_batches_validates_remote_response() {
        let port = reserve_real_service_port();
        let base_url = format!("http://127.0.0.1:{port}");
        let mut service = spawn_real_wendaoarrow_service(port);
        let client = test_transport_client(base_url.clone(), "/rerank");

        wait_for_service_ready(&base_url)
            .await
            .unwrap_or_else(|error| panic!("wait for real WendaoArrow Flight service: {error}"));

        let result = process_julia_flight_batches(&client, &[request_batch()]).await;
        assert!(result.is_ok(), "expected valid Flight response: {result:?}");

        service.kill();
    }

    #[tokio::test]
    async fn process_julia_flight_batches_rejects_invalid_remote_response() {
        let port = reserve_real_service_port();
        let base_url = format!("http://127.0.0.1:{port}");
        let mut service = spawn_real_wendaoarrow_bad_response_service(port);
        let client = test_transport_client(base_url.clone(), "/rerank");

        wait_for_service_ready(&base_url)
            .await
            .unwrap_or_else(|error| {
                panic!("wait for real WendaoArrow bad-response Flight service: {error}")
            });

        let Err(error) = process_julia_flight_batches(&client, &[request_batch()]).await else {
            panic!("invalid remote response must fail");
        };
        assert!(
            error.to_string().contains("analyzer_score")
                || error.to_string().contains("Arrow Flight request failed"),
            "unexpected process error: {error}"
        );

        service.kill();
    }

    #[tokio::test]
    async fn process_julia_flight_batches_for_repository_builds_transport_from_repo_config() {
        let port = reserve_real_service_port();
        let base_url = format!("http://127.0.0.1:{port}");
        let mut service = spawn_real_wendaoarrow_service(port);
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Config {
                id: "julia".to_string(),
                options: serde_json::json!({
                    "flight_transport": {
                        "base_url": base_url.clone(),
                        "route": "/rerank",
                        "health_route": "/healthz"
                    }
                }),
            }],
            ..RegisteredRepository::default()
        };

        wait_for_service_ready(&base_url)
            .await
            .unwrap_or_else(|error| {
                panic!("wait for repo-configured WendaoArrow Flight service: {error}")
            });
        let result =
            process_julia_flight_batches_for_repository(&repository, &[request_batch()]).await;
        assert!(
            result.is_ok(),
            "expected repository-configured Flight transport: {result:?}"
        );

        service.kill();
    }

    #[tokio::test]
    async fn process_julia_flight_batches_for_repository_rejects_missing_transport() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
            ..RegisteredRepository::default()
        };

        let Err(error) =
            process_julia_flight_batches_for_repository(&repository, &[request_batch()]).await
        else {
            panic!("missing inline transport must fail");
        };
        assert!(
            error
                .to_string()
                .contains("does not declare an enabled Julia Flight transport client"),
            "unexpected repository transport error: {error}"
        );
    }

    #[tokio::test]
    async fn process_julia_flight_batches_against_real_wendaoarrow_service() {
        let port = reserve_real_service_port();
        let base_url = format!("http://127.0.0.1:{port}");
        let mut service = spawn_real_wendaoarrow_service(port);
        let client = test_transport_client(base_url.clone(), "/rerank");

        wait_for_service_ready(&base_url)
            .await
            .unwrap_or_else(|error| panic!("wait for real WendaoArrow Flight service: {error}"));

        let response_batches = process_julia_flight_batches(&client, &[request_batch()])
            .await
            .unwrap_or_else(|error| {
                panic!("real WendaoArrow Flight roundtrip should succeed: {error}")
            });

        assert_eq!(response_batches.len(), 1);
        let batch = &response_batches[0];
        let doc_id = string_column(batch, JULIA_ARROW_DOC_ID_COLUMN);
        let analyzer_score = float64_column(batch, JULIA_ARROW_ANALYZER_SCORE_COLUMN);
        let final_score = float64_column(batch, JULIA_ARROW_FINAL_SCORE_COLUMN);

        assert_eq!(doc_id.value(0), "doc-a");
        assert_eq!(doc_id.value(1), "doc-b");
        assert!((analyzer_score.value(0) - 0.4).abs() < f64::EPSILON);
        assert!((analyzer_score.value(1) - 0.7).abs() < f64::EPSILON);
        assert!((final_score.value(0) - 0.4).abs() < f64::EPSILON);
        assert!((final_score.value(1) - 0.7).abs() < f64::EPSILON);

        service.kill();
    }

    #[tokio::test]
    async fn real_wendaoarrow_metadata_example_roundtrip_decodes_trace_id_column() {
        let port = reserve_real_service_port();
        let base_url = format!("http://127.0.0.1:{port}");
        let mut service = spawn_real_wendaoarrow_metadata_service(port);
        let client = test_transport_client(base_url.clone(), "/rerank");

        wait_for_service_ready(&base_url)
            .await
            .unwrap_or_else(|error| {
                panic!("wait for real WendaoArrow metadata Flight service: {error}")
            });

        let batches =
            process_julia_flight_batches(&client, &[request_batch_with_trace_id("trace-123")])
                .await
                .unwrap_or_else(|error| panic!("metadata Flight request: {error}"));
        assert_eq!(batches.len(), 1);

        let batch = &batches[0];
        let trace_id = string_column(batch, JULIA_ARROW_TRACE_ID_COLUMN);
        assert_eq!(trace_id.value(0), "trace-123");
        assert_eq!(trace_id.value(1), "trace-123");

        service.kill();
    }

    #[tokio::test]
    async fn real_wendaoanalyzer_linear_blend_roundtrip_emits_expected_scores() {
        let port = reserve_real_service_port();
        let base_url = format!("http://127.0.0.1:{port}");
        let mut service = spawn_real_wendaoanalyzer_linear_blend_service(port);
        let client = test_transport_client(base_url.clone(), "/rerank");

        wait_for_service_ready_with_attempts(&base_url, 600)
            .await
            .unwrap_or_else(|error| panic!("wait for real WendaoAnalyzer Flight service: {error}"));

        let batches = process_julia_flight_batches(&client, &[request_batch()])
            .await
            .unwrap_or_else(|error| panic!("linear blend Flight request: {error}"));
        assert_eq!(batches.len(), 1);

        let batch = &batches[0];
        let doc_id = string_column(batch, JULIA_ARROW_DOC_ID_COLUMN);
        let analyzer_score = float64_column(batch, JULIA_ARROW_ANALYZER_SCORE_COLUMN);
        let final_score = float64_column(batch, JULIA_ARROW_FINAL_SCORE_COLUMN);
        let ranking_reason = string_column(batch, "ranking_reason");

        assert_eq!(doc_id.value(0), "doc-a");
        assert_eq!(doc_id.value(1), "doc-b");
        assert!((analyzer_score.value(0) - 0.928_476_63).abs() < 1e-8);
        assert!((analyzer_score.value(1) - 0.979_936_66).abs() < 1e-8);
        assert!((final_score.value(0) - 0.743_509_810_566_902_2).abs() < 1e-12);
        assert!((final_score.value(1) - 0.881_958_828_568_458_5).abs() < 1e-12);
        assert_eq!(
            ranking_reason.value(0),
            "final_score=0.35*vector_score+0.65*cosine_similarity"
        );
        assert_eq!(
            ranking_reason.value(1),
            "final_score=0.35*vector_score+0.65*cosine_similarity"
        );

        service.kill();
    }

    fn test_transport_client(base_url: String, route: &str) -> NegotiatedFlightTransportClient {
        negotiate_flight_transport_client_from_bindings(&[PluginCapabilityBinding {
            selector: julia_rerank_provider_selector(),
            endpoint: PluginTransportEndpoint {
                base_url: Some(base_url),
                route: Some(route.to_string()),
                health_route: Some(DEFAULT_JULIA_HEALTH_ROUTE.to_string()),
                timeout_secs: Some(15),
            },
            launch: None,
            transport: PluginTransportKind::ArrowFlight,
            contract_version: ContractVersion(JULIA_ARROW_RESPONSE_SCHEMA_VERSION.to_string()),
        }])
        .unwrap_or_else(|error| panic!("build negotiated Flight transport client: {error}"))
        .unwrap_or_else(|| panic!("negotiated Flight transport client should exist"))
    }
}
