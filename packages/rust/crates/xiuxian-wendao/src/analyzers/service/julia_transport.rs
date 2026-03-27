#[cfg(feature = "julia")]
use std::collections::BTreeMap;
#[cfg(feature = "julia")]
use std::sync::Arc;

#[cfg(feature = "julia")]
use arrow::array::{Array, FixedSizeListArray, Float32Array, Float64Array, StringArray};
#[cfg(feature = "julia")]
use arrow::datatypes::{DataType, Field, Schema};
#[cfg(feature = "julia")]
use arrow::record_batch::RecordBatch;

#[cfg(feature = "julia")]
use crate::analyzers::config::RegisteredRepository;
#[cfg(feature = "julia")]
use crate::analyzers::errors::RepoIntelligenceError;
#[cfg(feature = "julia")]
use crate::analyzers::languages::process_julia_arrow_batches_for_repository;

/// One typed row materialized from the WendaoArrow Julia response contract.
#[cfg(feature = "julia")]
#[derive(Debug, Clone, PartialEq)]
pub struct JuliaArrowScoreRow {
    /// Stable document identifier emitted by the Rust request batch.
    pub doc_id: String,
    /// Julia-side analyzer score for the document.
    pub analyzer_score: f64,
    /// Final score after Julia-side reranking.
    pub final_score: f64,
    /// Optional trace identifier materialized from additive Julia response columns.
    pub trace_id: Option<String>,
}

/// One request row for the WendaoArrow `v1` Julia rerank contract.
#[cfg(feature = "julia")]
#[derive(Debug, Clone, PartialEq)]
pub struct JuliaArrowRequestRow {
    /// Stable document identifier for the candidate row.
    pub doc_id: String,
    /// Coarse Rust-side retrieval score.
    pub vector_score: f64,
    /// Candidate embedding forwarded to Julia.
    pub embedding: Vec<f32>,
}

/// Build one WendaoArrow `v1` request batch from typed Rust rows.
///
/// The request batch contains `doc_id`, `vector_score`, `embedding`, and
/// `query_embedding`, with the query vector repeated per row.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the request rows are empty, any row
/// carries an empty embedding, or the embedding dimensions do not match the
/// provided query vector dimension.
#[cfg(feature = "julia")]
pub fn build_julia_arrow_request_batch(
    rows: &[JuliaArrowRequestRow],
    query_vector: &[f32],
) -> Result<RecordBatch, RepoIntelligenceError> {
    if rows.is_empty() {
        return Err(contract_request_error(
            "WendaoArrow request batch requires at least one row",
        ));
    }
    if query_vector.is_empty() {
        return Err(contract_request_error(
            "WendaoArrow request batch requires a non-empty query vector",
        ));
    }

    let expected_dim = query_vector.len();
    let Some(vector_dim) = i32::try_from(expected_dim).ok() else {
        return Err(contract_request_error(format!(
            "query vector dimension {expected_dim} exceeds i32 range"
        )));
    };

    let mut doc_ids = Vec::with_capacity(rows.len());
    let mut vector_scores = Vec::with_capacity(rows.len());
    let mut embedding_values = Vec::with_capacity(rows.len() * expected_dim);
    let mut query_embedding_values = Vec::with_capacity(rows.len() * expected_dim);

    for row in rows {
        if row.doc_id.trim().is_empty() {
            return Err(contract_request_error(
                "WendaoArrow request row `doc_id` must be non-empty",
            ));
        }
        if row.embedding.len() != expected_dim {
            return Err(contract_request_error(format!(
                "embedding dimension mismatch for doc_id `{}`: expected {}, found {}",
                row.doc_id,
                expected_dim,
                row.embedding.len()
            )));
        }

        doc_ids.push(row.doc_id.as_str());
        vector_scores.push(row.vector_score);
        embedding_values.extend_from_slice(row.embedding.as_slice());
        query_embedding_values.extend_from_slice(query_vector);
    }

    let schema = Arc::new(Schema::new(vec![
        Field::new("doc_id", DataType::Utf8, false),
        Field::new("vector_score", DataType::Float64, false),
        Field::new(
            "embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                vector_dim,
            ),
            false,
        ),
        Field::new(
            "query_embedding",
            DataType::FixedSizeList(
                Arc::new(Field::new("item", DataType::Float32, true)),
                vector_dim,
            ),
            false,
        ),
    ]));

    let embedding = FixedSizeListArray::try_new(
        Arc::new(Field::new("item", DataType::Float32, true)),
        vector_dim,
        Arc::new(Float32Array::from(embedding_values)),
        None,
    )
    .map_err(|error| contract_request_error(error.to_string()))?;
    let query_embedding = FixedSizeListArray::try_new(
        Arc::new(Field::new("item", DataType::Float32, true)),
        vector_dim,
        Arc::new(Float32Array::from(query_embedding_values)),
        None,
    )
    .map_err(|error| contract_request_error(error.to_string()))?;

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(doc_ids)),
            Arc::new(Float64Array::from(vector_scores)),
            Arc::new(embedding),
            Arc::new(query_embedding),
        ],
    )
    .map_err(|error| contract_request_error(error.to_string()))
}

