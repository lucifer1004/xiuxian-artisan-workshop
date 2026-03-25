use crate::search_plane::repo_staging::versioned_repo_table_name;
use crate::search_plane::service::core::RepoMaintenanceTaskKind;
use crate::search_plane::service::tests::support::*;

#[tokio::test]
async fn search_repo_entities_reads_hits_from_published_table() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );

    ok_or_panic(
        service
            .publish_repo_entities_with_revision(
                "alpha/repo",
                &sample_repo_analysis(),
                &sample_repo_documents(),
                None,
            )
            .await,
        "publish repo entities",
    );

    let kind_filters = HashSet::from_iter([String::from("function")]);
    let hits = ok_or_panic(
        service
            .search_repo_entities("alpha/repo", "reexport", &HashSet::new(), &kind_filters, 5)
            .await,
        "query repo entities",
    );

    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].doc_type.as_deref(), Some("symbol"));
    assert_eq!(hits[0].stem, "reexport");
    assert_eq!(hits[0].path, "src/BaseModelica.jl");
    assert_eq!(hits[0].match_reason.as_deref(), Some("repo_symbol_search"));
}

#[tokio::test]
async fn repo_search_query_cache_key_uses_synchronized_runtime_state() {
    let temp_dir = temp_dir();
    let keyspace = service_test_manifest_keyspace();
    let service = SearchPlaneService::with_runtime(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        keyspace.clone(),
        SearchMaintenancePolicy::default(),
        SearchPlaneCache::for_tests(keyspace),
    );

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

    let ready_key = some_or_panic(
        service
            .repo_search_query_cache_key(RepoSearchQueryCacheKeyInput {
                scope: "code_search",
                corpora: &[],
                repo_corpora: &[SearchCorpusKind::RepoEntity],
                repo_ids: &[String::from("alpha/repo")],
                query: "alpha",
                limit: 10,
                intent: Some("code_search"),
                repo_hint: Some("alpha/repo"),
            })
            .await,
        "cache key should exist",
    );

    service.synchronize_repo_runtime(&RepoIndexStatusResponse {
        total: 1,
        active: 1,
        queued: 0,
        checking: 0,
        syncing: 0,
        indexing: 1,
        ready: 0,
        unsupported: 0,
        failed: 0,
        target_concurrency: 1,
        max_concurrency: 1,
        sync_concurrency_limit: 1,
        current_repo_id: Some("alpha/repo".to_string()),
        active_repo_ids: vec!["alpha/repo".to_string()],
        repos: vec![RepoIndexEntryStatus {
            last_revision: Some("rev-2".to_string()),
            ..repo_status_entry("alpha/repo", RepoIndexPhase::Indexing)
        }],
    });

    let refreshing_key = some_or_panic(
        service
            .repo_search_query_cache_key(RepoSearchQueryCacheKeyInput {
                scope: "code_search",
                corpora: &[],
                repo_corpora: &[SearchCorpusKind::RepoEntity],
                repo_ids: &[String::from("alpha/repo")],
                query: "alpha",
                limit: 10,
                intent: Some("code_search"),
                repo_hint: Some("alpha/repo"),
            })
            .await,
        "cache key should exist",
    );

    assert_ne!(ready_key, refreshing_key);
}

