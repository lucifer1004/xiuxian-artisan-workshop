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
use crate::gateway::studio::search::handlers::tests::linked_parser_summary::ensure_linked_modelica_parser_summary_service;
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
use xiuxian_wendao_julia::integration_support::{
    spawn_wendaosearch_julia_parser_summary_service,
    spawn_wendaosearch_modelica_parser_summary_service,
};

fn julia_parser_summary_plugin_config(base_url: &str) -> RepositoryPluginConfig {
    RepositoryPluginConfig::Config {
        id: "julia".to_string(),
        options: serde_json::json!({
            "parser_summary_transport": {
                "base_url": base_url,
                "file_summary": {
                    "schema_version": "v3"
                },
                "root_summary": {
                    "schema_version": "v3"
                }
            }
        }),
    }
}

fn modelica_parser_summary_plugin_config(base_url: &str) -> RepositoryPluginConfig {
    RepositoryPluginConfig::Config {
        id: "modelica".to_string(),
        options: serde_json::json!({
            "parser_summary_transport": {
                "base_url": base_url,
                "file_summary": {
                    "schema_version": "v3"
                }
            }
        }),
    }
}

fn mixed_julia_modelica_plugin_configs(
    julia_base_url: &str,
    modelica_base_url: &str,
) -> Vec<RepositoryPluginConfig> {
    vec![
        julia_parser_summary_plugin_config(julia_base_url),
        modelica_parser_summary_plugin_config(modelica_base_url),
    ]
}

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
    let (base_url, mut guard) = spawn_wendaosearch_julia_parser_summary_service().await;
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
        plugins: vec![julia_parser_summary_plugin_config(&base_url)],
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
    guard.kill();
}

#[tokio::test]
async fn prepare_incremental_analysis_reuses_cached_analysis_for_example_churn() {
    let (base_url, mut guard) = spawn_wendaosearch_julia_parser_summary_service().await;
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
        plugins: vec![julia_parser_summary_plugin_config(&base_url)],
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
    guard.kill();
}

#[tokio::test]
async fn prepare_incremental_analysis_reuses_cached_analysis_for_ast_equivalent_julia_source_churn()
{
    let (base_url, mut guard) = spawn_wendaosearch_julia_parser_summary_service().await;
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
    fs::write(tempdir.path().join("src/leaf.jl"), "alpha() = 1\n\n")
        .unwrap_or_else(|error| panic!("write leaf Julia source: {error}"));
    commit_all(tempdir.path(), "initial");
    let previous_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover previous revision"));

    let repository = RegisteredRepository {
        id: "incremental-ast-equivalent".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![julia_parser_summary_plugin_config(&base_url)],
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
        tempdir.path().join("src/leaf.jl"),
        "alpha() = 1\n# trailing comment should stay AST-equivalent\n",
    )
    .unwrap_or_else(|error| panic!("rewrite leaf Julia source: {error}"));
    commit_all(tempdir.path(), "ast equivalent leaf change");
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
        .unwrap_or_else(|error| panic!("prepare incremental ast-equivalent reuse: {error}"));

    let Some(PreparedIncrementalAnalysis::Analysis(analysis)) = prepared else {
        panic!("expected cached analysis reuse for AST-equivalent change");
    };
    assert_eq!(analysis.modules, baseline.modules);
    assert_eq!(analysis.symbols, baseline.symbols);
    assert_eq!(analysis.examples, baseline.examples);
    guard.kill();
}

