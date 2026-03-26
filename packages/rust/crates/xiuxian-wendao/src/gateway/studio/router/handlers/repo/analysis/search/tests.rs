use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use crate::gateway::studio::router::StudioApiError;
use crate::search_plane::{SearchMaintenancePolicy, SearchManifestKeyspace, SearchPlaneService};

use crate::gateway::studio::router::handlers::repo::analysis::search::cache::with_cached_repo_search_result;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct CachedRepoSearchProbe {
    value: String,
}

#[tokio::test]
async fn cached_repo_search_result_reuses_hot_query_payload() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let keyspace = SearchManifestKeyspace::new("xiuxian:test:repo_gateway_cache");
    let search_plane = SearchPlaneService::with_test_cache(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        keyspace.clone(),
        SearchMaintenancePolicy::default(),
    );
    let load_count = Arc::new(AtomicUsize::new(0));

    let first = with_cached_repo_search_result(
        &search_plane,
        "repo.symbol-search",
        "alpha/repo",
        "solve",
        5,
        {
            let load_count = Arc::clone(&load_count);
            || async move {
                load_count.fetch_add(1, Ordering::SeqCst);
                Ok(CachedRepoSearchProbe {
                    value: "first".to_string(),
                })
            }
        },
    )
    .await
    .unwrap_or_else(|error| panic!("first cached search result: {error:?}"));

    let second = with_cached_repo_search_result(
        &search_plane,
        "repo.symbol-search",
        "alpha/repo",
        "solve",
        5,
        {
            let load_count = Arc::clone(&load_count);
            || async move {
                load_count.fetch_add(1, Ordering::SeqCst);
                Err(StudioApiError::internal(
                    "UNEXPECTED_RELOAD",
                    "cached repo search should not execute loader twice",
                    None,
                ))
            }
        },
    )
    .await
    .unwrap_or_else(|error| panic!("cached repo search hit should succeed: {error:?}"));

    assert_eq!(first, second);
    assert_eq!(first.value, "first");
    assert_eq!(load_count.load(Ordering::SeqCst), 1);
}