#[tokio::test]
async fn repo_search_publication_state_prefers_publications_over_runtime_phase() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: Arc::<str>::from("fn alpha() {}\n"),
        size_bytes: 14,
        modified_unix_ms: 0,
    }];
    publish_repo_bundle(&service, "searchable/repo", &documents, Some("rev-1")).await;
    service.synchronize_repo_runtime(&RepoIndexStatusResponse {
        total: 3,
        active: 1,
        queued: 1,
        checking: 0,
        syncing: 0,
        indexing: 1,
        ready: 0,
        unsupported: 0,
        failed: 1,
        target_concurrency: 1,
        max_concurrency: 1,
        sync_concurrency_limit: 1,
        current_repo_id: Some("searchable/repo".to_string()),
        active_repo_ids: vec!["searchable/repo".to_string()],
        repos: vec![
            RepoIndexEntryStatus {
                last_revision: Some("rev-2".to_string()),
                ..repo_status_entry("searchable/repo", RepoIndexPhase::Indexing)
            },
            repo_status_entry("pending/repo", RepoIndexPhase::Queued),
            repo_status_entry("failed/repo", RepoIndexPhase::Failed),
        ],
    });

    let searchable = service
        .repo_search_publication_state("searchable/repo")
        .await;
    let pending = service.repo_search_publication_state("pending/repo").await;
    let skipped = service.repo_search_publication_state("failed/repo").await;

    assert_eq!(searchable.availability, RepoSearchAvailability::Searchable);
    assert!(searchable.entity_published);
    assert!(searchable.content_published);
    assert_eq!(pending.availability, RepoSearchAvailability::Pending);
    assert!(!pending.entity_published);
    assert!(!pending.content_published);
    assert_eq!(skipped.availability, RepoSearchAvailability::Skipped);
    assert!(!skipped.entity_published);
    assert!(!skipped.content_published);
}

#[tokio::test]
async fn repo_search_publication_states_batches_repo_snapshot_reads() {
    let temp_dir = temp_dir();
    let keyspace = unique_test_manifest_keyspace("batch-publication-state");
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
    publish_repo_bundle(&service, "searchable/repo", &documents, Some("rev-1")).await;
    service.synchronize_repo_runtime(&RepoIndexStatusResponse {
        total: 3,
        active: 0,
        queued: 1,
        checking: 0,
        syncing: 0,
        indexing: 0,
        ready: 1,
        unsupported: 0,
        failed: 1,
        target_concurrency: 1,
        max_concurrency: 1,
        sync_concurrency_limit: 1,
        current_repo_id: None,
        active_repo_ids: Vec::new(),
        repos: vec![
            repo_status_entry("searchable/repo", RepoIndexPhase::Ready),
            repo_status_entry("pending/repo", RepoIndexPhase::Queued),
            repo_status_entry("failed/repo", RepoIndexPhase::Failed),
        ],
    });
    service.clear_all_in_memory_repo_runtime_for_test();

    let repo_ids = vec![
        "searchable/repo".to_string(),
        "pending/repo".to_string(),
        "failed/repo".to_string(),
    ];
    let states = ok_or_panic(
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let states = service
                    .repo_search_publication_states(repo_ids.as_slice())
                    .await;
                if states.get("failed/repo").map(|state| state.availability)
                    == Some(RepoSearchAvailability::Skipped)
                {
                    break states;
                }
                tokio::task::yield_now().await;
            }
        })
        .await,
        "repo publication states should hydrate",
    );

    assert_eq!(
        states
            .get("searchable/repo")
            .map(|state| state.availability),
        Some(RepoSearchAvailability::Searchable)
    );
    assert_eq!(
        states
            .get("searchable/repo")
            .map(|state| state.entity_published),
        Some(true)
    );
    assert_eq!(
        states.get("pending/repo").map(|state| state.availability),
        Some(RepoSearchAvailability::Pending)
    );
    assert_eq!(
        states.get("failed/repo").map(|state| state.availability),
        Some(RepoSearchAvailability::Skipped)
    );
}

