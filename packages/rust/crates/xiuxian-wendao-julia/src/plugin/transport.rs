use std::collections::BTreeSet;

use arrow::array::{Array, Float64Array, StringArray};
use arrow::record_batch::RecordBatch;
use serde_json::Value;
use xiuxian_vector::{
    ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION, ArrowTransportClient, ArrowTransportConfig,
};
use xiuxian_wendao::analyzers::config::{RegisteredRepository, RepositoryPluginConfig};
use xiuxian_wendao::analyzers::errors::RepoIntelligenceError;
use xiuxian_wendao::analyzers::{
    JULIA_ARROW_ANALYZER_SCORE_COLUMN, JULIA_ARROW_DOC_ID_COLUMN, JULIA_ARROW_FINAL_SCORE_COLUMN,
};

const JULIA_PLUGIN_ID: &str = "julia";
const ARROW_TRANSPORT_KEY: &str = "arrow_transport";
/// Baseline WendaoArrow response contract version enforced by this crate.
pub const JULIA_ARROW_RESPONSE_SCHEMA_VERSION: &str = ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION;

/// Build a Julia Arrow transport client from repository plugin config.
///
/// The function looks for a `RepositoryPluginConfig::Config` entry whose `id`
/// is `julia`, and then reads either a nested `arrow_transport` object or
/// direct transport keys from that plugin's `options`.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the plugin config uses invalid types
/// or cannot be converted into a valid [`ArrowTransportConfig`].
pub fn build_julia_arrow_transport_client(
    repository: &RegisteredRepository,
) -> Result<Option<ArrowTransportClient>, RepoIntelligenceError> {
    let Some(config) = build_arrow_transport_config(repository)? else {
        return Ok(None);
    };

    ArrowTransportClient::new(config)
        .map(Some)
        .map_err(|error| RepoIntelligenceError::ConfigLoad {
            message: format!(
                "failed to build Julia Arrow transport client for repo `{}`: {error}",
                repository.id
            ),
        })
}

/// Send Arrow batches to a remote Julia transport and validate the `v1`
/// response contract before returning the decoded response batches.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the HTTP roundtrip fails or the
/// decoded response violates the Julia Arrow response contract.
pub async fn process_julia_arrow_batches(
    client: &ArrowTransportClient,
    batches: &[RecordBatch],
) -> Result<Vec<RecordBatch>, RepoIntelligenceError> {
    let response_batches = client.process_batches(batches).await.map_err(|error| {
        RepoIntelligenceError::AnalysisFailed {
            message: format!("Julia Arrow transport request failed: {error}"),
        }
    })?;
    validate_julia_arrow_response_batches(response_batches.as_slice())?;
    Ok(response_batches)
}

/// Resolve the repository's Julia Arrow transport client, perform the remote
/// Arrow IPC roundtrip, and validate the `v1` response contract.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the repository does not declare a
/// usable Julia Arrow transport client, the HTTP roundtrip fails, or the
/// decoded response violates the Julia response contract.
pub async fn process_julia_arrow_batches_for_repository(
    repository: &RegisteredRepository,
    batches: &[RecordBatch],
) -> Result<Vec<RecordBatch>, RepoIntelligenceError> {
    let client = build_julia_arrow_transport_client(repository)?.ok_or_else(|| {
        RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "repo `{}` does not declare an enabled Julia Arrow transport client",
                repository.id
            ),
        }
    })?;
    process_julia_arrow_batches(&client, batches).await
}

fn build_arrow_transport_config(
    repository: &RegisteredRepository,
) -> Result<Option<ArrowTransportConfig>, RepoIntelligenceError> {
    let Some(options) = resolve_transport_options(repository)? else {
        return Ok(None);
    };

    if let Some(false) = bool_option(options, "enabled", repository)? {
        return Ok(None);
    }

    let mut config = if let Some(base_url) = string_option(options, "base_url", repository)? {
        ArrowTransportConfig::new(base_url)
    } else {
        ArrowTransportConfig::default()
    };
    if let Some(route) = string_option(options, "route", repository)? {
        config = config.with_route(route);
    }
    if let Some(health_route) = string_option(options, "health_route", repository)? {
        config = config.with_health_route(health_route);
    }
    if let Some(content_type) = string_option(options, "content_type", repository)? {
        config = config.with_content_type(content_type);
    }
    if let Some(schema_version) = string_option(options, "schema_version", repository)? {
        config = config
            .with_schema_version(schema_version)
            .map_err(|error| RepoIntelligenceError::ConfigLoad {
                message: format!(
                    "repo `{}` Julia arrow_transport schema version is invalid: {error}",
                    repository.id
                ),
            })?;
    }
    if let Some(timeout_secs) = u64_option(options, "timeout_secs", repository)? {
        config = config.with_timeout_secs(timeout_secs).map_err(|error| {
            RepoIntelligenceError::ConfigLoad {
                message: format!(
                    "repo `{}` Julia arrow_transport timeout is invalid: {error}",
                    repository.id
                ),
            }
        })?;
    }

    Ok(Some(config))
}

