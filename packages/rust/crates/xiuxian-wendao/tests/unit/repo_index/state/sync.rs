use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use crate::analyzers::{
    RegisteredRepository, RepoIntelligenceError, RepositoryAnalysisOutput, RepositoryPluginConfig,
    RepositoryRefreshPolicy,
};
use crate::repo_index::state::collect::await_analysis_completion;
use crate::repo_index::state::fingerprint::{fingerprint, timestamp_now};
use crate::repo_index::state::task::RepoIndexTaskPriority;
use crate::repo_index::state::tests::{init_test_repository, new_coordinator, remote_repo, repo};
use crate::repo_index::types::{RepoIndexEntryStatus, RepoIndexPhase};
use crate::search::{
    SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace, SearchPlaneService,
};
use uuid::Uuid;
use xiuxian_git_repo::{
    SyncMode, discover_checkout_metadata, record_managed_remote_probe_failure,
    record_managed_remote_probe_state,
};

use crate::analyzers::resolve_registered_repository_source;

#[test]
fn sync_repositories_only_enqueues_new_or_changed_repositories() {
    let coordinator = new_coordinator(SearchPlaneService::new(PathBuf::from(".")));

    let first = coordinator.sync_repositories(vec![repo("sciml", "./sciml")]);
    let second = coordinator.sync_repositories(vec![repo("sciml", "./sciml")]);
    let third = coordinator.sync_repositories(vec![repo("sciml", "./sciml-next")]);

    assert_eq!(first, vec!["sciml".to_string()]);
    assert!(second.is_empty());
    assert_eq!(third, vec!["sciml".to_string()]);
}

#[test]
fn sync_repositories_skips_repositories_with_only_search_only_plugins() {
    let coordinator = new_coordinator(SearchPlaneService::new(PathBuf::from(".")));
    let repository = RegisteredRepository {
        id: "sciml".to_string(),
        path: Some(PathBuf::from("./sciml")),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Id("ast-grep".to_string())],
    };

    let enqueued = coordinator.sync_repositories(vec![repository]);

    assert!(enqueued.is_empty());
    assert!(coordinator.pending_repo_ids_for_test().is_empty());
    assert_eq!(coordinator.status_response(None).total, 0);
}

#[test]
fn sync_repositories_reenqueues_repositories_when_configured_plugin_set_changes() {
    let coordinator = new_coordinator(SearchPlaneService::new(PathBuf::from(".")));
    let repository = repo("sciml", "./sciml");
    let repository_with_ast_grep = RegisteredRepository {
        plugins: vec![
            RepositoryPluginConfig::Id("julia".to_string()),
            RepositoryPluginConfig::Id("ast-grep".to_string()),
        ],
        ..repository.clone()
    };

    let first = coordinator.sync_repositories(vec![repository]);
    let second = coordinator.sync_repositories(vec![repository_with_ast_grep]);

    assert_eq!(first, vec!["sciml".to_string()]);
    assert_eq!(second, vec!["sciml".to_string()]);
    assert_eq!(
        coordinator.pending_repo_ids_for_test(),
        vec!["sciml".to_string()]
    );
}

#[test]
fn sync_repositories_does_not_reenqueue_repositories_when_configured_plugin_order_only_changes() {
    let coordinator = new_coordinator(SearchPlaneService::new(PathBuf::from(".")));
    let repository = RegisteredRepository {
        plugins: vec![
            RepositoryPluginConfig::Id("julia".to_string()),
            RepositoryPluginConfig::Id("ast-grep".to_string()),
            RepositoryPluginConfig::Config {
                id: "modelica".to_string(),
                options: serde_json::json!({
                    "mode": "parser-summary"
                }),
            },
        ],
        ..repo("sciml", "./sciml")
    };
    let reordered_repository = RegisteredRepository {
        plugins: vec![
            RepositoryPluginConfig::Config {
                id: "modelica".to_string(),
                options: serde_json::json!({
                    "mode": "doc-surface"
                }),
            },
            RepositoryPluginConfig::Id("julia".to_string()),
            RepositoryPluginConfig::Id("ast-grep".to_string()),
            RepositoryPluginConfig::Id("ast-grep".to_string()),
        ],
        ..repository.clone()
    };

    let first = coordinator.sync_repositories(vec![repository]);
    let second = coordinator.sync_repositories(vec![reordered_repository]);

    assert_eq!(first, vec!["sciml".to_string()]);
    assert!(second.is_empty());
}