/// Decode Julia Arrow response batches into a `doc_id` keyed score map.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the response batch shape does not
/// match the WendaoArrow `v1` response contract.
#[cfg(feature = "julia")]
pub fn decode_julia_arrow_score_rows(
    batches: &[RecordBatch],
) -> Result<BTreeMap<String, JuliaArrowScoreRow>, RepoIntelligenceError> {
    let mut rows = BTreeMap::new();

    for batch in batches {
        let doc_id = batch
            .column_by_name("doc_id")
            .and_then(|array| array.as_any().downcast_ref::<StringArray>())
            .ok_or_else(|| contract_decode_error("missing required Utf8 column `doc_id`"))?;
        let analyzer_score = batch
            .column_by_name("analyzer_score")
            .and_then(|array| array.as_any().downcast_ref::<Float64Array>())
            .ok_or_else(|| {
                contract_decode_error("missing required Float64 column `analyzer_score`")
            })?;
        let final_score = batch
            .column_by_name("final_score")
            .and_then(|array| array.as_any().downcast_ref::<Float64Array>())
            .ok_or_else(|| {
                contract_decode_error("missing required Float64 column `final_score`")
            })?;
        let trace_id = batch
            .column_by_name("trace_id")
            .map(|array| {
                array
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .ok_or_else(|| contract_decode_error("optional `trace_id` column must be Utf8"))
            })
            .transpose()?;

        for row in 0..batch.num_rows() {
            let doc_id_value = doc_id
                .is_valid(row)
                .then(|| doc_id.value(row).to_string())
                .ok_or_else(|| contract_decode_error("`doc_id` must be non-null"))?;
            let analyzer_score_value = analyzer_score
                .is_valid(row)
                .then(|| analyzer_score.value(row))
                .ok_or_else(|| contract_decode_error("`analyzer_score` must be non-null"))?;
            let final_score_value = final_score
                .is_valid(row)
                .then(|| final_score.value(row))
                .ok_or_else(|| contract_decode_error("`final_score` must be non-null"))?;

            rows.insert(
                doc_id_value.clone(),
                JuliaArrowScoreRow {
                    doc_id: doc_id_value,
                    analyzer_score: analyzer_score_value,
                    final_score: final_score_value,
                    trace_id: trace_id.and_then(|array| {
                        array.is_valid(row).then(|| array.value(row).to_string())
                    }),
                },
            );
        }
    }

    Ok(rows)
}

