use std::sync::{Arc, Mutex};

use arrow::array::{Array, Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use axum::body::Bytes;
use axum::http::header::CONTENT_TYPE;
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Router, serve};
use std::sync::Arc as StdArc;
use tokio::net::TcpListener;

use super::client::ArrowTransportClient;
use super::config::{
    ARROW_TRANSPORT_CONTENT_TYPE, ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION, ArrowTransportConfig,
};
use super::error::ArrowTransportError;
use super::{decode_record_batches_ipc, encode_record_batches_ipc};

#[test]
fn transport_config_loads_from_toml_section() {
    let config = match ArrowTransportConfig::from_toml_str(
        r#"
            [gateway.arrow_transport]
            base_url = "http://127.0.0.1:9090"
            route = "process"
            health_route = "healthz"
            timeout_secs = 30
        "#,
    ) {
        Ok(Some(config)) => config,
        Ok(None) => panic!("expected [gateway.arrow_transport] section to exist"),
        Err(error) => panic!("failed to parse Arrow transport config: {error}"),
    };

    assert_eq!(config.base_url(), "http://127.0.0.1:9090");
    assert_eq!(config.route(), "/process");
    assert_eq!(config.health_route(), "/healthz");
    assert_eq!(config.timeout().as_secs(), 30);

    let endpoint = match config.endpoint_url() {
        Ok(url) => url,
        Err(error) => panic!("failed to resolve Arrow endpoint URL: {error}"),
    };
    let health = match config.health_url() {
        Ok(url) => url,
        Err(error) => panic!("failed to resolve Arrow health URL: {error}"),
    };
    assert_eq!(endpoint.as_str(), "http://127.0.0.1:9090/process");
    assert_eq!(health.as_str(), "http://127.0.0.1:9090/healthz");
}

#[tokio::test]
async fn transport_client_checks_health_endpoint() {
    let app = Router::new().route(
        "/health",
        get(|| async {
            (
                [(
                    "x-wendao-schema-version",
                    ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION,
                )],
                r#"{"status":"ok"}"#,
            )
        }),
    );
    let base_url = spawn_test_server(app).await;
    let client = build_test_client(ArrowTransportConfig::new(base_url));

    let result = client.check_health().await;
    assert!(result.is_ok(), "health probe should succeed: {result:?}");
}