#[tokio::test]
async fn sync_repositories_warm_starts_local_checkout_from_persisted_repo_publications() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let storage_root = temp_dir.path().join("search-plane");
    let manifest_keyspace = SearchManifestKeyspace::new("xiuxian:test:repo-warm-start-local");
    let initial_search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        storage_root.clone(),
        manifest_keyspace.clone(),
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![crate::repo_index::types::RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: std::sync::Arc::<str>::from("fn alpha() {}\n"),
        size_bytes: 14,
        modified_unix_ms: 0,
    }];
    initial_search_plane
        .publish_repo_entities_with_revision(
            "local-repo",
            &RepositoryAnalysisOutput {
                modules: vec![crate::analyzers::ModuleRecord {
                    repo_id: "local-repo".to_string(),
                    module_id: "module:alpha".to_string(),
                    qualified_name: "Alpha".to_string(),
                    path: "src/lib.rs".to_string(),
                }],
                ..RepositoryAnalysisOutput::default()
            },
            &documents,
            Some("rev-1"),
        )
        .await
        .unwrap_or_else(|error| panic!("publish repo entities: {error}"));
    initial_search_plane
        .publish_repo_content_chunks_with_revision("local-repo", &documents, Some("rev-1"))
        .await
        .unwrap_or_else(|error| panic!("publish repo content chunks: {error}"));

    let search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        storage_root,
        manifest_keyspace,
        SearchMaintenancePolicy::default(),
    );
    let coordinator = new_coordinator(search_plane);

    let enqueued = coordinator.sync_repositories(vec![repo("local-repo", "./local-repo")]);

    assert!(enqueued.is_empty());
    assert!(coordinator.pending_repo_ids_for_test().is_empty());

    let status = coordinator.status_response(Some("local-repo"));
    assert_eq!(status.total, 1);
    assert_eq!(status.ready, 1);
    assert_eq!(status.repos[0].phase, RepoIndexPhase::Ready);
    assert_eq!(status.repos[0].last_revision.as_deref(), Some("rev-1"));
}

#[tokio::test]
async fn managed_remote_with_missing_assets_warm_starts_from_readable_publications() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let storage_root = temp_dir.path().join("search-plane");
    let manifest_keyspace = SearchManifestKeyspace::new("xiuxian:test:repo-warm-start-remote");
    let initial_search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        storage_root.clone(),
        manifest_keyspace.clone(),
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![crate::repo_index::types::RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: std::sync::Arc::<str>::from("fn alpha() {}\n"),
        size_bytes: 14,
        modified_unix_ms: 0,
    }];
    initial_search_plane
        .publish_repo_entities_with_revision(
            "managed-remote",
            &RepositoryAnalysisOutput {
                modules: vec![crate::analyzers::ModuleRecord {
                    repo_id: "managed-remote".to_string(),
                    module_id: "module:alpha".to_string(),
                    qualified_name: "Alpha".to_string(),
                    path: "src/lib.rs".to_string(),
                }],
                ..RepositoryAnalysisOutput::default()
            },
            &documents,
            Some("rev-1"),
        )
        .await
        .unwrap_or_else(|error| panic!("publish repo entities: {error}"));
    initial_search_plane
        .publish_repo_content_chunks_with_revision("managed-remote", &documents, Some("rev-1"))
        .await
        .unwrap_or_else(|error| panic!("publish repo content chunks: {error}"));

    let search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        storage_root,
        manifest_keyspace,
        SearchMaintenancePolicy::default(),
    );
    let coordinator = new_coordinator(search_plane);

    let enqueued = coordinator.sync_repositories(vec![remote_repo(
        "managed-remote",
        "https://example.com/managed-remote.git",
    )]);

    assert!(enqueued.is_empty());
    assert!(coordinator.pending_repo_ids_for_test().is_empty());

    let status = coordinator.status_response(Some("managed-remote"));
    assert_eq!(status.total, 1);
    assert_eq!(status.ready, 1);
    assert_eq!(status.repos[0].phase, RepoIndexPhase::Ready);
    assert_eq!(status.repos[0].last_revision.as_deref(), Some("rev-1"));
}