#[tokio::test]
async fn prepare_incremental_analysis_reuses_cached_analysis_for_ast_equivalent_modelica_source_churn()
 {
    let (base_url, mut guard) = spawn_wendaosearch_modelica_parser_summary_service().await;
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(tempdir.path());
    fs::write(
        tempdir.path().join("package.mo"),
        "within ;\npackage DemoLib\nend DemoLib;\n",
    )
    .unwrap_or_else(|error| panic!("write root package: {error}"));
    fs::write(
        tempdir.path().join("PI.mo"),
        "within DemoLib;\nmodel PI\n  parameter Real k = 1;\nend PI;\n",
    )
    .unwrap_or_else(|error| panic!("write leaf Modelica source: {error}"));
    commit_all(tempdir.path(), "initial");
    let previous_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover previous revision"));

    let repository = RegisteredRepository {
        id: "incremental-modelica-ast-equivalent".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![modelica_parser_summary_plugin_config(&base_url)],
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
        tempdir.path().join("PI.mo"),
        "within DemoLib;\nmodel PI\n  parameter Real k = 1;\nend PI;\n// semantic no-op\n",
    )
    .unwrap_or_else(|error| panic!("rewrite leaf Modelica source: {error}"));
    commit_all(tempdir.path(), "ast equivalent Modelica change");
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
        .unwrap_or_else(|error| panic!("prepare incremental Modelica reuse: {error}"));

    let Some(PreparedIncrementalAnalysis::Analysis(analysis)) = prepared else {
        panic!("expected cached analysis reuse for AST-equivalent Modelica change");
    };
    assert_eq!(analysis.modules, baseline.modules);
    assert_eq!(analysis.symbols, baseline.symbols);
    assert_eq!(analysis.imports, baseline.imports);
    assert_eq!(analysis.examples, baseline.examples);
    assert_eq!(analysis.docs, baseline.docs);
    assert_eq!(analysis.relations, baseline.relations);
    guard.kill();
}

#[tokio::test]
async fn prepare_incremental_analysis_reuses_cached_analysis_for_ast_equivalent_modelica_package_source_churn()
 {
    let (base_url, mut guard) = spawn_wendaosearch_modelica_parser_summary_service().await;
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(tempdir.path());
    fs::write(
        tempdir.path().join("package.mo"),
        "within ;\npackage DemoLib\nend DemoLib;\n",
    )
    .unwrap_or_else(|error| panic!("write root package: {error}"));
    fs::write(
        tempdir.path().join("PI.mo"),
        "within DemoLib;\nmodel PI\n  parameter Real k = 1;\nend PI;\n",
    )
    .unwrap_or_else(|error| panic!("write leaf Modelica source: {error}"));
    commit_all(tempdir.path(), "initial");
    let previous_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover previous revision"));

    let repository = RegisteredRepository {
        id: "incremental-modelica-package-ast-equivalent".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![modelica_parser_summary_plugin_config(&base_url)],
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
        tempdir.path().join("package.mo"),
        "within ;\npackage DemoLib\n// semantic no-op\nend DemoLib;\n",
    )
    .unwrap_or_else(|error| panic!("rewrite package.mo: {error}"));
    commit_all(tempdir.path(), "ast equivalent package.mo change");
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
        .unwrap_or_else(|error| panic!("prepare package.mo incremental reuse: {error}"));

    let Some(PreparedIncrementalAnalysis::Analysis(analysis)) = prepared else {
        panic!("expected cached analysis reuse for AST-equivalent package.mo change");
    };
    assert_eq!(analysis.modules, baseline.modules);
    assert_eq!(analysis.symbols, baseline.symbols);
    assert_eq!(analysis.imports, baseline.imports);
    assert_eq!(analysis.examples, baseline.examples);
    assert_eq!(analysis.docs, baseline.docs);
    assert_eq!(analysis.relations, baseline.relations);
    guard.kill();
}

#[tokio::test]
async fn prepare_incremental_analysis_returns_none_for_semantic_modelica_source_change() {
    let (base_url, mut guard) = spawn_wendaosearch_modelica_parser_summary_service().await;
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(tempdir.path());
    fs::write(
        tempdir.path().join("package.mo"),
        "within ;\npackage DemoLib\nend DemoLib;\n",
    )
    .unwrap_or_else(|error| panic!("write root package: {error}"));
    fs::write(
        tempdir.path().join("PI.mo"),
        "within DemoLib;\nmodel PI\n  parameter Real k = 1;\nend PI;\n",
    )
    .unwrap_or_else(|error| panic!("write leaf Modelica source: {error}"));
    commit_all(tempdir.path(), "initial");
    let previous_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover previous revision"));

    let repository = RegisteredRepository {
        id: "incremental-modelica-semantic-change".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![modelica_parser_summary_plugin_config(&base_url)],
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
        tempdir.path().join("PI.mo"),
        "within DemoLib;\nmodel PI\n  parameter Real k = 1;\n  parameter Real Ti = 0.1;\nend PI;\n",
    )
    .unwrap_or_else(|error| panic!("rewrite leaf Modelica source: {error}"));
    commit_all(tempdir.path(), "semantic Modelica change");
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
        .unwrap_or_else(|error| panic!("prepare semantic Modelica change: {error}"));

    assert!(
        prepared.is_none(),
        "semantic Modelica source change should fall back to full analysis"
    );
    guard.kill();
}

