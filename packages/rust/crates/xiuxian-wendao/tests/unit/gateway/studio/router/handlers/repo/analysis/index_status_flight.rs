#[cfg(feature = "duckdb")]
use crate::duckdb::LocalRelationEngineKind;
use xiuxian_vector::LanceArray;

#[cfg(feature = "duckdb")]
use crate::gateway::studio::router::handlers::repo::analysis::index_status_flight::configured_repo_index_status_diagnostics_engine_kind;
use crate::gateway::studio::router::handlers::repo::analysis::index_status_flight::{
    build_repo_index_status_flight_batch, build_repo_index_status_flight_metadata,
    repo_index_status_response_with_diagnostics, summarize_repo_index_status_diagnostics,
};
use crate::repo_index::{RepoIndexEntryStatus, RepoIndexPhase, RepoIndexStatusResponse};
#[cfg(feature = "duckdb")]
use crate::set_link_graph_wendao_config_override;

#[test]
fn repo_index_status_flight_batch_preserves_summary_fields() {
    let batch = build_repo_index_status_flight_batch(&RepoIndexStatusResponse {
        total: 3,
        active: 2,
        queued: 1,
        checking: 0,
        syncing: 1,
        indexing: 1,
        ready: 1,
        unsupported: 0,
        failed: 0,
        target_concurrency: 2,
        max_concurrency: 4,
        sync_concurrency_limit: 1,
        current_repo_id: Some("gateway-sync".to_string()),
        active_repo_ids: vec!["gateway-sync".to_string()],
        repos: vec![RepoIndexEntryStatus {
            repo_id: "gateway-sync".to_string(),
            phase: RepoIndexPhase::Ready,
            queue_position: None,
            last_error: None,
            last_revision: Some("rev:123".to_string()),
            updated_at: Some("2026-04-03T19:15:00Z".to_string()),
            attempt_count: 2,
        }],
    })
    .unwrap_or_else(|error| panic!("repo index status batch should build: {error}"));

    assert_eq!(batch.num_rows(), 1);
    let Some(ready_column) = batch.column_by_name("ready") else {
        panic!("ready column");
    };
    let Some(ready) = ready_column
        .as_any()
        .downcast_ref::<xiuxian_vector::LanceInt32Array>()
    else {
        panic!("ready should be int32");
    };
    assert_eq!(ready.value(0), 1);

    let Some(repos_json_column) = batch.column_by_name("reposJson") else {
        panic!("reposJson column");
    };
    let Some(repos_json) = repos_json_column
        .as_any()
        .downcast_ref::<xiuxian_vector::LanceStringArray>()
    else {
        panic!("reposJson should be utf8");
    };
    assert!(repos_json.value(0).contains("gateway-sync"));
}

#[test]
fn repo_index_status_flight_metadata_preserves_summary_fields() {
    let metadata = build_repo_index_status_flight_metadata(&RepoIndexStatusResponse {
        total: 3,
        active: 2,
        queued: 1,
        checking: 0,
        syncing: 1,
        indexing: 1,
        ready: 1,
        unsupported: 0,
        failed: 0,
        target_concurrency: 2,
        max_concurrency: 4,
        sync_concurrency_limit: 1,
        current_repo_id: Some("gateway-sync".to_string()),
        active_repo_ids: vec!["gateway-sync".to_string()],
        repos: vec![RepoIndexEntryStatus {
            repo_id: "gateway-sync".to_string(),
            phase: RepoIndexPhase::Ready,
            queue_position: None,
            last_error: None,
            last_revision: Some("rev:123".to_string()),
            updated_at: Some("2026-04-03T19:15:00Z".to_string()),
            attempt_count: 2,
        }],
    })
    .unwrap_or_else(|error| panic!("repo index status metadata should encode: {error}"));

    let payload: serde_json::Value = serde_json::from_slice(&metadata)
        .unwrap_or_else(|error| panic!("metadata should decode: {error}"));
    assert_eq!(payload["total"], 3);
    assert_eq!(payload["syncConcurrencyLimit"], 1);
    assert_eq!(payload["currentRepoId"], "gateway-sync");
    assert_eq!(payload["repos"][0]["repoId"], "gateway-sync");
}