#[tokio::test]
async fn sync_repositories_warm_starts_from_valkey_repo_publications_after_memory_and_local_snapshot_miss()
 {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let storage_root = temp_dir.path().join("search-plane");
    let manifest_keyspace =
        SearchManifestKeyspace::new("xiuxian:test:repo-warm-start-local-cache-only");
    let search_plane = SearchPlaneService::with_test_cache(
        PathBuf::from("."),
        storage_root,
        manifest_keyspace,
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![crate::repo_index::types::RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: std::sync::Arc::<str>::from("fn alpha() {}\n"),
        size_bytes: 14,
        modified_unix_ms: 0,
    }];
    search_plane
        .publish_repo_entities_with_revision(
            "cache-only-local-repo",
            &RepositoryAnalysisOutput {
                modules: vec![crate::analyzers::ModuleRecord {
                    repo_id: "cache-only-local-repo".to_string(),
                    module_id: "module:alpha".to_string(),
                    qualified_name: "Alpha".to_string(),
                    path: "src/lib.rs".to_string(),
                }],
                ..RepositoryAnalysisOutput::default()
            },
            &documents,
            Some("rev-cache"),
        )
        .await
        .unwrap_or_else(|error| panic!("publish repo entities: {error}"));
    search_plane
        .publish_repo_content_chunks_with_revision(
            "cache-only-local-repo",
            &documents,
            Some("rev-cache"),
        )
        .await
        .unwrap_or_else(|error| panic!("publish repo content chunks: {error}"));

    search_plane.clear_all_in_memory_repo_corpus_records_for_test();
    for corpus in [
        SearchCorpusKind::RepoEntity,
        SearchCorpusKind::RepoContentChunk,
    ] {
        fs::remove_file(search_plane.repo_corpus_record_json_path(corpus, "cache-only-local-repo"))
            .ok();
    }
    fs::remove_file(search_plane.repo_corpus_snapshot_json_path()).ok();

    let coordinator = new_coordinator(search_plane);

    let enqueued = coordinator.sync_repositories(vec![repo(
        "cache-only-local-repo",
        "./cache-only-local-repo",
    )]);

    assert!(enqueued.is_empty());
    assert!(coordinator.pending_repo_ids_for_test().is_empty());

    let status = coordinator.status_response(Some("cache-only-local-repo"));
    assert_eq!(status.total, 1);
    assert_eq!(status.ready, 1);
    assert_eq!(status.repos[0].phase, RepoIndexPhase::Ready);
    assert_eq!(status.repos[0].last_revision.as_deref(), Some("rev-cache"));
}

