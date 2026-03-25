use crate::search_plane::service::tests::support::*;

#[tokio::test]
async fn status_with_repo_runtime_hydrates_repo_corpus_status_from_snapshot_cache() {
    let temp_dir = temp_dir();
    let keyspace = unique_test_manifest_keyspace("status-hydrate");
    let service = SearchPlaneService::with_runtime(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        keyspace.clone(),
        SearchMaintenancePolicy::default(),
        SearchPlaneCache::for_tests(keyspace),
    );
    let documents = vec![RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: Arc::<str>::from("fn alpha() {}\n"),
        size_bytes: 14,
        modified_unix_ms: 0,
    }];
    publish_repo_bundle(&service, "alpha/repo", &documents, Some("rev-1")).await;
    service.synchronize_repo_runtime(&RepoIndexStatusResponse {
        total: 1,
        active: 0,
        queued: 0,
        checking: 0,
        syncing: 0,
        indexing: 0,
        ready: 1,
        unsupported: 0,
        failed: 0,
        target_concurrency: 1,
        max_concurrency: 1,
        sync_concurrency_limit: 1,
        current_repo_id: None,
        active_repo_ids: Vec::new(),
        repos: vec![repo_status_entry("alpha/repo", RepoIndexPhase::Ready)],
    });
    service.clear_all_in_memory_repo_runtime_for_test();

    let snapshot = ok_or_panic(
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let snapshot = service.status_with_repo_runtime().await;
                let repo_entity =
                    corpus_status(&snapshot, SearchCorpusKind::RepoEntity, "repo entity row");
                let repo_content = corpus_status(
                    &snapshot,
                    SearchCorpusKind::RepoContentChunk,
                    "repo content row",
                );
                if repo_entity.phase == SearchPlanePhase::Ready
                    && repo_content.phase == SearchPlanePhase::Ready
                {
                    break snapshot;
                }
                tokio::task::yield_now().await;
            }
        })
        .await,
        "repo-corpus snapshot cache should hydrate",
    );
    let repo_entity = corpus_status(&snapshot, SearchCorpusKind::RepoEntity, "repo entity row");
    let repo_content = corpus_status(
        &snapshot,
        SearchCorpusKind::RepoContentChunk,
        "repo content row",
    );

    assert_eq!(repo_entity.phase, SearchPlanePhase::Ready);
    assert!(repo_entity.active_epoch.is_some());
    assert!(repo_entity.row_count.unwrap_or_default() > 0);
    assert_eq!(repo_content.phase, SearchPlanePhase::Ready);
    assert!(repo_content.active_epoch.is_some());
    assert!(repo_content.row_count.unwrap_or_default() > 0);
}

#[test]
fn synchronize_repo_runtime_replaces_previous_snapshot_entries() {
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        PathBuf::from("/tmp/project/.data/search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );

    service.synchronize_repo_runtime(&RepoIndexStatusResponse {
        total: 2,
        active: 0,
        queued: 1,
        checking: 0,
        syncing: 0,
        indexing: 0,
        ready: 1,
        unsupported: 0,
        failed: 0,
        target_concurrency: 1,
        max_concurrency: 1,
        sync_concurrency_limit: 1,
        current_repo_id: None,
        active_repo_ids: Vec::new(),
        repos: vec![
            repo_status_entry("alpha/repo", RepoIndexPhase::Ready),
            repo_status_entry("beta/repo", RepoIndexPhase::Queued),
        ],
    });
    assert_eq!(
        repo_phase(&service, "alpha/repo"),
        Some(RepoIndexPhase::Ready)
    );
    assert_eq!(
        repo_phase(&service, "beta/repo"),
        Some(RepoIndexPhase::Queued)
    );

    service.synchronize_repo_runtime(&RepoIndexStatusResponse {
        total: 1,
        active: 0,
        queued: 1,
        checking: 0,
        syncing: 0,
        indexing: 0,
        ready: 0,
        unsupported: 0,
        failed: 0,
        target_concurrency: 1,
        max_concurrency: 1,
        sync_concurrency_limit: 1,
        current_repo_id: None,
        active_repo_ids: Vec::new(),
        repos: vec![repo_status_entry("beta/repo", RepoIndexPhase::Queued)],
    });

    assert_eq!(repo_phase(&service, "alpha/repo"), None);
    assert_eq!(
        repo_phase(&service, "beta/repo"),
        Some(RepoIndexPhase::Queued)
    );
}