#[tokio::test]
async fn repo_index_status_diagnostics_recompute_summary_counts_from_rows() {
    let input = RepoIndexStatusResponse {
        total: 99,
        active: 99,
        queued: 99,
        checking: 99,
        syncing: 99,
        indexing: 99,
        ready: 99,
        unsupported: 99,
        failed: 99,
        target_concurrency: 2,
        max_concurrency: 4,
        sync_concurrency_limit: 1,
        current_repo_id: Some("stale-current".to_string()),
        active_repo_ids: vec!["gateway-failed".to_string(), "gateway-sync".to_string()],
        repos: vec![
            RepoIndexEntryStatus {
                repo_id: "gateway-sync".to_string(),
                phase: RepoIndexPhase::Queued,
                queue_position: Some(1),
                last_error: None,
                last_revision: Some("rev:123".to_string()),
                updated_at: Some("2026-04-03T19:15:00Z".to_string()),
                attempt_count: 2,
            },
            RepoIndexEntryStatus {
                repo_id: "gateway-ready".to_string(),
                phase: RepoIndexPhase::Ready,
                queue_position: None,
                last_error: None,
                last_revision: Some("rev:456".to_string()),
                updated_at: Some("2026-04-03T19:16:00Z".to_string()),
                attempt_count: 1,
            },
            RepoIndexEntryStatus {
                repo_id: "gateway-failed".to_string(),
                phase: RepoIndexPhase::Failed,
                queue_position: None,
                last_error: Some("boom".to_string()),
                last_revision: None,
                updated_at: Some("2026-04-03T19:17:00Z".to_string()),
                attempt_count: 3,
            },
        ],
    };

    let summary = summarize_repo_index_status_diagnostics(&input)
        .await
        .unwrap_or_else(|error| panic!("repo index diagnostics summary should build: {error}"));
    assert_eq!(summary.total, 3);
    assert_eq!(summary.active, 2);
    assert_eq!(summary.queued, 1);
    assert_eq!(summary.ready, 1);
    assert_eq!(summary.failed, 1);
    assert_eq!(
        summary.active_repo_ids,
        vec!["gateway-failed".to_string(), "gateway-sync".to_string()]
    );
    assert_eq!(summary.current_repo_id.as_deref(), Some("gateway-failed"));

    let response = repo_index_status_response_with_diagnostics(&input).await;

    assert_eq!(response.total, 3);
    assert_eq!(response.active, 2);
    assert_eq!(response.queued, 1);
    assert_eq!(response.checking, 0);
    assert_eq!(response.syncing, 0);
    assert_eq!(response.indexing, 0);
    assert_eq!(response.ready, 1);
    assert_eq!(response.unsupported, 0);
    assert_eq!(response.failed, 1);
    assert_eq!(response.target_concurrency, 2);
    assert_eq!(response.current_repo_id.as_deref(), Some("gateway-failed"));
    assert_eq!(
        response.active_repo_ids,
        vec!["gateway-failed".to_string(), "gateway-sync".to_string()]
    );
}

#[cfg(feature = "duckdb")]
#[tokio::test]
async fn repo_index_status_diagnostics_select_duckdb_when_enabled() {
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

    assert_eq!(
        configured_repo_index_status_diagnostics_engine_kind()
            .unwrap_or(LocalRelationEngineKind::DataFusion),
        LocalRelationEngineKind::DuckDb
    );

    let input = RepoIndexStatusResponse {
        total: 0,
        active: 0,
        queued: 0,
        checking: 0,
        syncing: 0,
        indexing: 0,
        ready: 0,
        unsupported: 0,
        failed: 0,
        target_concurrency: 2,
        max_concurrency: 4,
        sync_concurrency_limit: 1,
        current_repo_id: Some("stale-current".to_string()),
        active_repo_ids: vec!["gateway-sync".to_string(), "gateway-ready".to_string()],
        repos: vec![
            RepoIndexEntryStatus {
                repo_id: "gateway-ready".to_string(),
                phase: RepoIndexPhase::Ready,
                queue_position: None,
                last_error: None,
                last_revision: Some("rev:456".to_string()),
                updated_at: Some("2026-04-03T19:16:00Z".to_string()),
                attempt_count: 1,
            },
            RepoIndexEntryStatus {
                repo_id: "gateway-sync".to_string(),
                phase: RepoIndexPhase::Syncing,
                queue_position: None,
                last_error: None,
                last_revision: Some("rev:789".to_string()),
                updated_at: Some("2026-04-03T19:18:00Z".to_string()),
                attempt_count: 2,
            },
        ],
    };

    let summary = summarize_repo_index_status_diagnostics(&input)
        .await
        .unwrap_or_else(|error| panic!("repo index diagnostics summary should build: {error}"));
    assert_eq!(summary.total, 2);
    assert_eq!(summary.active, 2);
    assert_eq!(summary.ready, 1);
    assert_eq!(summary.syncing, 1);
    assert_eq!(
        summary.active_repo_ids,
        vec!["gateway-sync".to_string(), "gateway-ready".to_string()]
    );
    assert_eq!(summary.current_repo_id.as_deref(), Some("gateway-sync"));

    let response = repo_index_status_response_with_diagnostics(&input).await;

    assert_eq!(response.total, 2);
    assert_eq!(response.active, 2);
    assert_eq!(response.ready, 1);
    assert_eq!(response.syncing, 1);
    assert_eq!(response.current_repo_id.as_deref(), Some("gateway-sync"));
    assert_eq!(
        response.active_repo_ids,
        vec!["gateway-sync".to_string(), "gateway-ready".to_string()]
    );
}