#[tokio::test]
async fn sync_repositories_warm_starts_stale_fetch_policy_remote_when_recent_probe_matches() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let source_repo = temp_dir.path().join("managed-remote-source");
    fs::create_dir_all(&source_repo).unwrap_or_else(|error| panic!("create source repo: {error}"));
    init_test_repository(&source_repo);

    let repo_id = format!("managed-remote-probe-{}", Uuid::new_v4());
    let repository = remote_repo(&repo_id, source_repo.display().to_string().as_str());
    let first =
        resolve_registered_repository_source(&repository, temp_dir.path(), SyncMode::Ensure)
            .unwrap_or_else(|error| panic!("resolve first ensure: {error}"));
    let second =
        resolve_registered_repository_source(&repository, temp_dir.path(), SyncMode::Ensure)
            .unwrap_or_else(|error| panic!("resolve second ensure: {error}"));
    let revision = discover_checkout_metadata(&second.checkout_root)
        .unwrap_or_else(|| panic!("discover checkout metadata for `{repo_id}`"))
        .revision
        .unwrap_or_else(|| panic!("missing revision for `{repo_id}`"));

    set_mirror_fetch_age(
        second
            .mirror_root
            .as_deref()
            .unwrap_or_else(|| panic!("missing mirror root for `{repo_id}`")),
        Duration::from_secs(3 * 24 * 3600),
    );

    let storage_root = temp_dir.path().join("search-plane");
    let manifest_keyspace = SearchManifestKeyspace::new(format!(
        "xiuxian:test:repo-warm-start-managed-probe-{repo_id}"
    ));
    let initial_search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        storage_root.clone(),
        manifest_keyspace.clone(),
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![crate::repo_index::types::RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: std::sync::Arc::<str>::from("fn alpha() {}\n"),
        size_bytes: 14,
        modified_unix_ms: 0,
    }];
    initial_search_plane
        .publish_repo_entities_with_revision(
            repo_id.as_str(),
            &RepositoryAnalysisOutput {
                modules: vec![crate::analyzers::ModuleRecord {
                    repo_id: repo_id.clone(),
                    module_id: "module:alpha".to_string(),
                    qualified_name: "Alpha".to_string(),
                    path: "src/lib.rs".to_string(),
                }],
                ..RepositoryAnalysisOutput::default()
            },
            &documents,
            Some(revision.as_str()),
        )
        .await
        .unwrap_or_else(|error| panic!("publish repo entities: {error}"));
    initial_search_plane
        .publish_repo_content_chunks_with_revision(
            repo_id.as_str(),
            &documents,
            Some(revision.as_str()),
        )
        .await
        .unwrap_or_else(|error| panic!("publish repo content chunks: {error}"));

    let search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        storage_root,
        manifest_keyspace,
        SearchMaintenancePolicy::default(),
    );
    let coordinator = new_coordinator(search_plane);

    let enqueued = coordinator.sync_repositories(vec![repository]);

    assert!(enqueued.is_empty());
    assert!(coordinator.pending_repo_ids_for_test().is_empty());

    let status = coordinator.status_response(Some(repo_id.as_str()));
    assert_eq!(status.total, 1);
    assert_eq!(status.ready, 1);
    assert_eq!(status.repos[0].phase, RepoIndexPhase::Ready);
    assert_eq!(
        status.repos[0].last_revision.as_deref(),
        Some(revision.as_str())
    );

    let Some(mirror_root) = second.mirror_root.as_ref() else {
        panic!("mirror root");
    };
    fs::remove_dir_all(mirror_root)
        .unwrap_or_else(|error| panic!("cleanup managed mirror: {error}"));
    fs::remove_dir_all(&second.checkout_root)
        .unwrap_or_else(|error| panic!("cleanup managed checkout: {error}"));
    fs::remove_dir_all(first.checkout_root).ok();
}

