use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::analyzers::config::{RegisteredRepository, RepositoryRefreshPolicy};
use crate::analyzers::query::{RefineEntityDocRequest, RefineEntityDocResponse};
use crate::analyzers::records::{
    ImportKind, ImportRecord, ModuleRecord, RepoSymbolKind, RepositoryRecord, SymbolRecord,
};
use crate::analyzers::registry::PluginRegistry;
use crate::analyzers::{
    AnalysisContext, PluginAnalysisOutput, RepoIntelligenceError, RepoIntelligencePlugin,
    RepoSourceFile, RepositoryAnalysisOutput, RepositoryPluginConfig,
    resolve_registered_repository_source,
};
use crate::gateway::studio::test_support::{commit_all, init_git_repository};
use xiuxian_git_repo::{LocalCheckoutMetadata, SyncMode};

use super::analysis::{
    analyze_registered_repository_bundle_with_registry,
    analyze_registered_repository_target_file_with_registry,
};
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
        context: &AnalysisContext,
        file: &RepoSourceFile,
    ) -> Result<PluginAnalysisOutput, RepoIntelligenceError> {
        let module_id = format!("repo:{}:module:FixturePkg", context.repository.id);
        Ok(PluginAnalysisOutput {
            modules: vec![ModuleRecord {
                repo_id: context.repository.id.clone(),
                module_id: module_id.clone(),
                qualified_name: "FixturePkg".to_string(),
                path: file.path.clone(),
            }],
            symbols: vec![SymbolRecord {
                repo_id: context.repository.id.clone(),
                symbol_id: format!("repo:{}:symbol:solve", context.repository.id),
                module_id: Some(module_id),
                name: "solve".to_string(),
                qualified_name: "FixturePkg.solve".to_string(),
                kind: RepoSymbolKind::Function,
                path: file.path.clone(),
                line_start: Some(3),
                line_end: Some(3),
                signature: Some("solve(x)".to_string()),
                audit_status: None,
                verification_state: None,
                attributes: BTreeMap::new(),
            }],
            examples: Vec::new(),
            docs: Vec::new(),
            diagnostics: Vec::new(),
        })
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

#[derive(Clone)]
struct CachedTargetFilePlugin {
    repository_calls: Arc<AtomicUsize>,
    file_calls: Arc<AtomicUsize>,
}

impl RepoIntelligencePlugin for CachedTargetFilePlugin {
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
        self.file_calls.fetch_add(1, Ordering::SeqCst);
        Err(RepoIntelligenceError::AnalysisFailed {
            message: "target-file analysis should reuse cached repository output".to_string(),
        })
    }

    fn analyze_repository(
        &self,
        context: &AnalysisContext,
        repository_root: &Path,
    ) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
        self.repository_calls.fetch_add(1, Ordering::SeqCst);
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
            modules: vec![ModuleRecord {
                repo_id: context.repository.id.clone(),
                module_id: format!("repo:{}:module:FixturePkg", context.repository.id),
                qualified_name: "FixturePkg".to_string(),
                path: "src/FixturePkg.jl".to_string(),
            }],
            symbols: vec![SymbolRecord {
                repo_id: context.repository.id.clone(),
                symbol_id: format!("repo:{}:symbol:solve", context.repository.id),
                module_id: Some(format!("repo:{}:module:FixturePkg", context.repository.id)),
                name: "solve".to_string(),
                qualified_name: "FixturePkg.solve".to_string(),
                kind: RepoSymbolKind::Function,
                path: "src/FixturePkg.jl".to_string(),
                line_start: Some(3),
                line_end: Some(3),
                signature: Some("solve(x)".to_string()),
                audit_status: None,
                verification_state: None,
                attributes: BTreeMap::new(),
            }],
            imports: vec![ImportRecord {
                repo_id: context.repository.id.clone(),
                module_id: format!("repo:{}:module:FixturePkg", context.repository.id),
                import_name: "LinearAlgebra".to_string(),
                target_package: "LinearAlgebra".to_string(),
                source_module: "LinearAlgebra".to_string(),
                kind: ImportKind::Module,
                line_start: Some(2),
                resolved_id: None,
                attributes: BTreeMap::from([(
                    "dependency_form".to_string(),
                    "qualified_import".to_string(),
                )]),
            }],
            ..RepositoryAnalysisOutput::default()
        })
    }
}