#[tokio::test]
async fn prepare_incremental_analysis_merges_leaf_modelica_source_changes() {
    let (base_url, mut guard) = spawn_wendaosearch_modelica_parser_summary_service().await;
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(tempdir.path());
    fs::write(
        tempdir.path().join("package.mo"),
        "within ;\npackage DemoLib\nend DemoLib;\n",
    )
    .unwrap_or_else(|error| panic!("write root package: {error}"));
    fs::write(
        tempdir.path().join("PI.mo"),
        "within DemoLib;\nmodel PI\nend PI;\n",
    )
    .unwrap_or_else(|error| panic!("write leaf Modelica source: {error}"));
    commit_all(tempdir.path(), "initial");
    let previous_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover previous revision"));

    let repository = RegisteredRepository {
        id: "incremental-modelica-leaf-merge".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![modelica_parser_summary_plugin_config(&base_url)],
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
        tempdir.path().join("PI.mo"),
        "within DemoLib;\nmodel PID\nend PID;\n",
    )
    .unwrap_or_else(|error| panic!("rewrite leaf Modelica source: {error}"));
    commit_all(tempdir.path(), "leaf Modelica semantic change");
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
        .unwrap_or_else(|error| panic!("prepare leaf Modelica merge: {error}"));

    let Some(PreparedIncrementalAnalysis::Analysis(analysis)) = prepared else {
        panic!("expected incremental analysis merge for leaf Modelica source change");
    };
    assert!(
        analysis
            .symbols
            .iter()
            .any(|symbol| symbol.qualified_name == "DemoLib.PID"),
        "symbols: {:?}",
        analysis.symbols
    );
    assert!(
        analysis
            .symbols
            .iter()
            .all(|symbol| symbol.qualified_name != "DemoLib.PI"),
        "symbols: {:?}",
        analysis.symbols
    );
    assert!(
        analysis.imports.is_empty(),
        "imports: {:?}",
        analysis.imports
    );
    guard.kill();
}

#[tokio::test]
async fn prepare_incremental_analysis_merges_import_bearing_leaf_modelica_source_changes() {
    let (base_url, mut guard) = spawn_wendaosearch_modelica_parser_summary_service().await;
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(tempdir.path());
    fs::write(
        tempdir.path().join("package.mo"),
        "within ;\npackage DemoLib\nend DemoLib;\n",
    )
    .unwrap_or_else(|error| panic!("write root package: {error}"));
    fs::write(
        tempdir.path().join("PI.mo"),
        "within DemoLib;\nmodel PI\nend PI;\n",
    )
    .unwrap_or_else(|error| panic!("write leaf Modelica source: {error}"));
    commit_all(tempdir.path(), "initial");
    let previous_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover previous revision"));

    let repository = RegisteredRepository {
        id: "incremental-modelica-import-merge".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![modelica_parser_summary_plugin_config(&base_url)],
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
        tempdir.path().join("PI.mo"),
        "within DemoLib;\nmodel PI\n  import Modelica.Math;\nend PI;\n",
    )
    .unwrap_or_else(|error| panic!("rewrite import-bearing Modelica source: {error}"));
    commit_all(tempdir.path(), "import-bearing Modelica change");
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
        .unwrap_or_else(|error| panic!("prepare import-bearing Modelica merge: {error}"));

    let Some(PreparedIncrementalAnalysis::Analysis(analysis)) = prepared else {
        panic!("expected incremental analysis merge for import-bearing Modelica change");
    };
    assert!(
        analysis.imports.iter().any(|import| {
            import.path == "PI.mo"
                && import.module_id == "repo:incremental-modelica-import-merge:module:DemoLib"
                && import.import_name == "Math"
                && import.target_package == "Modelica"
                && import.source_module == "Modelica.Math"
        }),
        "imports: {:?}",
        analysis.imports
    );
    guard.kill();
}