#[tokio::test]
async fn sync_repositories_warm_starts_stale_fetch_policy_remote_when_recent_retryable_probe_failure_exists()
 {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let source_repo = temp_dir.path().join("managed-remote-source");
    fs::create_dir_all(&source_repo).unwrap_or_else(|error| panic!("create source repo: {error}"));
    init_test_repository(&source_repo);

    let repo_id = format!("managed-remote-probe-failure-{}", Uuid::new_v4());
    let repository = remote_repo(&repo_id, source_repo.display().to_string().as_str());
    let first =
        resolve_registered_repository_source(&repository, temp_dir.path(), SyncMode::Ensure)
            .unwrap_or_else(|error| panic!("resolve first ensure: {error}"));
    let second =
        resolve_registered_repository_source(&repository, temp_dir.path(), SyncMode::Ensure)
            .unwrap_or_else(|error| panic!("resolve second ensure: {error}"));
    let mirror_root = second
        .mirror_root
        .as_deref()
        .unwrap_or_else(|| panic!("missing mirror root for `{repo_id}`"));
    let revision = discover_checkout_metadata(&second.checkout_root)
        .unwrap_or_else(|| panic!("discover checkout metadata for `{repo_id}`"))
        .revision
        .unwrap_or_else(|| panic!("missing revision for `{repo_id}`"));

    set_mirror_fetch_age(mirror_root, Duration::from_secs(3 * 24 * 3600));
    record_managed_remote_probe_failure(mirror_root, "operation timed out", true)
        .unwrap_or_else(|error| panic!("record retryable probe failure: {error}"));

    let storage_root = temp_dir.path().join("search-plane");
    let manifest_keyspace = SearchManifestKeyspace::new(format!(
        "xiuxian:test:repo-warm-start-managed-probe-failure-{repo_id}"
    ));
    let initial_search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        storage_root.clone(),
        manifest_keyspace.clone(),
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![crate::repo_index::types::RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: std::sync::Arc::<str>::from("fn alpha() {}\n"),
        size_bytes: 14,
        modified_unix_ms: 0,
    }];
    initial_search_plane
        .publish_repo_entities_with_revision(
            repo_id.as_str(),
            &RepositoryAnalysisOutput {
                modules: vec![crate::analyzers::ModuleRecord {
                    repo_id: repo_id.clone(),
                    module_id: "module:alpha".to_string(),
                    qualified_name: "Alpha".to_string(),
                    path: "src/lib.rs".to_string(),
                }],
                ..RepositoryAnalysisOutput::default()
            },
            &documents,
            Some(revision.as_str()),
        )
        .await
        .unwrap_or_else(|error| panic!("publish repo entities: {error}"));
    initial_search_plane
        .publish_repo_content_chunks_with_revision(
            repo_id.as_str(),
            &documents,
            Some(revision.as_str()),
        )
        .await
        .unwrap_or_else(|error| panic!("publish repo content chunks: {error}"));

    let search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        storage_root,
        manifest_keyspace,
        SearchMaintenancePolicy::default(),
    );
    let coordinator = new_coordinator(search_plane);

    let enqueued = coordinator.sync_repositories(vec![repository]);

    assert!(enqueued.is_empty());
    assert!(coordinator.pending_repo_ids_for_test().is_empty());

    let status = coordinator.status_response(Some(repo_id.as_str()));
    assert_eq!(status.total, 1);
    assert_eq!(status.ready, 1);
    assert_eq!(status.repos[0].phase, RepoIndexPhase::Ready);
    assert_eq!(
        status.repos[0].last_revision.as_deref(),
        Some(revision.as_str())
    );

    let Some(mirror_root) = second.mirror_root.as_ref() else {
        panic!("mirror root");
    };
    fs::remove_dir_all(mirror_root)
        .unwrap_or_else(|error| panic!("cleanup managed mirror: {error}"));
    fs::remove_dir_all(&second.checkout_root)
        .unwrap_or_else(|error| panic!("cleanup managed checkout: {error}"));
    fs::remove_dir_all(first.checkout_root).ok();
}

