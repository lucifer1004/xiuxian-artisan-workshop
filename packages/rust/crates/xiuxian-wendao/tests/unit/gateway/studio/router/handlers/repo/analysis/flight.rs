use xiuxian_vector::LanceArray;

use crate::analyzers::{DocCoverageResult, DocRecord};
use crate::gateway::studio::router::handlers::repo::analysis::flight::{
    build_repo_doc_coverage_flight_batch, build_repo_doc_coverage_flight_metadata,
};

#[test]
fn repo_doc_coverage_flight_batch_preserves_doc_rows() {
    let batch = build_repo_doc_coverage_flight_batch(&[
        DocRecord {
            repo_id: "gateway-sync".to_string(),
            doc_id: "repo:gateway-sync:doc:README.md".to_string(),
            title: "README".to_string(),
            path: "README.md".to_string(),
            format: Some("markdown".to_string()),
        },
        DocRecord {
            repo_id: "gateway-sync".to_string(),
            doc_id: "repo:gateway-sync:doc:docs/solve.md".to_string(),
            title: "solve".to_string(),
            path: "docs/solve.md".to_string(),
            format: None,
        },
    ])
    .unwrap_or_else(|error| panic!("repo doc coverage batch should build: {error}"));

    assert_eq!(batch.num_rows(), 2);
    let Some(doc_id_column) = batch.column_by_name("docId") else {
        panic!("docId column");
    };
    let Some(doc_ids) = doc_id_column
        .as_any()
        .downcast_ref::<xiuxian_vector::LanceStringArray>()
    else {
        panic!("docId should be utf8");
    };
    assert_eq!(doc_ids.value(0), "repo:gateway-sync:doc:README.md");
    assert_eq!(doc_ids.value(1), "repo:gateway-sync:doc:docs/solve.md");

    let Some(format_column) = batch.column_by_name("format") else {
        panic!("format column");
    };
    let Some(formats) = format_column
        .as_any()
        .downcast_ref::<xiuxian_vector::LanceStringArray>()
    else {
        panic!("format should be utf8");
    };
    assert_eq!(formats.value(0), "markdown");
    assert!(formats.is_null(1));
}

#[test]
fn repo_doc_coverage_flight_metadata_preserves_summary_fields() {
    let metadata = build_repo_doc_coverage_flight_metadata(&DocCoverageResult {
        repo_id: "gateway-sync".to_string(),
        module_id: Some("GatewaySyncPkg".to_string()),
        docs: Vec::new(),
        covered_symbols: 3,
        uncovered_symbols: 1,
        hierarchical_uri: Some("repo://gateway-sync/docs".to_string()),
        hierarchy: Some(vec!["repo".to_string(), "gateway-sync".to_string()]),
    })
    .unwrap_or_else(|error| panic!("repo doc coverage metadata should encode: {error}"));

    let payload: serde_json::Value = serde_json::from_slice(&metadata)
        .unwrap_or_else(|error| panic!("metadata should decode: {error}"));
    assert_eq!(payload["repoId"], "gateway-sync");
    assert_eq!(payload["moduleId"], "GatewaySyncPkg");
    assert_eq!(payload["coveredSymbols"], 3);
    assert_eq!(payload["uncoveredSymbols"], 1);
    assert_eq!(payload["hierarchicalUri"], "repo://gateway-sync/docs");
    assert_eq!(payload["hierarchy"][0], "repo");
}