#[tokio::test]
async fn prepare_incremental_analysis_returns_none_for_documentation_annotation_modelica_source_change()
 {
    let (base_url, mut guard) = spawn_wendaosearch_modelica_parser_summary_service().await;
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(tempdir.path());
    fs::write(
        tempdir.path().join("package.mo"),
        "within ;\npackage DemoLib\nend DemoLib;\n",
    )
    .unwrap_or_else(|error| panic!("write root package: {error}"));
    fs::write(
        tempdir.path().join("PI.mo"),
        "within DemoLib;\nmodel PI\nend PI;\n",
    )
    .unwrap_or_else(|error| panic!("write leaf Modelica source: {error}"));
    commit_all(tempdir.path(), "initial");
    let previous_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover previous revision"));

    let repository = RegisteredRepository {
        id: "incremental-modelica-doc-bail".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![modelica_parser_summary_plugin_config(&base_url)],
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
        tempdir.path().join("PI.mo"),
        "within DemoLib;\nmodel PI\n  annotation(Documentation(info = \"doc\"));\nend PI;\n",
    )
    .unwrap_or_else(|error| panic!("rewrite documentation Modelica source: {error}"));
    commit_all(tempdir.path(), "documentation annotation Modelica change");
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
        .unwrap_or_else(|error| {
            panic!("prepare documentation annotation Modelica change: {error}")
        });

    assert!(
        prepared.is_none(),
        "documentation annotation Modelica change should stay on full-analysis fallback"
    );
    guard.kill();
}

#[tokio::test]
async fn prepare_incremental_analysis_merges_root_package_modelica_source_changes() {
    let (base_url, mut guard) = spawn_wendaosearch_modelica_parser_summary_service().await;
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(tempdir.path());
    fs::write(
        tempdir.path().join("package.mo"),
        "within ;\npackage DemoLib\nend DemoLib;\n",
    )
    .unwrap_or_else(|error| panic!("write root package: {error}"));
    fs::write(
        tempdir.path().join("PI.mo"),
        "within DemoLib;\nmodel PI\nend PI;\n",
    )
    .unwrap_or_else(|error| panic!("write leaf Modelica source: {error}"));
    commit_all(tempdir.path(), "initial");
    let previous_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover previous revision"));

    let repository = RegisteredRepository {
        id: "incremental-modelica-package-doc-bail".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![modelica_parser_summary_plugin_config(&base_url)],
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
        tempdir.path().join("package.mo"),
        "within ;\npackage DemoLib\n  import Modelica.Math;\n  annotation(Documentation(info = \"doc\"));\nend DemoLib;\n",
    )
    .unwrap_or_else(|error| panic!("rewrite root package source: {error}"));
    commit_all(tempdir.path(), "root package Modelica change");
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
        .unwrap_or_else(|error| panic!("prepare root package Modelica change: {error}"));

    let Some(PreparedIncrementalAnalysis::Analysis(analysis)) = prepared else {
        panic!("expected incremental analysis merge for root package Modelica change");
    };
    assert!(
        analysis.imports.iter().any(|import| {
            import.path == "package.mo"
                && import.module_id == "repo:incremental-modelica-package-doc-bail:module:DemoLib"
                && import.import_name == "Math"
                && import.target_package == "Modelica"
                && import.source_module == "Modelica.Math"
        }),
        "imports: {:?}",
        analysis.imports
    );
    assert!(
        analysis
            .docs
            .iter()
            .any(|doc| doc.path == "package.mo#annotation.documentation"),
        "docs: {:?}",
        analysis.docs
    );
    assert!(
        analysis.relations.iter().any(|relation| {
            relation.kind == crate::analyzers::RelationKind::Documents
                && relation.source_id
                    == "repo:incremental-modelica-package-doc-bail:doc:package.mo#annotation.documentation"
                && relation.target_id
                    == "repo:incremental-modelica-package-doc-bail:module:DemoLib"
        }),
        "relations: {:?}",
        analysis.relations
    );
    guard.kill();
}