#[tokio::test]
async fn sync_repositories_warm_starts_stale_fetch_policy_remote_when_retryable_probe_failure_preserves_aging_success_proof()
 {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let source_repo = temp_dir.path().join("managed-remote-source");
    fs::create_dir_all(&source_repo).unwrap_or_else(|error| panic!("create source repo: {error}"));
    init_test_repository(&source_repo);

    let repo_id = format!("managed-remote-probe-history-{}", Uuid::new_v4());
    let repository = remote_repo(&repo_id, source_repo.display().to_string().as_str());
    let first =
        resolve_registered_repository_source(&repository, temp_dir.path(), SyncMode::Ensure)
            .unwrap_or_else(|error| panic!("resolve first ensure: {error}"));
    let second =
        resolve_registered_repository_source(&repository, temp_dir.path(), SyncMode::Ensure)
            .unwrap_or_else(|error| panic!("resolve second ensure: {error}"));
    let mirror_root = second
        .mirror_root
        .as_deref()
        .unwrap_or_else(|| panic!("missing mirror root for `{repo_id}`"));
    let revision = discover_checkout_metadata(&second.checkout_root)
        .unwrap_or_else(|| panic!("discover checkout metadata for `{repo_id}`"))
        .revision
        .unwrap_or_else(|| panic!("missing revision for `{repo_id}`"));

    set_mirror_fetch_age(mirror_root, Duration::from_secs(3 * 24 * 3600));
    record_managed_remote_probe_state(mirror_root, Some(revision.as_str()))
        .unwrap_or_else(|error| panic!("record probe success: {error}"));
    record_managed_remote_probe_failure(mirror_root, "operation timed out", true)
        .unwrap_or_else(|error| panic!("record retryable probe failure: {error}"));
    set_managed_remote_probe_state_age(
        mirror_root,
        Duration::from_secs(2 * 3600),
        Some(Duration::from_secs(2 * 3600)),
    );

    let storage_root = temp_dir.path().join("search-plane");
    let manifest_keyspace = SearchManifestKeyspace::new(format!(
        "xiuxian:test:repo-warm-start-managed-probe-history-{repo_id}"
    ));
    let initial_search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        storage_root.clone(),
        manifest_keyspace.clone(),
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![crate::repo_index::types::RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: std::sync::Arc::<str>::from("fn alpha() {}\n"),
        size_bytes: 14,
        modified_unix_ms: 0,
    }];
    initial_search_plane
        .publish_repo_entities_with_revision(
            repo_id.as_str(),
            &RepositoryAnalysisOutput {
                modules: vec![crate::analyzers::ModuleRecord {
                    repo_id: repo_id.clone(),
                    module_id: "module:alpha".to_string(),
                    qualified_name: "Alpha".to_string(),
                    path: "src/lib.rs".to_string(),
                }],
                ..RepositoryAnalysisOutput::default()
            },
            &documents,
            Some(revision.as_str()),
        )
        .await
        .unwrap_or_else(|error| panic!("publish repo entities: {error}"));
    initial_search_plane
        .publish_repo_content_chunks_with_revision(
            repo_id.as_str(),
            &documents,
            Some(revision.as_str()),
        )
        .await
        .unwrap_or_else(|error| panic!("publish repo content chunks: {error}"));

    let search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        storage_root,
        manifest_keyspace,
        SearchMaintenancePolicy::default(),
    );
    let coordinator = new_coordinator(search_plane);

    let enqueued = coordinator.sync_repositories(vec![repository]);

    assert!(enqueued.is_empty());
    assert!(coordinator.pending_repo_ids_for_test().is_empty());

    let status = coordinator.status_response(Some(repo_id.as_str()));
    assert_eq!(status.total, 1);
    assert_eq!(status.ready, 1);
    assert_eq!(status.repos[0].phase, RepoIndexPhase::Ready);
    assert_eq!(
        status.repos[0].last_revision.as_deref(),
        Some(revision.as_str())
    );

    let Some(mirror_root) = second.mirror_root.as_ref() else {
        panic!("mirror root");
    };
    fs::remove_dir_all(mirror_root)
        .unwrap_or_else(|error| panic!("cleanup managed mirror: {error}"));
    fs::remove_dir_all(&second.checkout_root)
        .unwrap_or_else(|error| panic!("cleanup managed checkout: {error}"));
    fs::remove_dir_all(first.checkout_root).ok();
}

#[test]
fn record_repo_status_advances_attempt_count_without_lock_reentrancy() {
    let coordinator = new_coordinator(SearchPlaneService::new(PathBuf::from(".")));
    coordinator.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "ADTypes.jl".to_string(),
        phase: RepoIndexPhase::Indexing,
        queue_position: None,
        last_error: None,
        last_revision: Some("abc123".to_string()),
        updated_at: Some(timestamp_now()),
        attempt_count: 2,
    });

    coordinator.record_repo_status(
        "ADTypes.jl",
        RepoIndexPhase::Ready,
        Some("abc123".to_string()),
        None,
    );

    let status = coordinator.status_response(Some("ADTypes.jl"));
    assert_eq!(status.ready, 1);
    assert_eq!(status.repos.first().map(|item| item.attempt_count), Some(3));
}