#[tokio::test]
async fn transport_client_rejects_missing_health_schema_version() {
    let app = Router::new().route("/health", get(|| async { r#"{"status":"ok"}"# }));
    let base_url = spawn_test_server(app).await;
    let client = build_test_client(ArrowTransportConfig::new(base_url));

    let error = match client.check_health().await {
        Ok(_) => panic!("expected missing health schema header to fail"),
        Err(error) => error,
    };
    match error {
        ArrowTransportError::UnexpectedSchemaVersion { expected, found } => {
            assert_eq!(expected, ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION);
            assert_eq!(found, "<missing>");
        }
        other => panic!("unexpected error variant: {other}"),
    }
}

#[tokio::test]
async fn transport_client_roundtrips_arrow_batches() {
    let observed_content_type = Arc::new(Mutex::new(None::<String>));
    let observed_schema_version = Arc::new(Mutex::new(None::<String>));
    let app = Router::new()
        .route("/health", get(|| async { r#"{"status":"ok"}"# }))
        .route(
            "/arrow-ipc",
            post({
                let observed_content_type = observed_content_type.clone();
                let observed_schema_version = observed_schema_version.clone();
                move |headers: HeaderMap, body: Bytes| {
                    process_arrow_request(
                        observed_content_type.clone(),
                        observed_schema_version.clone(),
                        headers,
                        body,
                    )
                }
            }),
        );
    let base_url = spawn_test_server(app).await;
    let client = build_test_client(ArrowTransportConfig::new(base_url));
    let request_batch = sample_batch();

    let response_batches = match client.process_batch(&request_batch).await {
        Ok(batches) => batches,
        Err(error) => panic!("Arrow transport roundtrip failed: {error}"),
    };
    let content_type = match observed_content_type.lock() {
        Ok(guard) => guard.clone(),
        Err(error) => panic!("failed to inspect observed content-type: {error}"),
    };
    let schema_version = match observed_schema_version.lock() {
        Ok(guard) => guard.clone(),
        Err(error) => panic!("failed to inspect observed schema version: {error}"),
    };

    assert_eq!(content_type.as_deref(), Some(ARROW_TRANSPORT_CONTENT_TYPE));
    assert_eq!(
        schema_version.as_deref(),
        Some(ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION)
    );
    assert_eq!(response_batches.len(), 1);
    assert_string_column_eq(&request_batch, &response_batches[0], "doc_id");
    assert_float_column_eq(&request_batch, &response_batches[0], "score");
}

#[tokio::test]
async fn transport_client_rejects_mismatched_schema_version() {
    let app = Router::new()
        .route("/health", get(|| async { r#"{"status":"ok"}"# }))
        .route(
            "/arrow-ipc",
            post(|| async {
                let payload = encode_record_batches_ipc(&[sample_batch()]).expect("encode sample");
                let mut response = payload.into_response();
                response.headers_mut().insert(
                    CONTENT_TYPE,
                    HeaderValue::from_static(ARROW_TRANSPORT_CONTENT_TYPE),
                );
                response
                    .headers_mut()
                    .insert("x-wendao-schema-version", HeaderValue::from_static("v2"));
                response
            }),
        );
    let base_url = spawn_test_server(app).await;
    let client = build_test_client(ArrowTransportConfig::new(base_url));

    let error = match client.process_batch(&sample_batch()).await {
        Ok(_) => panic!("expected schema version mismatch to fail"),
        Err(error) => error,
    };
    match error {
        ArrowTransportError::UnexpectedSchemaVersion { expected, found } => {
            assert_eq!(expected, ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION);
            assert_eq!(found, "v2");
        }
        other => panic!("unexpected error variant: {other}"),
    }
}

#[tokio::test]
async fn transport_client_rejects_missing_response_schema_version() {
    let app = Router::new()
        .route(
            "/health",
            get(|| async {
                (
                    [(
                        "x-wendao-schema-version",
                        ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION,
                    )],
                    r#"{"status":"ok"}"#,
                )
            }),
        )
        .route(
            "/arrow-ipc",
            post(|| async {
                let payload = encode_record_batches_ipc(&[sample_batch()]).expect("encode sample");
                let mut response = payload.into_response();
                response.headers_mut().insert(
                    CONTENT_TYPE,
                    HeaderValue::from_static(ARROW_TRANSPORT_CONTENT_TYPE),
                );
                response
            }),
        );
    let base_url = spawn_test_server(app).await;
    let client = build_test_client(ArrowTransportConfig::new(base_url));

    let error = match client.process_batch(&sample_batch()).await {
        Ok(_) => panic!("expected missing schema version header to fail"),
        Err(error) => error,
    };
    match error {
        ArrowTransportError::UnexpectedSchemaVersion { expected, found } => {
            assert_eq!(expected, ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION);
            assert_eq!(found, "<missing>");
        }
        other => panic!("unexpected error variant: {other}"),
    }
}

#[tokio::test]
async fn transport_client_rejects_error_status() {
    let app = Router::new()
        .route("/health", get(|| async { r#"{"status":"ok"}"# }))
        .route(
            "/arrow-ipc",
            post(|| async { (StatusCode::INTERNAL_SERVER_ERROR, "processor_failed") }),
        );
    let base_url = spawn_test_server(app).await;
    let client = build_test_client(ArrowTransportConfig::new(base_url));

    let error = match client.process_batch(&sample_batch()).await {
        Ok(_) => panic!("expected error status to fail"),
        Err(error) => error,
    };
    match error {
        ArrowTransportError::UnexpectedStatus { status, body } => {
            assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body, "processor_failed");
        }
        other => panic!("unexpected error variant: {other}"),
    }
}

#[tokio::test]
async fn transport_client_rejects_unexpected_content_type() {
    let app = Router::new()
        .route("/health", get(|| async { r#"{"status":"ok"}"# }))
        .route(
            "/arrow-ipc",
            post(|| async {
                (
                    [(CONTENT_TYPE, "application/json")],
                    r#"{"error":"not_arrow"}"#,
                )
            }),
        );
    let base_url = spawn_test_server(app).await;
    let client = build_test_client(ArrowTransportConfig::new(base_url));

    let error = match client.process_batch(&sample_batch()).await {
        Ok(_) => panic!("expected content-type mismatch to fail"),
        Err(error) => error,
    };
    match error {
        ArrowTransportError::UnexpectedContentType { expected, found } => {
            assert_eq!(expected, ARROW_TRANSPORT_CONTENT_TYPE);
            assert_eq!(found, "application/json");
        }
        other => panic!("unexpected error variant: {other}"),
    }
}

fn build_test_client(config: ArrowTransportConfig) -> ArrowTransportClient {
    match ArrowTransportClient::new(config) {
        Ok(client) => client,
        Err(error) => panic!("failed to build Arrow transport client: {error}"),
    }
}

async fn process_arrow_request(
    observed_content_type: Arc<Mutex<Option<String>>>,
    observed_schema_version: Arc<Mutex<Option<String>>>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    match observed_content_type.lock() {
        Ok(mut guard) => {
            *guard = content_type;
        }
        Err(error) => panic!("failed to lock observed content-type: {error}"),
    }
    let schema_version = headers
        .get("x-wendao-schema-version")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    match observed_schema_version.lock() {
        Ok(mut guard) => {
            *guard = schema_version;
        }
        Err(error) => panic!("failed to lock observed schema version: {error}"),
    }

    let batches = match decode_record_batches_ipc(body.as_ref()) {
        Ok(batches) => batches,
        Err(error) => {
            return (
                StatusCode::BAD_REQUEST,
                format!("invalid Arrow request: {error}"),
            )
                .into_response();
        }
    };
    let payload = match encode_record_batches_ipc(batches.as_slice()) {
        Ok(payload) => payload,
        Err(error) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("failed to encode Arrow response: {error}"),
            )
                .into_response();
        }
    };

    let mut response = payload.into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static(ARROW_TRANSPORT_CONTENT_TYPE),
    );
    response.headers_mut().insert(
        "x-wendao-schema-version",
        HeaderValue::from_static(ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION),
    );
    response
}

async fn spawn_test_server(app: Router) -> String {
    let listener = match TcpListener::bind("127.0.0.1:0").await {
        Ok(listener) => listener,
        Err(error) => panic!("failed to bind test server: {error}"),
    };
    let address = match listener.local_addr() {
        Ok(address) => address,
        Err(error) => panic!("failed to resolve test server address: {error}"),
    };
    tokio::spawn(async move {
        if let Err(error) = serve(listener, app).await {
            panic!("Arrow transport test server crashed: {error}");
        }
    });
    format!("http://{address}")
}

fn sample_batch() -> RecordBatch {
    let schema = StdArc::new(Schema::new(vec![
        Field::new("doc_id", DataType::Utf8, false),
        Field::new("score", DataType::Float64, false),
    ]));
    match RecordBatch::try_new(
        schema,
        vec![
            StdArc::new(StringArray::from(vec!["doc-a", "doc-b"])),
            StdArc::new(Float64Array::from(vec![0.4_f64, 0.9_f64])),
        ],
    ) {
        Ok(batch) => batch,
        Err(error) => panic!("failed to build sample batch: {error}"),
    }
}

fn assert_string_column_eq(expected: &RecordBatch, actual: &RecordBatch, column: &str) {
    let expected = match expected.column_by_name(column) {
        Some(array_ref) => match array_ref.as_any().downcast_ref::<StringArray>() {
            Some(array) => array,
            None => panic!("expected `{column}` to be a StringArray"),
        },
        None => panic!("missing expected string column `{column}`"),
    };
    let actual = match actual.column_by_name(column) {
        Some(array_ref) => match array_ref.as_any().downcast_ref::<StringArray>() {
            Some(array) => array,
            None => panic!("actual `{column}` is not a StringArray"),
        },
        None => panic!("missing actual string column `{column}`"),
    };

    assert_eq!(expected.len(), actual.len());
    for row in 0..expected.len() {
        assert_eq!(expected.value(row), actual.value(row));
    }
}

fn assert_float_column_eq(expected: &RecordBatch, actual: &RecordBatch, column: &str) {
    let expected = match expected.column_by_name(column) {
        Some(array_ref) => match array_ref.as_any().downcast_ref::<Float64Array>() {
            Some(array) => array,
            None => panic!("expected `{column}` to be a Float64Array"),
        },
        None => panic!("missing expected float column `{column}`"),
    };
    let actual = match actual.column_by_name(column) {
        Some(array_ref) => match array_ref.as_any().downcast_ref::<Float64Array>() {
            Some(array) => array,
            None => panic!("actual `{column}` is not a Float64Array"),
        },
        None => panic!("missing actual float column `{column}`"),
    };

    assert_eq!(expected.len(), actual.len());
    for row in 0..expected.len() {
        assert_eq!(expected.value(row), actual.value(row));
    }
}