#[tokio::test]
async fn prepare_incremental_analysis_reuses_cached_analysis_for_ast_equivalent_nested_modelica_package_source_churn()
 {
    ensure_linked_modelica_parser_summary_service()
        .unwrap_or_else(|error| panic!("linked Modelica parser-summary service: {error}"));
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(tempdir.path());
    fs::write(
        tempdir.path().join("package.mo"),
        "within ;\npackage DemoLib\nend DemoLib;\n",
    )
    .unwrap_or_else(|error| panic!("write root package: {error}"));
    fs::create_dir_all(tempdir.path().join("Blocks"))
        .unwrap_or_else(|error| panic!("create Blocks dir: {error}"));
    fs::write(
        tempdir.path().join("Blocks/package.mo"),
        "within DemoLib;\npackage Blocks\n  import Modelica.Math;\nend Blocks;\n",
    )
    .unwrap_or_else(|error| panic!("write nested package: {error}"));
    commit_all(tempdir.path(), "initial");
    let previous_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover previous revision"));

    let repository = RegisteredRepository {
        id: "incremental-modelica-nested-package-ast-equivalent".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Id("modelica".to_string())],
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
        tempdir.path().join("Blocks/package.mo"),
        "within DemoLib;\npackage Blocks\n  // semantic no-op\n  import Modelica.Math;\nend Blocks;\n",
    )
    .unwrap_or_else(|error| panic!("rewrite nested package: {error}"));
    commit_all(tempdir.path(), "ast equivalent nested package change");
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
        .unwrap_or_else(|error| panic!("prepare nested package incremental reuse: {error}"));

    let Some(PreparedIncrementalAnalysis::Analysis(analysis)) = prepared else {
        panic!("expected cached analysis reuse for AST-equivalent nested package change");
    };
    assert_eq!(analysis.modules, baseline.modules);
    assert_eq!(analysis.symbols, baseline.symbols);
    assert_eq!(analysis.imports, baseline.imports);
    assert_eq!(analysis.examples, baseline.examples);
    assert_eq!(analysis.docs, baseline.docs);
    assert_eq!(analysis.relations, baseline.relations);
}

#[tokio::test]
async fn prepare_incremental_analysis_merges_nested_package_modelica_source_changes() {
    ensure_linked_modelica_parser_summary_service()
        .unwrap_or_else(|error| panic!("linked Modelica parser-summary service: {error}"));
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(tempdir.path());
    fs::write(
        tempdir.path().join("package.mo"),
        "within ;\npackage DemoLib\nend DemoLib;\n",
    )
    .unwrap_or_else(|error| panic!("write root package: {error}"));
    fs::create_dir_all(tempdir.path().join("Blocks"))
        .unwrap_or_else(|error| panic!("create Blocks dir: {error}"));
    fs::write(
        tempdir.path().join("Blocks/package.mo"),
        "within DemoLib;\npackage Blocks\nend Blocks;\n",
    )
    .unwrap_or_else(|error| panic!("write nested package: {error}"));
    commit_all(tempdir.path(), "initial");
    let previous_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover previous revision"));

    let repository = RegisteredRepository {
        id: "incremental-modelica-nested-package-merge".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Id("modelica".to_string())],
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
        tempdir.path().join("Blocks/package.mo"),
        "within DemoLib;\npackage Blocks\n  import Modelica.Math;\n  annotation(Documentation(info = \"doc\"));\nend Blocks;\n",
    )
    .unwrap_or_else(|error| panic!("rewrite nested package: {error}"));
    commit_all(tempdir.path(), "nested package Modelica change");
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
        .unwrap_or_else(|error| panic!("prepare nested package Modelica change: {error}"));

    let Some(PreparedIncrementalAnalysis::Analysis(analysis)) = prepared else {
        panic!("expected incremental analysis merge for nested package Modelica change");
    };
    assert!(
        analysis.imports.iter().any(|import| {
            import.path == "Blocks/package.mo"
                && import.module_id
                    == "repo:incremental-modelica-nested-package-merge:module:DemoLib.Blocks"
                && import.import_name == "Math"
                && import.target_package == "Modelica"
                && import.source_module == "Modelica.Math"
        }),
        "imports: {:?}",
        analysis.imports
    );
    assert!(
        analysis
            .docs
            .iter()
            .any(|doc| doc.path == "Blocks/package.mo#annotation.documentation"),
        "docs: {:?}",
        analysis.docs
    );
    assert!(
        analysis.relations.iter().any(|relation| {
            relation.source_id
                == "repo:incremental-modelica-nested-package-merge:doc:Blocks/package.mo#annotation.documentation"
                && relation.target_id
                    == "repo:incremental-modelica-nested-package-merge:module:DemoLib.Blocks"
        }),
        "relations: {:?}",
        analysis.relations
    );
}