#[test]
fn interactive_enqueue_promotes_pending_repository_to_front() {
    let coordinator = new_coordinator(SearchPlaneService::new(PathBuf::from(".")));
    let first_repo = repo("ADTypes.jl", "./ADTypes.jl");
    let second_repo = repo("DifferentialEquations.jl", "./DifferentialEquations.jl");
    let first_fingerprint = fingerprint(&first_repo);
    let second_fingerprint = fingerprint(&second_repo);

    assert!(coordinator.enqueue_repository(
        first_repo,
        false,
        true,
        first_fingerprint,
        RepoIndexTaskPriority::Background,
    ));
    assert!(coordinator.enqueue_repository(
        second_repo.clone(),
        false,
        true,
        second_fingerprint.clone(),
        RepoIndexTaskPriority::Background,
    ));
    assert!(coordinator.enqueue_repository(
        second_repo,
        false,
        false,
        second_fingerprint,
        RepoIndexTaskPriority::Interactive,
    ));

    let pending = coordinator.pending_repo_ids_for_test();
    assert_eq!(
        pending,
        vec![
            "DifferentialEquations.jl".to_string(),
            "ADTypes.jl".to_string()
        ]
    );

    let status = coordinator.status_response(None);
    assert_eq!(
        status
            .repos
            .iter()
            .find(|repo| repo.repo_id == "DifferentialEquations.jl")
            .and_then(|repo| repo.queue_position),
        Some(1)
    );
    assert_eq!(
        status
            .repos
            .iter()
            .find(|repo| repo.repo_id == "ADTypes.jl")
            .and_then(|repo| repo.queue_position),
        Some(2)
    );
}

#[tokio::test]
async fn await_analysis_completion_returns_timeout_error_for_stuck_analysis() {
    let task = tokio::task::spawn_blocking(|| {
        std::thread::sleep(Duration::from_millis(25));
        Ok(RepositoryAnalysisOutput::default())
    });

    let Err(error) = await_analysis_completion("stuck", task, Duration::from_millis(1)).await
    else {
        panic!("slow analysis should time out");
    };

    match error {
        RepoIntelligenceError::AnalysisFailed { message } => {
            assert!(message.contains("repo `stuck` indexing timed out"));
        }
        other => panic!("expected analysis timeout failure, got {other:?}"),
    }
}

fn set_mirror_fetch_age(mirror_root: &std::path::Path, age: Duration) {
    let target_time = std::time::SystemTime::now()
        .checked_sub(age)
        .unwrap_or_else(|| panic!("failed to compute mirror age timestamp"));

    for candidate in [mirror_root.join("FETCH_HEAD"), mirror_root.join("HEAD")] {
        if candidate.exists() {
            let file = fs::OpenOptions::new()
                .write(true)
                .open(&candidate)
                .unwrap_or_else(|error| panic!("open `{}`: {error}", candidate.display()));
            let times = fs::FileTimes::new().set_modified(target_time);
            file.set_times(times)
                .unwrap_or_else(|error| panic!("set times for `{}`: {error}", candidate.display()));
        }
    }
}

fn set_managed_remote_probe_state_age(
    mirror_root: &std::path::Path,
    probe_age: Duration,
    last_success_age: Option<Duration>,
) {
    let state_path = mirror_root.join("xiuxian-upstream-probe-state.json");
    let mut payload: serde_json::Value = serde_json::from_slice(
        &fs::read(&state_path)
            .unwrap_or_else(|error| panic!("read `{}`: {error}", state_path.display())),
    )
    .unwrap_or_else(|error| panic!("parse `{}`: {error}", state_path.display()));
    payload["checked_at"] = serde_json::Value::String(
        chrono::DateTime::<chrono::Utc>::from(
            std::time::SystemTime::now()
                .checked_sub(probe_age)
                .unwrap_or_else(|| panic!("failed to compute probe timestamp")),
        )
        .to_rfc3339(),
    );
    match last_success_age {
        Some(age) => {
            payload["last_success_checked_at"] = serde_json::Value::String(
                chrono::DateTime::<chrono::Utc>::from(
                    std::time::SystemTime::now()
                        .checked_sub(age)
                        .unwrap_or_else(|| panic!("failed to compute success timestamp")),
                )
                .to_rfc3339(),
            );
        }
        None => {
            payload
                .as_object_mut()
                .unwrap_or_else(|| panic!("probe payload should be an object"))
                .remove("last_success_checked_at");
        }
    }
    fs::write(
        &state_path,
        serde_json::to_vec(&payload)
            .unwrap_or_else(|error| panic!("encode `{}`: {error}", state_path.display())),
    )
    .unwrap_or_else(|error| panic!("write `{}`: {error}", state_path.display()));
}
