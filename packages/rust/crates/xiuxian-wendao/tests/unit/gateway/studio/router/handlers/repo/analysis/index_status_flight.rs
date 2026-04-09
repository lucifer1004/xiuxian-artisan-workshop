use xiuxian_vector::LanceArray;

use crate::gateway::studio::router::handlers::repo::analysis::index_status_flight::{
    build_repo_index_status_flight_batch, build_repo_index_status_flight_metadata,
};
use crate::repo_index::{RepoIndexEntryStatus, RepoIndexPhase, RepoIndexStatusResponse};

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