fn resolve_transport_options<'a>(
    repository: &'a RegisteredRepository,
) -> Result<Option<&'a Value>, RepoIntelligenceError> {
    for plugin in &repository.plugins {
        let RepositoryPluginConfig::Config { id, options } = plugin else {
            continue;
        };
        if id != JULIA_PLUGIN_ID {
            continue;
        }

        if let Some(transport) = options.get(ARROW_TRANSPORT_KEY) {
            return object_option(transport, ARROW_TRANSPORT_KEY, repository)
                .map(Some)
                .or_else(|error| Err(error));
        }
        if contains_transport_keys(options) {
            return object_option(options, "options", repository)
                .map(Some)
                .or_else(|error| Err(error));
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
        "content_type",
        "schema_version",
        "timeout_secs",
    ]
    .iter()
    .any(|key| object.contains_key(*key))
}

/// Validate a WendaoArrow `v1` Julia analyzer response batch set.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the response batches are missing
/// required columns, contain duplicate `doc_id` values, or emit invalid
/// `final_score` values.
pub fn validate_julia_arrow_response_batches(
    batches: &[RecordBatch],
) -> Result<(), RepoIntelligenceError> {
    let mut seen_doc_ids = BTreeSet::new();

    for batch in batches {
        let doc_id = batch
            .column_by_name(JULIA_ARROW_DOC_ID_COLUMN)
            .and_then(|array| array.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| contract_error("missing required Utf8 column `doc_id`"))?;
        let analyzer_score = batch
            .column_by_name(JULIA_ARROW_ANALYZER_SCORE_COLUMN)
            .and_then(|array| array.as_any().downcast_ref::<Float64Array>())
            .ok_or_else(|| contract_error("missing required Float64 column `analyzer_score`"))?;
        let final_score = batch
            .column_by_name(JULIA_ARROW_FINAL_SCORE_COLUMN)
            .and_then(|array| array.as_any().downcast_ref::<Float64Array>())
            .ok_or_else(|| contract_error("missing required Float64 column `final_score`"))?;

        for row in 0..batch.num_rows() {
            if doc_id.is_null(row) {
                return Err(contract_error("`doc_id` must be non-null"));
            }
            if analyzer_score.is_null(row) {
                return Err(contract_error("`analyzer_score` must be non-null"));
            }
            if final_score.is_null(row) {
                return Err(contract_error("`final_score` must be non-null"));
            }

            let doc_id_value = doc_id.value(row).to_string();
            if !seen_doc_ids.insert(doc_id_value.clone()) {
                return Err(contract_error(format!(
                    "duplicate `doc_id` in Julia analyzer response: {doc_id_value}"
                )));
            }

            if !final_score.value(row).is_finite() {
                return Err(contract_error(format!(
                    "`final_score` must be finite for doc_id `{doc_id_value}`"
                )));
            }
        }
    }

    Ok(())
}

