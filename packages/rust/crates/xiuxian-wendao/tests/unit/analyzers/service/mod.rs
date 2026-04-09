use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::analyzers::config::{RegisteredRepository, RepositoryRefreshPolicy};
use crate::analyzers::query::{RefineEntityDocRequest, RefineEntityDocResponse};
use crate::analyzers::records::RepositoryRecord;
use crate::analyzers::registry::PluginRegistry;
use crate::analyzers::{
    AnalysisContext, PluginAnalysisOutput, RepoIntelligenceError, RepoIntelligencePlugin,
    RepoSourceFile, RepositoryAnalysisOutput, RepositoryPluginConfig,
};
use crate::gateway::studio::test_support::{commit_all, init_git_repository};
use xiuxian_git_repo::LocalCheckoutMetadata;

use super::analysis::analyze_registered_repository_bundle_with_registry;
use super::merge::{hydrate_repository_record, merge_repository_record};

#[test]
fn test_refine_contract_serialization() {
    let req = RefineEntityDocRequest {
        repo_id: "test".to_string(),
        entity_id: "sym1".to_string(),
        user_hints: Some("more details".to_string()),
    };
    let res = RefineEntityDocResponse {
        repo_id: "test".to_string(),
        entity_id: "sym1".to_string(),
        refined_content: "Refined".to_string(),
        verification_state: "verified".to_string(),
    };
    assert_eq!(req.repo_id, "test");
    assert_eq!(res.verification_state, "verified");
}

#[test]
fn merge_repository_record_prefers_overlay_metadata() {
    let base = RepositoryRecord {
        repo_id: "demo".to_string(),
        name: "demo".to_string(),
        path: "/tmp/demo".to_string(),
        url: Some("https://base.invalid/demo.git".to_string()),
        revision: Some("base-rev".to_string()),
        version: None,
        uuid: None,
        dependencies: Vec::new(),
    };
    let overlay = RepositoryRecord {
        repo_id: "demo".to_string(),
        name: "DemoPkg".to_string(),
        path: "/tmp/demo".to_string(),
        url: None,
        revision: None,
        version: Some("0.1.0".to_string()),
        uuid: Some("uuid-demo".to_string()),
        dependencies: vec!["LinearAlgebra".to_string()],
    };

    let merged = merge_repository_record(base, overlay);

    assert_eq!(merged.name, "DemoPkg");
    assert_eq!(merged.url.as_deref(), Some("https://base.invalid/demo.git"));
    assert_eq!(merged.revision.as_deref(), Some("base-rev"));
    assert_eq!(merged.version.as_deref(), Some("0.1.0"));
    assert_eq!(merged.uuid.as_deref(), Some("uuid-demo"));
    assert_eq!(merged.dependencies, vec!["LinearAlgebra".to_string()]);
}

#[test]
fn hydrate_repository_record_backfills_checkout_metadata() {
    let repository = RegisteredRepository {
        id: "sample".to_string(),
        path: Some("/tmp/sample".into()),
        url: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        git_ref: None,
        plugins: Vec::new(),
    };
    let mut record = RepositoryRecord {
        repo_id: String::new(),
        name: String::new(),
        path: String::new(),
        url: None,
        revision: None,
        version: None,
        uuid: None,
        dependencies: Vec::new(),
    };

    hydrate_repository_record(
        &mut record,
        &repository,
        Path::new("/tmp/sample"),
        Some(&LocalCheckoutMetadata {
            revision: Some("abc123".to_string()),
            remote_url: Some("https://example.invalid/sample.git".to_string()),
        }),
    );

    assert_eq!(record.repo_id, "sample");
    assert_eq!(record.name, "sample");
    assert_eq!(record.path, "/tmp/sample");
    assert_eq!(
        record.url.as_deref(),
        Some("https://example.invalid/sample.git")
    );
    assert_eq!(record.revision.as_deref(), Some("abc123"));
}

#[derive(Clone)]
struct CountingJuliaPlugin {
    calls: Arc<AtomicUsize>,
}

