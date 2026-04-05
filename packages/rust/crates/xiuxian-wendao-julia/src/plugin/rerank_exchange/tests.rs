use xiuxian_wendao_core::repo_intelligence::{
    RegisteredRepository, RepositoryPluginConfig, julia_arrow_request_schema,
    julia_arrow_response_schema,
};

use super::{
    PluginArrowRequestRow, PluginArrowScoreRow, build_plugin_arrow_request_batch,
    decode_plugin_arrow_score_rows, fetch_plugin_arrow_score_rows_for_repository,
};
use crate::julia_plugin_test_support::contract::{
    invalid_response_missing_analyzer_score_batch, request_batch, response_batch_without_trace_id,
};
use crate::julia_plugin_test_support::official_examples::{
    reserve_real_service_port, spawn_real_wendaoarrow_service, wait_for_service_ready,
};

#[test]
fn build_plugin_arrow_request_batch_uses_contract_columns() {
    let batch = build_plugin_arrow_request_batch(
        &[
            PluginArrowRequestRow {
                doc_id: "doc-1".to_string(),
                vector_score: 0.3,
                embedding: vec![1.0, 2.0, 3.0],
            },
            PluginArrowRequestRow {
                doc_id: "doc-2".to_string(),
                vector_score: 0.4,
                embedding: vec![4.0, 5.0, 6.0],
            },
        ],
        &[9.0, 8.0, 7.0],
    )
    .unwrap_or_else(|error| panic!("request batch should build: {error}"));

    assert_eq!(batch.num_rows(), 2);
    assert_eq!(batch.schema().field(0).name(), "doc_id");
    assert_eq!(batch.schema().field(1).name(), "vector_score");
    assert_eq!(batch.schema().field(2).name(), "embedding");
    assert_eq!(batch.schema().field(3).name(), "query_embedding");
}

#[test]
fn julia_arrow_request_schema_uses_contract_columns() {
    let schema = julia_arrow_request_schema(3);

    assert_eq!(schema.field(0).name(), "doc_id");
    assert_eq!(schema.field(1).name(), "vector_score");
    assert_eq!(schema.field(2).name(), "embedding");
    assert_eq!(schema.field(3).name(), "query_embedding");
}

#[test]
fn julia_arrow_response_schema_optionally_includes_trace_id() {
    let base = julia_arrow_response_schema(false);
    let traced = julia_arrow_response_schema(true);

    assert_eq!(base.fields().len(), 3);
    assert_eq!(traced.fields().len(), 4);
    assert_eq!(traced.field(3).name(), "trace_id");
}

#[test]
fn build_plugin_arrow_request_batch_rejects_dimension_mismatch() {
    let Err(error) = build_plugin_arrow_request_batch(
        &[PluginArrowRequestRow {
            doc_id: "doc-1".to_string(),
            vector_score: 0.3,
            embedding: vec![1.0, 2.0],
        }],
        &[9.0, 8.0, 7.0],
    ) else {
        panic!("dimension mismatch should fail");
    };

    assert!(
        error.to_string().contains("embedding dimension mismatch"),
        "unexpected error: {error}"
    );
}

#[test]
fn decode_plugin_arrow_score_rows_materializes_doc_scores() {
    let rows = decode_plugin_arrow_score_rows(&[response_batch_without_trace_id()])
        .unwrap_or_else(|error| panic!("decode should work: {error}"));

    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows.get("doc-a"),
        Some(&PluginArrowScoreRow {
            doc_id: "doc-a".to_string(),
            analyzer_score: 0.2,
            final_score: 0.5,
            trace_id: None,
        })
    );
    assert_eq!(
        rows.get("doc-b"),
        Some(&PluginArrowScoreRow {
            doc_id: "doc-b".to_string(),
            analyzer_score: 0.7,
            final_score: 0.9,
            trace_id: None,
        })
    );
}

#[test]
fn decode_plugin_arrow_score_rows_rejects_missing_columns() {
    let batch = invalid_response_missing_analyzer_score_batch();

    let Err(error) = decode_plugin_arrow_score_rows(&[batch]) else {
        panic!("decode should fail");
    };
    assert!(
        error
            .to_string()
            .contains("missing required Float64 column `analyzer_score`"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn fetch_plugin_arrow_score_rows_for_repository_roundtrips_remote_scores() {
    let port = reserve_real_service_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let _service = spawn_real_wendaoarrow_service(port);
    wait_for_service_ready(&base_url)
        .await
        .unwrap_or_else(|error| {
            panic!("real WendaoArrow Flight service should become ready: {error}")
        });
    let repository = RegisteredRepository {
        id: "demo".to_string(),
        plugins: vec![RepositoryPluginConfig::Config {
            id: "julia".to_string(),
            options: serde_json::json!({
                "flight_transport": {
                    "base_url": base_url,
                    "route": "/rerank",
                    "schema_version": "v1"
                }
            }),
        }],
        ..RegisteredRepository::default()
    };

    let rows = fetch_plugin_arrow_score_rows_for_repository(&repository, &[request_batch()])
        .await
        .unwrap_or_else(|error| panic!("transport should succeed: {error}"));

    assert_eq!(rows.len(), 2);
    assert_eq!(rows.get("doc-a").map(|row| row.analyzer_score), Some(0.4));
    assert_eq!(rows.get("doc-a").map(|row| row.final_score), Some(0.4));
}