/// Execute the repository-configured Julia Arrow transport and materialize the
/// validated response into typed score rows.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the remote roundtrip fails or the
/// decoded response cannot be materialized into the WendaoArrow `v1` score row
/// contract.
#[cfg(feature = "julia")]
pub async fn fetch_julia_arrow_score_rows_for_repository(
    repository: &RegisteredRepository,
    batches: &[RecordBatch],
) -> Result<BTreeMap<String, JuliaArrowScoreRow>, RepoIntelligenceError> {
    let response_batches = process_julia_arrow_batches_for_repository(repository, batches).await?;
    decode_julia_arrow_score_rows(response_batches.as_slice())
}

#[cfg(feature = "julia")]
fn contract_decode_error(message: impl Into<String>) -> RepoIntelligenceError {
    RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "failed to decode Julia Arrow score rows from WendaoArrow `v1` contract: {}",
            message.into()
        ),
    }
}

#[cfg(feature = "julia")]
fn contract_request_error(message: impl Into<String>) -> RepoIntelligenceError {
    RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "failed to build WendaoArrow `v1` request batch: {}",
            message.into()
        ),
    }
}

#[cfg(test)]
mod tests {
    use arrow::array::{Float64Array, StringArray};
    use axum::body::Bytes;
    use axum::routing::post;
    use axum::{Router, serve};
    use serde_json::json;
    use tokio::net::TcpListener;
    use xiuxian_vector::{
        ARROW_TRANSPORT_CONTENT_TYPE, decode_record_batches_ipc, encode_record_batches_ipc,
    };

    use super::*;
    use crate::analyzers::config::{RepositoryPluginConfig, RepositoryRefreshPolicy};