impl RepoIntelligencePlugin for CountingJuliaPlugin {
    fn id(&self) -> &'static str {
        "julia"
    }

    fn supports_repository(&self, _repository: &RegisteredRepository) -> bool {
        true
    }

    fn analyze_file(
        &self,
        _context: &AnalysisContext,
        _file: &RepoSourceFile,
    ) -> Result<PluginAnalysisOutput, RepoIntelligenceError> {
        Ok(PluginAnalysisOutput::default())
    }

    fn analyze_repository(
        &self,
        context: &AnalysisContext,
        repository_root: &Path,
    ) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(RepositoryAnalysisOutput {
            repository: Some(RepositoryRecord {
                repo_id: context.repository.id.clone(),
                name: "FixturePkg".to_string(),
                path: repository_root.display().to_string(),
                url: None,
                revision: None,
                version: None,
                uuid: None,
                dependencies: Vec::new(),
            }),
            ..RepositoryAnalysisOutput::default()
        })
    }
}

#[test]
fn analyze_repository_reuses_cached_analysis_for_non_affecting_revision_churn() {
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

    let repository = RegisteredRepository {
        id: "counting-julia-cache".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        git_ref: None,
        plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = PluginRegistry::new();
    registry
        .register(CountingJuliaPlugin {
            calls: Arc::clone(&calls),
        })
        .unwrap_or_else(|error| panic!("register test plugin: {error}"));

    let first =
        analyze_registered_repository_bundle_with_registry(&repository, tempdir.path(), &registry)
            .unwrap_or_else(|error| panic!("first analysis should succeed: {error}"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    fs::write(
        tempdir.path().join("notes.txt"),
        "second non-affecting note\n",
    )
    .unwrap_or_else(|error| panic!("rewrite notes: {error}"));
    commit_all(tempdir.path(), "non-affecting");

    let second =
        analyze_registered_repository_bundle_with_registry(&repository, tempdir.path(), &registry)
            .unwrap_or_else(|error| panic!("second analysis should succeed: {error}"));

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        first.cache_key.analysis_identity,
        second.cache_key.analysis_identity
    );
    assert_ne!(
        first.cache_key.checkout_revision,
        second.cache_key.checkout_revision
    );
}

#[test]
fn analyze_repository_invalidates_cached_analysis_for_julia_source_change() {
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(tempdir.path());
    fs::create_dir_all(tempdir.path().join("src"))
        .unwrap_or_else(|error| panic!("create src: {error}"));
    let source_path = tempdir.path().join("src/FixturePkg.jl");
    fs::write(
        tempdir.path().join("Project.toml"),
        "name = \"FixturePkg\"\n",
    )
    .unwrap_or_else(|error| panic!("write Project.toml: {error}"));
    fs::write(&source_path, "module FixturePkg\nend\n")
        .unwrap_or_else(|error| panic!("write Julia source: {error}"));
    commit_all(tempdir.path(), "initial");

    let repository = RegisteredRepository {
        id: "counting-julia-cache-change".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        git_ref: None,
        plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = PluginRegistry::new();
    registry
        .register(CountingJuliaPlugin {
            calls: Arc::clone(&calls),
        })
        .unwrap_or_else(|error| panic!("register test plugin: {error}"));

    let first =
        analyze_registered_repository_bundle_with_registry(&repository, tempdir.path(), &registry)
            .unwrap_or_else(|error| panic!("first analysis should succeed: {error}"));
    assert_eq!(calls.load(Ordering::SeqCst), 1);

    fs::write(&source_path, "module FixturePkg\nconst VERSION = 2\nend\n")
        .unwrap_or_else(|error| panic!("rewrite Julia source: {error}"));
    commit_all(tempdir.path(), "affecting");

    let second =
        analyze_registered_repository_bundle_with_registry(&repository, tempdir.path(), &registry)
            .unwrap_or_else(|error| panic!("second analysis should succeed: {error}"));

    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_ne!(
        first.cache_key.analysis_identity,
        second.cache_key.analysis_identity
    );
}