#[test]
fn analyze_target_file_reuses_existing_managed_checkout_without_remote_probe() {
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let source_dir = tempdir.path().join("fixture-source");
    fs::create_dir_all(source_dir.join("src"))
        .unwrap_or_else(|error| panic!("create source src: {error}"));
    init_git_repository(&source_dir);
    fs::write(
        source_dir.join("Project.toml"),
        "name = \"FixturePkg\"\nversion = \"0.1.0\"\n",
    )
    .unwrap_or_else(|error| panic!("write Project.toml: {error}"));
    fs::write(
        source_dir.join("src/FixturePkg.jl"),
        "module FixturePkg\nsolve(x) = x\nend\n",
    )
    .unwrap_or_else(|error| panic!("write Julia source: {error}"));
    commit_all(&source_dir, "initial");

    let remote_dir = tempdir.path().join("fixture-remote.git");
    let clone_status = Command::new("git")
        .args([
            "clone",
            "--bare",
            source_dir
                .to_str()
                .unwrap_or_else(|| panic!("source path utf8")),
            remote_dir
                .to_str()
                .unwrap_or_else(|| panic!("remote path utf8")),
        ])
        .status()
        .unwrap_or_else(|error| panic!("clone bare remote: {error}"));
    assert!(clone_status.success(), "clone bare remote should succeed");

    let repository = RegisteredRepository {
        id: format!("managed-target-file-{}", std::process::id()),
        path: None,
        url: Some(remote_dir.display().to_string()),
        refresh: RepositoryRefreshPolicy::Fetch,
        git_ref: None,
        plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
    };

    let materialized =
        resolve_registered_repository_source(&repository, tempdir.path(), SyncMode::Ensure)
            .unwrap_or_else(|error| panic!("materialize managed checkout: {error}"));
    assert!(materialized.checkout_root.is_dir());

    fs::remove_dir_all(&remote_dir)
        .unwrap_or_else(|error| panic!("remove bare remote to block ensure path: {error}"));

    let calls = Arc::new(AtomicUsize::new(0));
    let mut registry = PluginRegistry::new();
    registry
        .register(CountingJuliaPlugin {
            calls: Arc::clone(&calls),
        })
        .unwrap_or_else(|error| panic!("register test plugin: {error}"));

    let analysis = analyze_registered_repository_target_file_with_registry(
        &repository,
        tempdir.path(),
        &registry,
        "src/FixturePkg.jl",
    )
    .unwrap_or_else(|error| panic!("target-file analysis should reuse checkout: {error}"));

    assert_eq!(analysis.modules.len(), 1);
    assert_eq!(analysis.modules[0].path, "src/FixturePkg.jl");
    assert_eq!(analysis.symbols.len(), 1);
    assert_eq!(analysis.symbols[0].path, "src/FixturePkg.jl");
    assert_eq!(calls.load(Ordering::SeqCst), 0);

    let _ = fs::remove_dir_all(&materialized.checkout_root);
    if let Some(mirror_root) = materialized.mirror_root.as_ref() {
        let _ = fs::remove_dir_all(mirror_root);
    }
}

#[test]
fn analyze_target_file_reuses_ready_cached_analysis_before_file_probe() {
    let tempdir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    init_git_repository(tempdir.path());
    fs::create_dir_all(tempdir.path().join("src"))
        .unwrap_or_else(|error| panic!("create src: {error}"));
    fs::write(
        tempdir.path().join("Project.toml"),
        "name = \"FixturePkg\"\nversion = \"0.1.0\"\n",
    )
    .unwrap_or_else(|error| panic!("write Project.toml: {error}"));
    fs::write(
        tempdir.path().join("src/FixturePkg.jl"),
        "module FixturePkg\nusing LinearAlgebra\nsolve(x) = x\nend\n",
    )
    .unwrap_or_else(|error| panic!("write Julia source: {error}"));
    commit_all(tempdir.path(), "initial");

    let repository = RegisteredRepository {
        id: "cached-target-file".to_string(),
        path: Some(tempdir.path().to_path_buf()),
        url: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        git_ref: None,
        plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
    };
    let repository_calls = Arc::new(AtomicUsize::new(0));
    let file_calls = Arc::new(AtomicUsize::new(0));
    let mut registry = PluginRegistry::new();
    registry
        .register(CachedTargetFilePlugin {
            repository_calls: Arc::clone(&repository_calls),
            file_calls: Arc::clone(&file_calls),
        })
        .unwrap_or_else(|error| panic!("register cached target-file plugin: {error}"));

    analyze_registered_repository_bundle_with_registry(&repository, tempdir.path(), &registry)
        .unwrap_or_else(|error| panic!("seed cached analysis: {error}"));

    let analysis = analyze_registered_repository_target_file_with_registry(
        &repository,
        tempdir.path(),
        &registry,
        "src/FixturePkg.jl",
    )
    .unwrap_or_else(|error| panic!("target-file analysis should reuse cache: {error}"));

    assert_eq!(repository_calls.load(Ordering::SeqCst), 1);
    assert_eq!(file_calls.load(Ordering::SeqCst), 0);
    assert_eq!(analysis.modules.len(), 1);
    assert_eq!(analysis.modules[0].path, "src/FixturePkg.jl");
    assert_eq!(analysis.symbols.len(), 1);
    assert_eq!(analysis.symbols[0].path, "src/FixturePkg.jl");
    assert_eq!(analysis.imports.len(), 1);
    assert_eq!(analysis.imports[0].import_name, "LinearAlgebra");
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