#[tokio::test]
async fn repo_search_publication_state_hydrates_from_repo_corpus_snapshot_after_memory_miss() {
    let temp_dir = temp_dir();
    let keyspace = unique_test_manifest_keyspace("runtime-hydrate");
    let service = SearchPlaneService::with_runtime(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        keyspace.clone(),
        SearchMaintenancePolicy::default(),
        SearchPlaneCache::for_tests(keyspace),
    );

    service.synchronize_repo_runtime(&RepoIndexStatusResponse {
        total: 1,
        active: 0,
        queued: 0,
        checking: 0,
        syncing: 0,
        indexing: 0,
        ready: 0,
        unsupported: 0,
        failed: 1,
        target_concurrency: 1,
        max_concurrency: 1,
        sync_concurrency_limit: 1,
        current_repo_id: None,
        active_repo_ids: Vec::new(),
        repos: vec![repo_status_entry("failed/repo", RepoIndexPhase::Failed)],
    });
    service.clear_in_memory_repo_runtime_for_test("failed/repo");

    ok_or_panic(
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let skipped = service.repo_search_publication_state("failed/repo").await;
                if skipped.availability == RepoSearchAvailability::Skipped {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await,
        "repo-corpus snapshot should hydrate",
    );

    assert_eq!(
        repo_phase(&service, "failed/repo"),
        Some(RepoIndexPhase::Failed)
    );
}

#[tokio::test]
async fn repo_search_publication_state_hydrates_from_repo_corpus_record_after_memory_miss() {
    let temp_dir = temp_dir();
    let keyspace = service_test_manifest_keyspace();
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
    publish_repo_bundle(&service, "searchable/repo", &documents, Some("rev-1")).await;
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
        repos: vec![repo_status_entry("searchable/repo", RepoIndexPhase::Ready)],
    });
    service.clear_in_memory_repo_runtime_for_test("searchable/repo");
    service.clear_in_memory_repo_publications_for_test("searchable/repo");
    service.clear_all_in_memory_repo_corpus_records_for_test();

    ok_or_panic(
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let searchable = service
                    .repo_search_publication_state("searchable/repo")
                    .await;
                if searchable.availability == RepoSearchAvailability::Searchable
                    && searchable.entity_published
                    && searchable.content_published
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await,
        "repo-corpus record cache should hydrate",
    );
}

#[tokio::test]
async fn repo_search_publication_state_does_not_hydrate_from_manifest_without_repo_corpus_cache() {
    let temp_dir = temp_dir();
    let keyspace = unique_test_manifest_keyspace("manifest-not-runtime");
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
    publish_repo_bundle(&service, "searchable/repo", &documents, Some("rev-1")).await;
    let ready_status = RepoIndexStatusResponse {
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
        repos: vec![repo_status_entry("searchable/repo", RepoIndexPhase::Ready)],
    };
    service.synchronize_repo_runtime(&ready_status);
    service
        .clear_persisted_repo_corpus_for_test("searchable/repo")
        .await;
    service.synchronize_repo_runtime(&ready_status);

    ok_or_panic(
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let state = service
                    .repo_search_publication_state("searchable/repo")
                    .await;
                if state.availability == RepoSearchAvailability::Pending {
                    assert!(!state.entity_published);
                    assert!(!state.content_published);
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await,
        "manifest-only fallback should stay disabled",
    );

    assert_eq!(
        repo_phase(&service, "searchable/repo"),
        Some(RepoIndexPhase::Ready)
    );
}

#[tokio::test]
async fn repo_publication_runs_prewarm_via_maintenance_task_and_releases_repo_slot() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );

    publish_repo_bundle(
        &service,
        "alpha/repo",
        &sample_repo_documents(),
        Some("rev-1"),
    )
    .await;

    let entity = some_or_panic(
        service
            .repo_corpus_record_for_reads(SearchCorpusKind::RepoEntity, "alpha/repo")
            .await,
        "repo entity record should exist",
    );
    let content = some_or_panic(
        service
            .repo_corpus_record_for_reads(SearchCorpusKind::RepoContentChunk, "alpha/repo")
            .await,
        "repo content record should exist",
    );
    assert!(
        entity
            .maintenance
            .as_ref()
            .and_then(|maintenance| maintenance.last_prewarmed_at.as_ref())
            .is_some()
    );
    assert!(
        !entity
            .maintenance
            .as_ref()
            .map(|maintenance| maintenance.prewarm_running)
            .unwrap_or(false)
    );
    assert!(
        content
            .maintenance
            .as_ref()
            .and_then(|maintenance| maintenance.last_prewarmed_at.as_ref())
            .is_some()
    );
    assert!(
        !content
            .maintenance
            .as_ref()
            .map(|maintenance| maintenance.prewarm_running)
            .unwrap_or(false)
    );
    assert!(
        service
            .repo_maintenance
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .in_flight
            .is_empty()
    );
}