fn contract_error(message: impl Into<String>) -> RepoIntelligenceError {
    RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "Julia Arrow response contract `{JULIA_ARROW_RESPONSE_SCHEMA_VERSION}` violated: {}",
            message.into()
        ),
    }
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
    use axum::body::Bytes;
    use axum::http::HeaderValue;
    use axum::http::header::CONTENT_TYPE;
    use axum::response::{IntoResponse, Response};
    use axum::routing::{get, post};
    use axum::{Router, serve};
    use tokio::net::TcpListener;
    use xiuxian_vector::{ArrowTransportClient, ArrowTransportConfig, encode_record_batches_ipc};
    use xiuxian_wendao::analyzers::{
        JULIA_ARROW_ANALYZER_SCORE_COLUMN, JULIA_ARROW_DOC_ID_COLUMN,
        JULIA_ARROW_FINAL_SCORE_COLUMN, JULIA_ARROW_TRACE_ID_COLUMN,
    };

    use super::{
        JULIA_ARROW_RESPONSE_SCHEMA_VERSION, build_julia_arrow_transport_client,
        process_julia_arrow_batches, process_julia_arrow_batches_for_repository,
        validate_julia_arrow_response_batches,
    };
    use crate::julia_plugin_test_support::contract::{
        invalid_response_missing_analyzer_score_batch, invalid_response_missing_final_batch,
        request_batch, request_batch_with_trace_id, response_batch, response_batch_with_duplicates,
        response_batch_without_trace_id,
    };
    use crate::julia_plugin_test_support::official_examples::{
        reserve_real_service_port, spawn_real_wendaoanalyzer_linear_blend_service,
        spawn_real_wendaoarrow_metadata_service, spawn_real_wendaoarrow_service, wait_for_health,
        wait_for_health_with_attempts,
    };
    use xiuxian_wendao::analyzers::config::{RegisteredRepository, RepositoryPluginConfig};

    #[test]
    fn build_julia_arrow_transport_client_returns_none_without_inline_config() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
            ..RegisteredRepository::default()
        };

        let client = match build_julia_arrow_transport_client(&repository) {
            Ok(client) => client,
            Err(error) => panic!("expected missing inline config to be ignored: {error}"),
        };
        assert!(client.is_none());
    }

    #[test]
    fn build_julia_arrow_transport_client_reads_nested_arrow_transport_options() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Config {
                id: "julia".to_string(),
                options: serde_json::json!({
                    "arrow_transport": {
                        "base_url": "http://127.0.0.1:8081",
                        "route": "/analysis",
                        "health_route": "/ready",
                        "timeout_secs": 30
                    }
                }),
            }],
            ..RegisteredRepository::default()
        };

        let client = match build_julia_arrow_transport_client(&repository) {
            Ok(Some(client)) => client,
            Ok(None) => panic!("expected inline Julia arrow transport config"),
            Err(error) => panic!("expected nested config to build successfully: {error}"),
        };

        assert_eq!(client.config().base_url(), "http://127.0.0.1:8081");
        assert_eq!(client.config().route(), "/analysis");
        assert_eq!(client.config().health_route(), "/ready");
        assert_eq!(
            client.config().schema_version(),
            JULIA_ARROW_RESPONSE_SCHEMA_VERSION
        );
        assert_eq!(client.config().timeout().as_secs(), 30);
    }

    #[test]
    fn build_julia_arrow_transport_client_rejects_invalid_field_types() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Config {
                id: "julia".to_string(),
                options: serde_json::json!({
                    "arrow_transport": {
                        "timeout_secs": "fast"
                    }
                }),
            }],
            ..RegisteredRepository::default()
        };

        let error = match build_julia_arrow_transport_client(&repository) {
            Ok(_) => panic!("expected invalid timeout type to fail"),
            Err(error) => error,
        };
        assert!(
            error
                .to_string()
                .contains("Julia plugin field `timeout_secs` must be an unsigned integer"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn build_julia_arrow_transport_client_honors_enabled_false() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Config {
                id: "julia".to_string(),
                options: serde_json::json!({
                    "arrow_transport": {
                        "enabled": false,
                        "base_url": "http://127.0.0.1:8081"
                    }
                }),
            }],
            ..RegisteredRepository::default()
        };

        let client = match build_julia_arrow_transport_client(&repository) {
            Ok(client) => client,
            Err(error) => panic!("expected disabled config to be ignored: {error}"),
        };
        assert!(client.is_none());
    }

    #[test]
    fn validate_julia_arrow_response_batches_accepts_v1_shape() {
        let batch = response_batch_without_trace_id();

        let result = validate_julia_arrow_response_batches(&[batch]);
        assert!(result.is_ok(), "expected valid Julia response: {result:?}");
    }

    #[test]
    fn validate_julia_arrow_response_batches_rejects_duplicates_and_missing_columns() {
        let duplicate_batch = response_batch_with_duplicates();

        let duplicate_error = validate_julia_arrow_response_batches(&[duplicate_batch])
            .expect_err("duplicate doc_id must fail");
        assert!(
            duplicate_error
                .to_string()
                .contains("duplicate `doc_id` in Julia analyzer response"),
            "unexpected duplicate error: {duplicate_error}"
        );

        let missing_column_batch = invalid_response_missing_analyzer_score_batch();

        let missing_error = validate_julia_arrow_response_batches(&[missing_column_batch])
            .expect_err("missing analyzer_score must fail");
        assert!(
            missing_error
                .to_string()
                .contains("missing required Float64 column `analyzer_score`"),
            "unexpected missing-column error: {missing_error}"
        );
    }

    #[tokio::test]
    async fn process_julia_arrow_batches_validates_remote_response() {
        let app = Router::new()
            .route("/health", get(|| async { r#"{"status":"ok"}"# }))
            .route("/arrow-ipc", post(|| async { valid_response().await }));
        let client = xiuxian_vector::ArrowTransportClient::new(ArrowTransportConfig::new(
            spawn_test_server(app).await,
        ))
        .expect("build transport client");

        let result = process_julia_arrow_batches(&client, &[request_batch()]).await;
        assert!(
            result.is_ok(),
            "expected valid transport response: {result:?}"
        );
    }

    #[tokio::test]
    async fn process_julia_arrow_batches_rejects_invalid_remote_response() {
        let app = Router::new()
            .route("/health", get(|| async { r#"{"status":"ok"}"# }))
            .route(
                "/arrow-ipc",
                post(|| async { invalid_response_missing_final().await }),
            );
        let client = xiuxian_vector::ArrowTransportClient::new(ArrowTransportConfig::new(
            spawn_test_server(app).await,
        ))
        .expect("build transport client");

        let error = process_julia_arrow_batches(&client, &[request_batch()])
            .await
            .expect_err("missing final_score must fail");
        assert!(
            error
                .to_string()
                .contains("missing required Float64 column `final_score`"),
            "unexpected process error: {error}"
        );
    }

    #[tokio::test]
    async fn process_julia_arrow_batches_for_repository_builds_transport_from_repo_config() {
        let app = Router::new()
            .route("/health", get(|| async { r#"{"status":"ok"}"# }))
            .route("/arrow-ipc", post(|| async { valid_response().await }));
        let base_url = spawn_test_server(app).await;
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Config {
                id: "julia".to_string(),
                options: serde_json::json!({
                    "arrow_transport": {
                        "base_url": base_url,
                        "route": "/arrow-ipc",
                        "health_route": "/health"
                    }
                }),
            }],
            ..RegisteredRepository::default()
        };

        let result =
            process_julia_arrow_batches_for_repository(&repository, &[request_batch()]).await;
        assert!(
            result.is_ok(),
            "expected repository-configured transport: {result:?}"
        );
    }

    #[tokio::test]
    async fn process_julia_arrow_batches_for_repository_rejects_missing_transport() {
        let repository = RegisteredRepository {
            id: "repo-julia".to_string(),
            plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
            ..RegisteredRepository::default()
        };

        let error = process_julia_arrow_batches_for_repository(&repository, &[request_batch()])
            .await
            .expect_err("missing inline transport must fail");
        assert!(
            error
                .to_string()
                .contains("does not declare an enabled Julia Arrow transport client"),
            "unexpected repository transport error: {error}"
        );
    }

    #[tokio::test]
    async fn process_julia_arrow_batches_against_real_wendaoarrow_service() {
        let port = reserve_real_service_port();
        let base_url = format!("http://127.0.0.1:{port}");
        let mut service = spawn_real_wendaoarrow_service(port);
        let client = xiuxian_vector::ArrowTransportClient::new(ArrowTransportConfig::new(base_url))
            .expect("build transport client");

        wait_for_health(&client)
            .await
            .unwrap_or_else(|error| panic!("wait for real WendaoArrow health: {error}"));

        let response_batches = process_julia_arrow_batches(&client, &[request_batch()])
            .await
            .unwrap_or_else(|error| panic!("real WendaoArrow roundtrip should succeed: {error}"));

        assert_eq!(response_batches.len(), 1);
        let batch = &response_batches[0];
        let doc_id = batch
            .column_by_name(JULIA_ARROW_DOC_ID_COLUMN)
            .and_then(|array| array.as_any().downcast_ref::<StringArray>())
            .expect("doc_id column");
        let analyzer_score = batch
            .column_by_name(JULIA_ARROW_ANALYZER_SCORE_COLUMN)
            .and_then(|array| array.as_any().downcast_ref::<Float64Array>())
            .expect("analyzer_score column");
        let final_score = batch
            .column_by_name(JULIA_ARROW_FINAL_SCORE_COLUMN)
            .and_then(|array| array.as_any().downcast_ref::<Float64Array>())
            .expect("final_score column");

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
        let client = ArrowTransportClient::new(ArrowTransportConfig::new(base_url))
            .expect("build metadata transport client");

        wait_for_health(&client)
            .await
            .unwrap_or_else(|error| panic!("wait for real WendaoArrow metadata health: {error}"));

        let batches = client
            .process_batches(&[request_batch_with_trace_id("trace-123")])
            .await
            .expect("metadata transport request");
        assert_eq!(batches.len(), 1);

        let batch = &batches[0];
        let trace_id = batch
            .column_by_name(JULIA_ARROW_TRACE_ID_COLUMN)
            .and_then(|array| array.as_any().downcast_ref::<StringArray>())
            .expect("trace_id column");
        assert_eq!(trace_id.value(0), "trace-123");
        assert_eq!(trace_id.value(1), "trace-123");

        service.kill();
    }

    #[tokio::test]
    async fn real_wendaoanalyzer_linear_blend_roundtrip_emits_expected_scores() {
        let port = reserve_real_service_port();
        let base_url = format!("http://127.0.0.1:{port}");
        let mut service = spawn_real_wendaoanalyzer_linear_blend_service(port);
        let client = ArrowTransportClient::new(ArrowTransportConfig::new(base_url))
            .expect("build analyzer transport client");

        wait_for_health_with_attempts(&client, 150)
            .await
            .unwrap_or_else(|error| panic!("wait for real WendaoAnalyzer health: {error}"));

        let batches = client
            .process_batches(&[request_batch()])
            .await
            .expect("linear blend transport request");
        assert_eq!(batches.len(), 1);

        let batch = &batches[0];
        let doc_id = batch
            .column_by_name(JULIA_ARROW_DOC_ID_COLUMN)
            .and_then(|array| array.as_any().downcast_ref::<StringArray>())
            .expect("doc_id column");
        let analyzer_score = batch
            .column_by_name(JULIA_ARROW_ANALYZER_SCORE_COLUMN)
            .and_then(|array| array.as_any().downcast_ref::<Float64Array>())
            .expect("analyzer_score column");
        let final_score = batch
            .column_by_name(JULIA_ARROW_FINAL_SCORE_COLUMN)
            .and_then(|array| array.as_any().downcast_ref::<Float64Array>())
            .expect("final_score column");
        let ranking_reason = batch
            .column_by_name("ranking_reason")
            .and_then(|array| array.as_any().downcast_ref::<StringArray>())
            .expect("ranking_reason column");

        assert_eq!(doc_id.value(0), "doc-a");
        assert_eq!(doc_id.value(1), "doc-b");
        assert!((analyzer_score.value(0) - 0.92847663).abs() < 1e-8);
        assert!((analyzer_score.value(1) - 0.97993666).abs() < 1e-8);
        assert!((final_score.value(0) - 0.7435098105669022).abs() < 1e-12);
        assert!((final_score.value(1) - 0.8819588285684585).abs() < 1e-12);
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

    async fn valid_response() -> Response {
        let batch = response_batch();
        arrow_response(&[batch])
    }

    async fn invalid_response_missing_final() -> Response {
        let batch = invalid_response_missing_final_batch();
        arrow_response(&[batch])
    }

    fn arrow_response(batches: &[RecordBatch]) -> Response {
        let payload = encode_record_batches_ipc(batches).expect("encode response");
        let mut response = Bytes::from(payload).into_response();
        response.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/vnd.apache.arrow.stream"),
        );
        response.headers_mut().insert(
            "x-wendao-schema-version",
            HeaderValue::from_static(JULIA_ARROW_RESPONSE_SCHEMA_VERSION),
        );
        response
    }

    async fn spawn_test_server(app: Router) -> String {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let address = listener.local_addr().expect("resolve test server addr");
        tokio::spawn(async move {
            if let Err(error) = serve(listener, app).await {
                panic!("Julia transport test server crashed: {error}");
            }
        });
        format!("http://{address}")
    }
}
