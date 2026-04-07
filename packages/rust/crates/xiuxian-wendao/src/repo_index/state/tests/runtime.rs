use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use crate::analyzers::query::{RepoSourceKind, RepoSyncResult};
use crate::analyzers::{
    RegisteredRepository, RepositoryAnalysisOutput, RepositoryPluginConfig,
    RepositoryRefreshPolicy, analyze_registered_repository_with_registry,
    bootstrap_builtin_registry,
};
use crate::gateway::studio::test_support::{commit_all, init_git_repository};
use crate::repo_index::state::coordinator::PreparedIncrementalAnalysis;
use crate::repo_index::state::fingerprint::timestamp_now;
use crate::repo_index::state::tests::{new_coordinator, new_coordinator_with_registry};
use crate::repo_index::types::RepoIndexEntryStatus;
use crate::search::{
    RepoSearchAvailability, SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace,
    SearchPlaneCache, SearchPlanePhase, SearchPlaneService, SearchPublicationStorageFormat,
    SearchRepoPublicationInput,
};
use chrono::Utc;

#[tokio::test]
async fn refresh_status_snapshot_synchronizes_search_plane_runtime() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        temp_dir.path().join("search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-runtime-sync"),
        SearchMaintenancePolicy::default(),
    );
    let coordinator = new_coordinator(search_plane.clone());
    coordinator.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "pending".to_string(),
        phase: crate::repo_index::types::RepoIndexPhase::Queued,
        queue_position: None,
        last_error: None,
        last_revision: None,
        updated_at: Some(timestamp_now()),
        attempt_count: 1,
    });
    coordinator.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "skipped".to_string(),
        phase: crate::repo_index::types::RepoIndexPhase::Failed,
        queue_position: None,
        last_error: Some("boom".to_string()),
        last_revision: None,
        updated_at: Some(timestamp_now()),
        attempt_count: 1,
    });

    let pending = search_plane.repo_search_publication_state("pending").await;
    let skipped = search_plane.repo_search_publication_state("skipped").await;

    assert_eq!(pending.availability, RepoSearchAvailability::Pending);
    assert_eq!(skipped.availability, RepoSearchAvailability::Skipped);
}

#[tokio::test]
async fn stop_releases_background_runner_arc() {
    let coordinator = Arc::new(new_coordinator(SearchPlaneService::new(PathBuf::from("."))));
    let weak = Arc::downgrade(&coordinator);

    coordinator.start();
    tokio::task::yield_now().await;
    coordinator.stop();
    drop(coordinator);

    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if weak.upgrade().is_none() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap_or_else(|error| panic!("runner arc should be released after stop: {error}"));
}