    #[test]
    fn build_julia_arrow_request_batch_uses_contract_columns() {
        let batch = build_julia_arrow_request_batch(
            &[
                JuliaArrowRequestRow {
                    doc_id: "doc-1".to_string(),
                    vector_score: 0.3,
                    embedding: vec![1.0, 2.0, 3.0],
                },
                JuliaArrowRequestRow {
                    doc_id: "doc-2".to_string(),
                    vector_score: 0.4,
                    embedding: vec![4.0, 5.0, 6.0],
                },
            ],
            &[9.0, 8.0, 7.0],
        )
        .expect("request batch should build");

        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.schema().field(0).name(), "doc_id");
        assert_eq!(batch.schema().field(1).name(), "vector_score");
        assert_eq!(batch.schema().field(2).name(), "embedding");
        assert_eq!(batch.schema().field(3).name(), "query_embedding");
    }

    #[test]
    fn build_julia_arrow_request_batch_rejects_dimension_mismatch() {
        let error = build_julia_arrow_request_batch(
            &[JuliaArrowRequestRow {
                doc_id: "doc-1".to_string(),
                vector_score: 0.3,
                embedding: vec![1.0, 2.0],
            }],
            &[9.0, 8.0, 7.0],
        )
        .expect_err("dimension mismatch should fail");

        assert!(
            error.to_string().contains("embedding dimension mismatch"),
            "unexpected error: {error}"
        );
    }

    #[test]
    fn decode_julia_arrow_score_rows_materializes_doc_scores() {
        let rows = decode_julia_arrow_score_rows(&[response_batch()]).expect("decode should work");

        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows.get("doc-1"),
            Some(&JuliaArrowScoreRow {
                doc_id: "doc-1".to_string(),
                analyzer_score: 0.9,
                final_score: 0.95,
                trace_id: None,
            })
        );
        assert_eq!(
            rows.get("doc-2"),
            Some(&JuliaArrowScoreRow {
                doc_id: "doc-2".to_string(),
                analyzer_score: 0.7,
                final_score: 0.8,
                trace_id: None,
            })
        );
    }

    #[test]
    fn decode_julia_arrow_score_rows_materializes_optional_trace_id() {
        let rows = decode_julia_arrow_score_rows(&[response_batch_with_trace_ids()])
            .expect("decode should work");

        assert_eq!(
            rows.get("doc-1").and_then(|row| row.trace_id.as_deref()),
            Some("trace-123")
        );
        assert_eq!(
            rows.get("doc-2").and_then(|row| row.trace_id.as_deref()),
            Some("trace-123")
        );
    }

    #[test]
    fn decode_julia_arrow_score_rows_rejects_missing_columns() {
        let schema = Arc::new(Schema::new(vec![
            Field::new("doc_id", DataType::Utf8, false),
            Field::new("final_score", DataType::Float64, false),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["doc-1"])),
                Arc::new(Float64Array::from(vec![0.95])),
            ],
        )
        .expect("batch");

        let error = decode_julia_arrow_score_rows(&[batch]).expect_err("decode should fail");
        assert!(
            error
                .to_string()
                .contains("missing required Float64 column `analyzer_score`"),
            "unexpected error: {error}"
        );
    }

    #[tokio::test]
    async fn fetch_julia_arrow_score_rows_for_repository_roundtrips_remote_scores() {
        let app = Router::new().route(
            "/arrow-ipc",
            post(|body: Bytes| async move {
                let request_batches =
                    decode_record_batches_ipc(&body).expect("request batches should decode");
                assert_eq!(request_batches.len(), 1);

                let payload =
                    encode_record_batches_ipc(&[response_batch()]).expect("response payload");
                (
                    [
                        ("content-type", ARROW_TRANSPORT_CONTENT_TYPE),
                        ("x-wendao-schema-version", "v1"),
                    ],
                    payload,
                )
            }),
        );
        let base_url = spawn_test_server(app).await;
        let repository = RegisteredRepository {
            id: "demo".to_string(),
            path: None,
            url: None,
            git_ref: None,
            refresh: RepositoryRefreshPolicy::Fetch,
            plugins: vec![RepositoryPluginConfig::Config {
                id: "julia".to_string(),
                options: json!({
                    "arrow_transport": {
                        "base_url": base_url,
                        "route": "/arrow-ipc",
                        "schema_version": "v1"
                    }
                }),
            }],
        };

        let rows = fetch_julia_arrow_score_rows_for_repository(&repository, &[request_batch()])
            .await
            .expect("transport should succeed");

        assert_eq!(rows.len(), 2);
        assert_eq!(rows.get("doc-1").map(|row| row.final_score), Some(0.95));
    }

    fn request_batch() -> RecordBatch {
        build_julia_arrow_request_batch(
            &[
                JuliaArrowRequestRow {
                    doc_id: "doc-1".to_string(),
                    vector_score: 0.3,
                    embedding: vec![1.0, 2.0, 3.0],
                },
                JuliaArrowRequestRow {
                    doc_id: "doc-2".to_string(),
                    vector_score: 0.4,
                    embedding: vec![4.0, 5.0, 6.0],
                },
            ],
            &[9.0, 8.0, 7.0],
        )
        .expect("request batch")
    }

    fn response_batch() -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("doc_id", DataType::Utf8, false),
            Field::new("analyzer_score", DataType::Float64, false),
            Field::new("final_score", DataType::Float64, false),
        ]));
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["doc-1", "doc-2"])),
                Arc::new(Float64Array::from(vec![0.9, 0.7])),
                Arc::new(Float64Array::from(vec![0.95, 0.8])),
            ],
        )
        .expect("response batch")
    }

    fn response_batch_with_trace_ids() -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("doc_id", DataType::Utf8, false),
            Field::new("analyzer_score", DataType::Float64, false),
            Field::new("final_score", DataType::Float64, false),
            Field::new("trace_id", DataType::Utf8, false),
        ]));
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["doc-1", "doc-2"])),
                Arc::new(Float64Array::from(vec![0.9, 0.7])),
                Arc::new(Float64Array::from(vec![0.95, 0.8])),
                Arc::new(StringArray::from(vec!["trace-123", "trace-123"])),
            ],
        )
        .expect("response batch")
    }

    async fn spawn_test_server(app: Router) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let address = listener.local_addr().expect("local addr");
        tokio::spawn(async move {
            serve(listener, app).await.expect("server should run");
        });
        format!("http://{address}")
    }
}