#[tokio::test]
async fn repo_publication_does_not_skip_prewarm_when_slot_is_already_claimed() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy::default(),
    );
    let repo_id = "alpha/repo";
    let revision = Some("rev-1");
    let documents = sample_repo_documents();
    let file_fingerprints = documents
        .iter()
        .map(|document| {
            (
                document.path.clone(),
                document
                    .to_file_fingerprint(1, SearchCorpusKind::RepoContentChunk.schema_version()),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let table_name = versioned_repo_table_name(
        SearchPlaneService::repo_content_chunk_table_name(repo_id).as_str(),
        repo_id,
        &file_fingerprints,
        revision,
        SearchCorpusKind::RepoContentChunk,
        1,
    );
    service
        .repo_maintenance
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .in_flight
        .insert((
            SearchCorpusKind::RepoContentChunk,
            repo_id.to_string(),
            table_name,
            RepoMaintenanceTaskKind::Prewarm,
        ));

    ok_or_panic(
        service
            .publish_repo_content_chunks_with_revision(repo_id, &documents, revision)
            .await,
        "publish repo content chunks",
    );

    let content = some_or_panic(
        service
            .repo_corpus_record_for_reads(SearchCorpusKind::RepoContentChunk, repo_id)
            .await,
        "repo content record should exist",
    );
    assert!(
        content
            .maintenance
            .as_ref()
            .and_then(|maintenance| maintenance.last_prewarmed_at.as_ref())
            .is_some()
    );
}

#[tokio::test]
async fn repo_publication_schedules_compaction_and_resets_repo_maintenance() {
    let temp_dir = temp_dir();
    let service = SearchPlaneService::with_paths(
        PathBuf::from("/tmp/project"),
        temp_dir.path().join("search_plane"),
        service_test_manifest_keyspace(),
        SearchMaintenancePolicy {
            publish_count_threshold: 1,
            row_delta_ratio_threshold: 1.0,
        },
    );

    publish_repo_bundle(
        &service,
        "alpha/repo",
        &sample_repo_documents(),
        Some("rev-1"),
    )
    .await;

    ok_or_panic(
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let entity = some_or_panic(
                    service
                        .repo_corpus_record_for_reads(SearchCorpusKind::RepoEntity, "alpha/repo")
                        .await,
                    "repo entity record should exist",
                );
                let content = some_or_panic(
                    service
                        .repo_corpus_record_for_reads(
                            SearchCorpusKind::RepoContentChunk,
                            "alpha/repo",
                        )
                        .await,
                    "repo content record should exist",
                );
                let entity_done = entity
                    .maintenance
                    .as_ref()
                    .and_then(|maintenance| maintenance.last_compacted_at.as_ref())
                    .is_some();
                let content_done = content
                    .maintenance
                    .as_ref()
                    .and_then(|maintenance| maintenance.last_compacted_at.as_ref())
                    .is_some();
                if entity_done && content_done {
                    let entity_maintenance =
                        some_or_panic(entity.maintenance.as_ref(), "entity maintenance");
                    let content_maintenance =
                        some_or_panic(content.maintenance.as_ref(), "content maintenance");
                    assert!(!entity_maintenance.compaction_running);
                    assert!(!entity_maintenance.compaction_pending);
                    assert_eq!(entity_maintenance.publish_count_since_compaction, 0);
                    assert_eq!(
                        entity_maintenance.last_compaction_reason.as_deref(),
                        Some("publish_threshold")
                    );
                    assert_eq!(
                        entity_maintenance.last_compacted_row_count,
                        entity
                            .publication
                            .as_ref()
                            .map(|publication| publication.row_count)
                    );
                    assert!(!content_maintenance.compaction_running);
                    assert!(!content_maintenance.compaction_pending);
                    assert_eq!(content_maintenance.publish_count_since_compaction, 0);
                    assert_eq!(
                        content_maintenance.last_compaction_reason.as_deref(),
                        Some("publish_threshold")
                    );
                    assert_eq!(
                        content_maintenance.last_compacted_row_count,
                        content
                            .publication
                            .as_ref()
                            .map(|publication| publication.row_count)
                    );
                    break;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await,
        "repo compaction should complete",
    );
}