#[tokio::test]
async fn refresh_status_snapshot_synchronizes_repo_backed_corpus_statuses() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        temp_dir.path().join("search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-status-sync"),
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![crate::repo_index::types::RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: Arc::<str>::from("fn alpha() {}\n"),
        size_bytes: 14,
        modified_unix_ms: 0,
    }];
    search_plane
        .publish_repo_entities_with_revision(
            "alpha/repo",
            &RepositoryAnalysisOutput {
                modules: vec![crate::analyzers::ModuleRecord {
                    repo_id: "alpha/repo".to_string(),
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
    search_plane
        .publish_repo_content_chunks_with_revision("alpha/repo", &documents, Some("rev-1"))
        .await
        .unwrap_or_else(|error| panic!("publish repo content chunks: {error}"));
    let coordinator = new_coordinator(search_plane.clone());
    coordinator.set_status_for_test(RepoIndexEntryStatus {
        repo_id: "alpha/repo".to_string(),
        phase: crate::repo_index::types::RepoIndexPhase::Ready,
        queue_position: None,
        last_error: None,
        last_revision: Some("rev-1".to_string()),
        updated_at: Some(timestamp_now()),
        attempt_count: 1,
    });

    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let snapshot = search_plane.status();
            let Some(repo_entity) = snapshot
                .corpora
                .iter()
                .find(|entry| entry.corpus == SearchCorpusKind::RepoEntity)
            else {
                panic!("repo entity row");
            };
            let Some(repo_content) = snapshot
                .corpora
                .iter()
                .find(|entry| entry.corpus == SearchCorpusKind::RepoContentChunk)
            else {
                panic!("repo content row");
            };
            if repo_entity.phase == SearchPlanePhase::Ready
                && repo_content.phase == SearchPlanePhase::Ready
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap_or_else(|error| panic!("repo-backed corpus status should synchronize: {error}"));
}

#[tokio::test]
async fn managed_remote_skips_reindex_when_repo_publications_already_match_revision() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        temp_dir.path().join("search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-current-publications"),
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![crate::repo_index::types::RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: Arc::<str>::from("fn alpha() {}\n"),
        size_bytes: 14,
        modified_unix_ms: 0,
    }];
    search_plane
        .publish_repo_entities_with_revision(
            "alpha/repo",
            &RepositoryAnalysisOutput {
                modules: vec![crate::analyzers::ModuleRecord {
                    repo_id: "alpha/repo".to_string(),
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
    search_plane
        .publish_repo_content_chunks_with_revision("alpha/repo", &documents, Some("rev-1"))
        .await
        .unwrap_or_else(|error| panic!("publish repo content chunks: {error}"));
    let coordinator = new_coordinator(search_plane);

    assert!(
        coordinator
            .repo_publications_are_current(
                "alpha/repo",
                &RepoSyncResult {
                    repo_id: "alpha/repo".to_string(),
                    source_kind: RepoSourceKind::ManagedRemote,
                    revision: Some("rev-1".to_string()),
                    ..RepoSyncResult::default()
                },
            )
            .await
    );
}

#[tokio::test]
async fn managed_remote_reuses_latest_persisted_publications_without_revision_cache() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let keyspace = SearchManifestKeyspace::new("xiuxian:test:repo-latest-publication-fast-path");
    let cache = SearchPlaneCache::for_tests(keyspace.clone());
    let search_plane = SearchPlaneService::with_runtime(
        PathBuf::from("."),
        temp_dir.path().join("search-plane"),
        keyspace,
        SearchMaintenancePolicy::default(),
        cache.clone(),
    );

    for corpus in [
        SearchCorpusKind::RepoEntity,
        SearchCorpusKind::RepoContentChunk,
    ] {
        search_plane
            .record_repo_publication_input_with_storage_format(
                corpus,
                "alpha/repo",
                SearchRepoPublicationInput {
                    table_name: format!("{corpus}_alpha_repo_rev_1"),
                    schema_version: corpus.schema_version(),
                    source_revision: Some("rev-1".to_string()),
                    table_version_id: 1,
                    row_count: 1,
                    fragment_count: 1,
                    published_at: "2026-04-06T00:00:01Z".to_string(),
                },
                SearchPublicationStorageFormat::Parquet,
            )
            .await;
        cache
            .delete_repo_publication_revision_cache(corpus, "alpha/repo")
            .await;
    }

    search_plane.clear_all_in_memory_repo_corpus_records_for_test();
    let coordinator = new_coordinator(search_plane);

    assert!(
        coordinator
            .repo_publications_are_current(
                "alpha/repo",
                &RepoSyncResult {
                    repo_id: "alpha/repo".to_string(),
                    source_kind: RepoSourceKind::ManagedRemote,
                    revision: Some("rev-1".to_string()),
                    ..RepoSyncResult::default()
                },
            )
            .await
    );
}

#[tokio::test]
async fn prepare_incremental_analysis_returns_refresh_only_for_non_code_revision_churn() {
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(tempdir.path());
    fs::create_dir_all(tempdir.path().join("src"))
        .unwrap_or_else(|error| panic!("create src: {error}"));
    fs::write(
        tempdir.path().join("Project.toml"),
        "name = \"FixturePkg\"\n",
    )
    .unwrap_or_else(|error| panic!("write Project.toml: {error}"));
    fs::write(
        tempdir.path().join("src/FixturePkg.jl"),
        "module FixturePkg\nend\n",
    )
    .unwrap_or_else(|error| panic!("write Julia source: {error}"));
    fs::write(tempdir.path().join("notes.txt"), "first note\n")
        .unwrap_or_else(|error| panic!("write notes: {error}"));
    commit_all(tempdir.path(), "initial");
    let previous_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover previous revision"));

    fs::write(tempdir.path().join("notes.txt"), "second note\n")
        .unwrap_or_else(|error| panic!("rewrite notes: {error}"));
    commit_all(tempdir.path(), "non-code");
    let current_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover current revision"));

    let repository = RegisteredRepository {
        id: "incremental-refresh-only".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
    };
    let registry = Arc::new(
        bootstrap_builtin_registry().unwrap_or_else(|error| panic!("bootstrap registry: {error}")),
    );
    let coordinator =
        new_coordinator_with_registry(SearchPlaneService::new(PathBuf::from(".")), registry);

    let prepared = coordinator
        .prepare_incremental_analysis(
            &repository,
            &RepoSyncResult {
                repo_id: repository.id.clone(),
                source_kind: RepoSourceKind::LocalCheckout,
                checkout_path: tempdir.path().display().to_string(),
                revision: Some(current_revision),
                ..RepoSyncResult::default()
            },
            Some(previous_revision.as_str()),
        )
        .unwrap_or_else(|error| panic!("prepare incremental refresh-only: {error}"));

    assert!(matches!(
        prepared,
        Some(PreparedIncrementalAnalysis::RefreshOnly)
    ));
}

#[tokio::test]
async fn prepare_incremental_analysis_merges_leaf_julia_source_changes() {
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(tempdir.path());
    fs::create_dir_all(tempdir.path().join("src"))
        .unwrap_or_else(|error| panic!("create src: {error}"));
    fs::write(
        tempdir.path().join("Project.toml"),
        "name = \"FixturePkg\"\n",
    )
    .unwrap_or_else(|error| panic!("write Project.toml: {error}"));
    fs::write(
        tempdir.path().join("src/FixturePkg.jl"),
        "module FixturePkg\ninclude(\"leaf.jl\")\nend\n",
    )
    .unwrap_or_else(|error| panic!("write root Julia source: {error}"));
    fs::write(tempdir.path().join("src/leaf.jl"), "alpha() = 1\n")
        .unwrap_or_else(|error| panic!("write leaf Julia source: {error}"));
    commit_all(tempdir.path(), "initial");
    let previous_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover previous revision"));

    let repository = RegisteredRepository {
        id: "incremental-leaf-merge".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
    };
    let registry = Arc::new(
        bootstrap_builtin_registry().unwrap_or_else(|error| panic!("bootstrap registry: {error}")),
    );
    let coordinator = new_coordinator_with_registry(
        SearchPlaneService::new(PathBuf::from(".")),
        Arc::clone(&registry),
    );
    analyze_registered_repository_with_registry(&repository, tempdir.path(), registry.as_ref())
        .unwrap_or_else(|error| panic!("seed analysis cache: {error}"));

    fs::write(
        tempdir.path().join("src/leaf.jl"),
        "alpha() = 2\nbeta() = 3\n",
    )
    .unwrap_or_else(|error| panic!("rewrite leaf Julia source: {error}"));
    commit_all(tempdir.path(), "leaf change");
    let current_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover current revision"));

    let prepared = coordinator
        .prepare_incremental_analysis(
            &repository,
            &RepoSyncResult {
                repo_id: repository.id.clone(),
                source_kind: RepoSourceKind::LocalCheckout,
                checkout_path: tempdir.path().display().to_string(),
                revision: Some(current_revision),
                ..RepoSyncResult::default()
            },
            Some(previous_revision.as_str()),
        )
        .unwrap_or_else(|error| panic!("prepare incremental merge: {error}"));

    let Some(PreparedIncrementalAnalysis::Analysis(analysis)) = prepared else {
        panic!("expected incremental analysis merge");
    };
    assert!(analysis.symbols.iter().any(|symbol| {
        symbol.qualified_name == "FixturePkg.alpha" && symbol.path == "src/leaf.jl"
    }));
    assert!(analysis.symbols.iter().any(|symbol| {
        symbol.qualified_name == "FixturePkg.beta" && symbol.path == "src/leaf.jl"
    }));
}

#[tokio::test]
async fn prepare_incremental_analysis_reuses_cached_analysis_for_example_churn() {
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(tempdir.path());
    fs::create_dir_all(tempdir.path().join("src"))
        .unwrap_or_else(|error| panic!("create src: {error}"));
    fs::create_dir_all(tempdir.path().join("examples"))
        .unwrap_or_else(|error| panic!("create examples: {error}"));
    fs::write(
        tempdir.path().join("Project.toml"),
        "name = \"FixturePkg\"\n",
    )
    .unwrap_or_else(|error| panic!("write Project.toml: {error}"));
    fs::write(
        tempdir.path().join("src/FixturePkg.jl"),
        "module FixturePkg\nalpha() = 1\nend\n",
    )
    .unwrap_or_else(|error| panic!("write root Julia source: {error}"));
    fs::write(
        tempdir.path().join("examples/demo.jl"),
        "using FixturePkg\nalpha()\n",
    )
    .unwrap_or_else(|error| panic!("write example Julia source: {error}"));
    commit_all(tempdir.path(), "initial");
    let previous_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover previous revision"));

    let repository = RegisteredRepository {
        id: "incremental-example-reuse".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
    };
    let registry = Arc::new(
        bootstrap_builtin_registry().unwrap_or_else(|error| panic!("bootstrap registry: {error}")),
    );
    let coordinator = new_coordinator_with_registry(
        SearchPlaneService::new(PathBuf::from(".")),
        Arc::clone(&registry),
    );
    let baseline =
        analyze_registered_repository_with_registry(&repository, tempdir.path(), registry.as_ref())
            .unwrap_or_else(|error| panic!("seed analysis cache: {error}"));

    fs::write(
        tempdir.path().join("examples/demo.jl"),
        "using FixturePkg\nalpha()\nalpha()\n",
    )
    .unwrap_or_else(|error| panic!("rewrite example Julia source: {error}"));
    commit_all(tempdir.path(), "example change");
    let current_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover current revision"));

    let prepared = coordinator
        .prepare_incremental_analysis(
            &repository,
            &RepoSyncResult {
                repo_id: repository.id.clone(),
                source_kind: RepoSourceKind::LocalCheckout,
                checkout_path: tempdir.path().display().to_string(),
                revision: Some(current_revision),
                ..RepoSyncResult::default()
            },
            Some(previous_revision.as_str()),
        )
        .unwrap_or_else(|error| panic!("prepare incremental example reuse: {error}"));

    let Some(PreparedIncrementalAnalysis::Analysis(analysis)) = prepared else {
        panic!("expected cached analysis reuse");
    };
    assert_eq!(analysis.modules, baseline.modules);
    assert_eq!(analysis.symbols, baseline.symbols);
    assert_eq!(analysis.examples, baseline.examples);
}

#[tokio::test]
async fn local_checkout_does_not_short_circuit_on_revision_match() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        temp_dir.path().join("search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-local-checkout-short-circuit"),
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![crate::repo_index::types::RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: Arc::<str>::from("fn alpha() {}\n"),
        size_bytes: 14,
        modified_unix_ms: 0,
    }];
    search_plane
        .publish_repo_entities_with_revision(
            "alpha/repo",
            &RepositoryAnalysisOutput {
                modules: vec![crate::analyzers::ModuleRecord {
                    repo_id: "alpha/repo".to_string(),
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
    search_plane
        .publish_repo_content_chunks_with_revision("alpha/repo", &documents, Some("rev-1"))
        .await
        .unwrap_or_else(|error| panic!("publish repo content chunks: {error}"));
    let coordinator = new_coordinator(search_plane);

    assert!(
        !coordinator
            .repo_publications_are_current(
                "alpha/repo",
                &RepoSyncResult {
                    repo_id: "alpha/repo".to_string(),
                    source_kind: RepoSourceKind::LocalCheckout,
                    revision: Some("rev-1".to_string()),
                    ..RepoSyncResult::default()
                },
            )
            .await
    );
}

#[tokio::test]
async fn managed_remote_reuses_revision_scoped_publications_after_latest_record_advances() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        temp_dir.path().join("search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-revision-scoped-reuse"),
        SearchMaintenancePolicy::default(),
    );
    let documents = vec![crate::repo_index::types::RepoCodeDocument {
        path: "src/lib.rs".to_string(),
        language: Some("rust".to_string()),
        contents: Arc::<str>::from("fn alpha() {}\n"),
        size_bytes: 14,
        modified_unix_ms: 0,
    }];
    let analysis = RepositoryAnalysisOutput {
        modules: vec![crate::analyzers::ModuleRecord {
            repo_id: "alpha/repo".to_string(),
            module_id: "module:alpha".to_string(),
            qualified_name: "Alpha".to_string(),
            path: "src/lib.rs".to_string(),
        }],
        ..RepositoryAnalysisOutput::default()
    };
    search_plane
        .publish_repo_entities_with_revision("alpha/repo", &analysis, &documents, Some("rev-1"))
        .await
        .unwrap_or_else(|error| panic!("publish repo entities rev-1: {error}"));
    search_plane
        .publish_repo_content_chunks_with_revision("alpha/repo", &documents, Some("rev-1"))
        .await
        .unwrap_or_else(|error| panic!("publish repo content chunks rev-1: {error}"));
    search_plane
        .publish_repo_entities_with_revision("alpha/repo", &analysis, &documents, Some("rev-2"))
        .await
        .unwrap_or_else(|error| panic!("publish repo entities rev-2: {error}"));
    search_plane
        .publish_repo_content_chunks_with_revision("alpha/repo", &documents, Some("rev-2"))
        .await
        .unwrap_or_else(|error| panic!("publish repo content chunks rev-2: {error}"));

    let coordinator = new_coordinator(search_plane);

    assert!(
        coordinator
            .repo_publications_are_current(
                "alpha/repo",
                &RepoSyncResult {
                    repo_id: "alpha/repo".to_string(),
                    source_kind: RepoSourceKind::ManagedRemote,
                    revision: Some("rev-1".to_string()),
                    ..RepoSyncResult::default()
                },
            )
            .await
    );
}

#[tokio::test]
async fn managed_remote_does_not_reuse_evicted_revision_scoped_publications() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let search_plane = SearchPlaneService::with_test_cache_and_revision_retention(
        PathBuf::from("."),
        temp_dir.path().join("search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-revision-retention"),
        SearchMaintenancePolicy::default(),
        1,
    );
    for (table_version_id, revision) in [(1, "rev-1"), (2, "rev-2")] {
        for corpus in [
            SearchCorpusKind::RepoEntity,
            SearchCorpusKind::RepoContentChunk,
        ] {
            search_plane
                .record_repo_publication_input_with_storage_format(
                    corpus,
                    "alpha/repo",
                    SearchRepoPublicationInput {
                        table_name: format!("{corpus}_alpha_repo_{revision}"),
                        schema_version: corpus.schema_version(),
                        source_revision: Some(revision.to_string()),
                        table_version_id,
                        row_count: 1,
                        fragment_count: 1,
                        published_at: format!("2026-04-06T00:00:0{table_version_id}Z"),
                    },
                    SearchPublicationStorageFormat::Parquet,
                )
                .await;
        }
    }

    let coordinator = new_coordinator(search_plane);

    assert!(
        !coordinator
            .repo_publications_are_current(
                "alpha/repo",
                &RepoSyncResult {
                    repo_id: "alpha/repo".to_string(),
                    source_kind: RepoSourceKind::ManagedRemote,
                    revision: Some("rev-1".to_string()),
                    ..RepoSyncResult::default()
                },
            )
            .await
    );
    assert!(
        coordinator
            .repo_publications_are_current(
                "alpha/repo",
                &RepoSyncResult {
                    repo_id: "alpha/repo".to_string(),
                    source_kind: RepoSourceKind::ManagedRemote,
                    revision: Some("rev-2".to_string()),
                    ..RepoSyncResult::default()
                },
            )
            .await
    );
}

#[tokio::test]
async fn managed_remote_requires_both_repo_corpora_to_be_current_parquet_publications() {
    let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let search_plane = SearchPlaneService::with_paths(
        PathBuf::from("."),
        temp_dir.path().join("search-plane"),
        SearchManifestKeyspace::new("xiuxian:test:repo-missing-parquet-short-circuit"),
        SearchMaintenancePolicy::default(),
    );
    let published_at = Utc::now().to_rfc3339();
    search_plane
        .record_repo_publication_input_with_storage_format(
            SearchCorpusKind::RepoEntity,
            "alpha/repo",
            SearchRepoPublicationInput {
                table_name: "repo_entity_alpha_repo".to_string(),
                schema_version: SearchCorpusKind::RepoEntity.schema_version(),
                source_revision: Some("rev-1".to_string()),
                table_version_id: 1,
                row_count: 1,
                fragment_count: 1,
                published_at: published_at.clone(),
            },
            SearchPublicationStorageFormat::Lance,
        )
        .await;
    search_plane
        .record_repo_publication_input_with_storage_format(
            SearchCorpusKind::RepoContentChunk,
            "alpha/repo",
            SearchRepoPublicationInput {
                table_name: "repo_content_chunk_alpha_repo".to_string(),
                schema_version: SearchCorpusKind::RepoContentChunk.schema_version(),
                source_revision: Some("rev-1".to_string()),
                table_version_id: 1,
                row_count: 1,
                fragment_count: 1,
                published_at,
            },
            SearchPublicationStorageFormat::Parquet,
        )
        .await;
    let coordinator = new_coordinator(search_plane);

    assert!(
        !coordinator
            .repo_publications_are_current(
                "alpha/repo",
                &RepoSyncResult {
                    repo_id: "alpha/repo".to_string(),
                    source_kind: RepoSourceKind::ManagedRemote,
                    revision: Some("rev-1".to_string()),
                    ..RepoSyncResult::default()
                },
            )
            .await
    );
}