#[tokio::test]
async fn prepare_incremental_analysis_returns_none_for_nested_package_modelica_declaration_change()
{
    ensure_linked_modelica_parser_summary_service()
        .unwrap_or_else(|error| panic!("linked Modelica parser-summary service: {error}"));
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(tempdir.path());
    fs::write(
        tempdir.path().join("package.mo"),
        "within ;\npackage DemoLib\nend DemoLib;\n",
    )
    .unwrap_or_else(|error| panic!("write root package: {error}"));
    fs::create_dir_all(tempdir.path().join("Blocks"))
        .unwrap_or_else(|error| panic!("create Blocks dir: {error}"));
    fs::write(
        tempdir.path().join("Blocks/package.mo"),
        "within DemoLib;\npackage Blocks\nend Blocks;\n",
    )
    .unwrap_or_else(|error| panic!("write nested package: {error}"));
    commit_all(tempdir.path(), "initial");
    let previous_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover previous revision"));

    let repository = RegisteredRepository {
        id: "incremental-modelica-nested-package-fallback".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Id("modelica".to_string())],
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
        tempdir.path().join("Blocks/package.mo"),
        "within DemoLib;\npackage Blocks\n  model Controller\n  end Controller;\nend Blocks;\n",
    )
    .unwrap_or_else(|error| panic!("rewrite nested package: {error}"));
    commit_all(tempdir.path(), "nested package declaration change");
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
        .unwrap_or_else(|error| panic!("prepare nested package declaration change: {error}"));

    assert!(
        prepared.is_none(),
        "nested package declaration change should stay on full-analysis fallback"
    );
}

#[tokio::test]
async fn prepare_incremental_analysis_returns_none_for_root_package_modelica_rename() {
    let (base_url, mut guard) = spawn_wendaosearch_modelica_parser_summary_service().await;
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(tempdir.path());
    fs::write(
        tempdir.path().join("package.mo"),
        "within ;\npackage DemoLib\nend DemoLib;\n",
    )
    .unwrap_or_else(|error| panic!("write root package: {error}"));
    fs::write(
        tempdir.path().join("PI.mo"),
        "within DemoLib;\nmodel PI\nend PI;\n",
    )
    .unwrap_or_else(|error| panic!("write leaf Modelica source: {error}"));
    commit_all(tempdir.path(), "initial");
    let previous_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover previous revision"));

    let repository = RegisteredRepository {
        id: "incremental-modelica-root-rename".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![modelica_parser_summary_plugin_config(&base_url)],
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
        tempdir.path().join("package.mo"),
        "within ;\npackage RenamedLib\nend RenamedLib;\n",
    )
    .unwrap_or_else(|error| panic!("rewrite root package name: {error}"));
    commit_all(tempdir.path(), "root package rename");
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
        .unwrap_or_else(|error| panic!("prepare root package rename: {error}"));

    assert!(
        prepared.is_none(),
        "root package rename should stay on full-analysis fallback"
    );
    guard.kill();
}

#[tokio::test]
async fn prepare_incremental_analysis_reuses_cached_analysis_for_ast_equivalent_mixed_julia_modelica_julia_source_churn()
 {
    let (julia_base_url, mut julia_guard) = spawn_wendaosearch_julia_parser_summary_service().await;
    let (modelica_base_url, mut modelica_guard) =
        spawn_wendaosearch_modelica_parser_summary_service().await;
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(tempdir.path());
    fs::create_dir_all(tempdir.path().join("src"))
        .unwrap_or_else(|error| panic!("create src: {error}"));
    fs::write(tempdir.path().join("Project.toml"), "name = \"MixedPkg\"\n")
        .unwrap_or_else(|error| panic!("write Project.toml: {error}"));
    fs::write(
        tempdir.path().join("src/MixedPkg.jl"),
        "module MixedPkg\ninclude(\"leaf.jl\")\nend\n",
    )
    .unwrap_or_else(|error| panic!("write root Julia source: {error}"));
    fs::write(tempdir.path().join("src/leaf.jl"), "alpha() = 1\n")
        .unwrap_or_else(|error| panic!("write leaf Julia source: {error}"));
    fs::write(
        tempdir.path().join("package.mo"),
        "within ;\npackage DemoLib\nend DemoLib;\n",
    )
    .unwrap_or_else(|error| panic!("write root package: {error}"));
    fs::write(
        tempdir.path().join("PI.mo"),
        "within DemoLib;\nmodel PI\n  parameter Real k = 1;\nend PI;\n",
    )
    .unwrap_or_else(|error| panic!("write leaf Modelica source: {error}"));
    commit_all(tempdir.path(), "initial");
    let previous_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover previous revision"));

    let repository = RegisteredRepository {
        id: "incremental-mixed-julia-modelica-julia".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: mixed_julia_modelica_plugin_configs(&julia_base_url, &modelica_base_url),
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
        tempdir.path().join("src/leaf.jl"),
        "alpha() = 1\n# semantic no-op\n",
    )
    .unwrap_or_else(|error| panic!("rewrite leaf Julia source: {error}"));
    commit_all(tempdir.path(), "ast equivalent mixed Julia change");
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
        .unwrap_or_else(|error| panic!("prepare mixed Julia reuse: {error}"));

    let Some(PreparedIncrementalAnalysis::Analysis(analysis)) = prepared else {
        panic!("expected cached analysis reuse for mixed Julia AST-equivalent change");
    };
    assert_eq!(analysis.modules, baseline.modules);
    assert_eq!(analysis.symbols, baseline.symbols);
    assert_eq!(analysis.imports, baseline.imports);
    assert_eq!(analysis.examples, baseline.examples);
    assert_eq!(analysis.docs, baseline.docs);
    assert_eq!(analysis.relations, baseline.relations);
    julia_guard.kill();
    modelica_guard.kill();
}

