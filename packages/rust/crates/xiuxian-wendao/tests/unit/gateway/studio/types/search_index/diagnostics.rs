#[cfg(feature = "duckdb")]
use crate::duckdb::LocalRelationEngineKind;
use crate::gateway::studio::types::search_index::SearchIndexStatusResponse;
#[cfg(feature = "duckdb")]
use crate::gateway::studio::types::search_index::configured_status_diagnostics_engine_kind;
use crate::search::SearchPlaneStatusSnapshot;
#[cfg(feature = "duckdb")]
use crate::set_link_graph_wendao_config_override;

use super::helpers::{
    compacting_local_symbol_status, degraded_repo_entity_status, telemetry_attachment_status,
    telemetry_knowledge_status,
};

fn sample_status_snapshot() -> SearchPlaneStatusSnapshot {
    SearchPlaneStatusSnapshot {
        repo_read_pressure: None,
        corpora: vec![
            compacting_local_symbol_status(),
            degraded_repo_entity_status(),
            telemetry_attachment_status(),
            telemetry_knowledge_status(),
        ],
    }
}

#[tokio::test]
async fn diagnostics_rollup_matches_rust_status_response() {
    let snapshot = sample_status_snapshot();

    let response = SearchIndexStatusResponse::from_snapshot_with_diagnostics(&snapshot).await;
    let baseline = SearchIndexStatusResponse::from(&snapshot);

    assert_eq!(response, baseline);
}

#[cfg(feature = "duckdb")]
#[tokio::test]
async fn diagnostics_rollup_selects_duckdb_when_enabled() {
    let temp = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
    let config_path = temp.path().join("wendao.toml");
    std::fs::write(
        &config_path,
        r#"[search.duckdb]
enabled = true
database_path = ":memory:"
temp_directory = ".data/duckdb/tmp"
threads = 2
materialize_threshold_rows = 16
prefer_virtual_arrow = true
"#,
    )
    .unwrap_or_else(|error| panic!("write config: {error}"));
    set_link_graph_wendao_config_override(&config_path.to_string_lossy());

    let snapshot = sample_status_snapshot();
    let response = SearchIndexStatusResponse::from_snapshot_with_diagnostics(&snapshot).await;
    let baseline = SearchIndexStatusResponse::from(&snapshot);

    assert_eq!(
        configured_status_diagnostics_engine_kind().unwrap_or(LocalRelationEngineKind::DataFusion),
        LocalRelationEngineKind::DuckDb
    );
    assert_eq!(response, baseline);
}