#[tokio::test]
async fn prepare_incremental_analysis_reuses_cached_analysis_for_ast_equivalent_mixed_julia_modelica_modelica_source_churn()
 {
    let (julia_base_url, mut julia_guard) = spawn_wendaosearch_julia_parser_summary_service().await;
    let (modelica_base_url, mut modelica_guard) =
        spawn_wendaosearch_modelica_parser_summary_service().await;
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(tempdir.path());
    fs::create_dir_all(tempdir.path().join("src"))
        .unwrap_or_else(|error| panic!("create src: {error}"));
    fs::write(tempdir.path().join("Project.toml"), "name = \"MixedPkg\"\n")
        .unwrap_or_else(|error| panic!("write Project.toml: {error}"));
    fs::write(
        tempdir.path().join("src/MixedPkg.jl"),
        "module MixedPkg\ninclude(\"leaf.jl\")\nend\n",
    )
    .unwrap_or_else(|error| panic!("write root Julia source: {error}"));
    fs::write(tempdir.path().join("src/leaf.jl"), "alpha() = 1\n")
        .unwrap_or_else(|error| panic!("write leaf Julia source: {error}"));
    fs::write(
        tempdir.path().join("package.mo"),
        "within ;\npackage DemoLib\nend DemoLib;\n",
    )
    .unwrap_or_else(|error| panic!("write root package: {error}"));
    fs::write(
        tempdir.path().join("PI.mo"),
        "within DemoLib;\nmodel PI\n  parameter Real k = 1;\nend PI;\n",
    )
    .unwrap_or_else(|error| panic!("write leaf Modelica source: {error}"));
    commit_all(tempdir.path(), "initial");
    let previous_revision = xiuxian_git_repo::discover_checkout_metadata(tempdir.path())
        .and_then(|metadata| metadata.revision)
        .unwrap_or_else(|| panic!("discover previous revision"));

    let repository = RegisteredRepository {
        id: "incremental-mixed-julia-modelica-modelica".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: mixed_julia_modelica_plugin_configs(&julia_base_url, &modelica_base_url),
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
        tempdir.path().join("PI.mo"),
        "within DemoLib;\nmodel PI\n  parameter Real k = 1;\nend PI;\n// semantic no-op\n",
    )
    .unwrap_or_else(|error| panic!("rewrite leaf Modelica source: {error}"));
    commit_all(tempdir.path(), "ast equivalent mixed Modelica change");
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
        .unwrap_or_else(|error| panic!("prepare mixed Modelica reuse: {error}"));

    let Some(PreparedIncrementalAnalysis::Analysis(analysis)) = prepared else {
        panic!("expected cached analysis reuse for mixed Modelica AST-equivalent change");
    };
    assert_eq!(analysis.modules, baseline.modules);
    assert_eq!(analysis.symbols, baseline.symbols);
    assert_eq!(analysis.imports, baseline.imports);
    assert_eq!(analysis.examples, baseline.examples);
    assert_eq!(analysis.docs, baseline.docs);
    assert_eq!(analysis.relations, baseline.relations);
    julia_guard.kill();
    modelica_guard.kill();
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
