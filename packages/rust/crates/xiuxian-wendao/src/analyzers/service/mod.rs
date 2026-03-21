//! High-level repository intelligence service orchestration.

use std::collections::BTreeSet;
use std::path::Path;

use crate::analyzers::cache::{
    ValkeyAnalysisCache, build_repository_analysis_cache_key, load_cached_repository_analysis,
    store_cached_repository_analysis,
};
use crate::analyzers::config::{RegisteredRepository, load_repo_intelligence_config};
use crate::analyzers::errors::RepoIntelligenceError;
#[cfg(feature = "julia")]
use crate::analyzers::languages::register_julia_plugin;
use crate::analyzers::plugin::{AnalysisContext, PluginLinkContext, RepositoryAnalysisOutput};
use crate::analyzers::records::{RelationKind, RelationRecord, RepositoryRecord};
use crate::analyzers::registry::PluginRegistry;
use crate::analyzers::skeptic;
use crate::git::checkout::{
    CheckoutSyncMode, LocalCheckoutMetadata, ResolvedRepositorySourceKind,
    discover_checkout_metadata, resolve_repository_source,
};

mod helpers;
mod projection;
mod search;
mod sync;

/// Returns [`RepoIntelligenceError`] if a built-in plugin cannot be registered.
pub fn bootstrap_builtin_registry() -> Result<PluginRegistry, RepoIntelligenceError> {
    #[allow(unused_mut)]
    let mut registry = PluginRegistry::new();

    #[cfg(feature = "julia")]
    {
        register_julia_plugin(&mut registry)?;
    }

    Ok(registry)
}

/// Load one registered repository from configuration by id.
pub fn load_registered_repository(
    repo_id: &str,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RegisteredRepository, RepoIntelligenceError> {
    let config = load_repo_intelligence_config(config_path, cwd)?;
    config
        .repos
        .into_iter()
        .find(|repository| repository.id == repo_id)
        .ok_or_else(|| RepoIntelligenceError::UnknownRepository {
            repo_id: repo_id.to_string(),
        })
}

/// Analyze one repository from configuration into normalized records.
pub fn analyze_repository_from_config_with_registry(
    repo_id: &str,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
    let repository = load_registered_repository(repo_id, config_path, cwd)?;
    analyze_registered_repository_with_registry(&repository, cwd, registry)
}

/// Analyze one repository from configuration into normalized records.
pub fn analyze_repository_from_config(
    repo_id: &str,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    analyze_repository_from_config_with_registry(repo_id, config_path, cwd, &registry)
}

/// Analyze one already-resolved registered repository.
pub fn analyze_registered_repository_with_registry(
    repository: &RegisteredRepository,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
    let status_source = resolve_repository_source(repository, cwd, CheckoutSyncMode::Status)?;
    let repository_source = if matches!(
        status_source.source_kind,
        ResolvedRepositorySourceKind::ManagedRemote
    ) || !status_source.checkout_root.is_dir()
    {
        resolve_repository_source(repository, cwd, CheckoutSyncMode::Ensure)?
    } else {
        status_source
    };
    let repository_root = repository_source.checkout_root.clone();
    let analysis_context = AnalysisContext {
        repository: repository.clone(),
        repository_root: repository_root.clone(),
    };
    for plugin in registry.resolve_for_repository(repository)? {
        plugin.preflight_repository(&analysis_context, repository_root.as_path())?;
    }
    let checkout_metadata = discover_checkout_metadata(repository_root.as_path());
    let cache_key = build_repository_analysis_cache_key(
        repository,
        &repository_source,
        checkout_metadata.as_ref(),
    );
    if let Some(cached) = load_cached_repository_analysis(&cache_key)? {
        return Ok(cached);
    }

    let revision = cache_key
        .checkout_revision
        .as_deref()
        .or(cache_key.mirror_revision.as_deref())
        .or(cache_key.tracking_revision.as_deref())
        .unwrap_or("unknown");

    let valkey_cache = ValkeyAnalysisCache::new()?;
    if let Some(ref cache) = valkey_cache {
        if let Some(cached) = cache.get(repository, revision)? {
            store_cached_repository_analysis(cache_key, &cached)?;
            return Ok(cached);
        }
    }

    if repository.plugins.is_empty() {
        return Err(RepoIntelligenceError::MissingRequiredPlugin {
            repo_id: repository.id.clone(),
            plugin_id: "any".to_string(),
        });
    }

    let plugins = registry.resolve_for_repository(repository)?;
    let mut output = RepositoryAnalysisOutput::default();
    let mut any_plugin_output = false;

    for plugin in plugins {
        let plugin_output =
            plugin.analyze_repository(&analysis_context, repository_root.as_path())?;
        any_plugin_output = true;
        merge_repository_analysis(&mut output, plugin_output);
    }

    if !any_plugin_output {
        return Err(RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "repo `{}` produced no repository analysis output",
                repository.id
            ),
        });
    }

    let link_context = PluginLinkContext {
        repository: repository.clone(),
        repository_root: repository_root.clone(),
        modules: output.modules.clone(),
        symbols: output.symbols.clone(),
        examples: output.examples.clone(),
        docs: output.docs.clone(),
    };
    for plugin in registry.resolve_for_repository(repository)? {
        output
            .relations
            .extend(plugin.enrich_relations(&link_context)?);
    }
    dedupe_relations(&mut output.relations);

    if output.repository.is_none() {
        output.repository = Some(repository.into());
    }
    if let Some(record) = output.repository.as_mut() {
        hydrate_repository_record(
            record,
            repository,
            repository_root.as_path(),
            checkout_metadata.as_ref(),
        );
    }

    let audit_results = skeptic::audit_symbols(&output.symbols, &output.docs, &output.relations);
    for symbol in &mut output.symbols {
        if let Some(state) = audit_results.get(&symbol.symbol_id) {
            symbol.verification_state = Some(state.clone());
        }
    }

    if let Some(ref cache) = valkey_cache {
        cache.set(repository, revision, output.clone())?;
    }
    store_cached_repository_analysis(cache_key, &output)?;

    Ok(output)
}

/// Load repository analysis from ready caches only.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::PendingRepositoryIndex`] when no ready cache exists yet.
pub fn analyze_registered_repository_cached_with_registry(
    repository: &RegisteredRepository,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
    if repository.plugins.is_empty() {
        return Err(RepoIntelligenceError::MissingRequiredPlugin {
            repo_id: repository.id.clone(),
            plugin_id: "any".to_string(),
        });
    }

    let repository_source = resolve_repository_source(repository, cwd, CheckoutSyncMode::Status)?;
    let repository_root = repository_source.checkout_root.clone();
    let analysis_context = AnalysisContext {
        repository: repository.clone(),
        repository_root: repository_root.clone(),
    };
    for plugin in registry.resolve_for_repository(repository)? {
        plugin.preflight_repository(&analysis_context, repository_root.as_path())?;
    }

    let checkout_metadata = discover_checkout_metadata(repository_root.as_path());
    let cache_key = build_repository_analysis_cache_key(
        repository,
        &repository_source,
        checkout_metadata.as_ref(),
    );
    if let Some(cached) = load_cached_repository_analysis(&cache_key)? {
        return Ok(cached);
    }

    let revision = cache_key
        .checkout_revision
        .as_deref()
        .or(cache_key.mirror_revision.as_deref())
        .or(cache_key.tracking_revision.as_deref());
    if let Some(revision) = revision {
        let valkey_cache = ValkeyAnalysisCache::new()?;
        if let Some(ref cache) = valkey_cache
            && let Some(cached) = cache.get(repository, revision)?
        {
            store_cached_repository_analysis(cache_key, &cached)?;
            return Ok(cached);
        }
    }

    Err(RepoIntelligenceError::PendingRepositoryIndex {
        repo_id: repository.id.clone(),
    })
}

/// Analyze one already-resolved registered repository.
pub fn analyze_registered_repository(
    repository: &RegisteredRepository,
    cwd: &Path,
) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    analyze_registered_repository_with_registry(repository, cwd, &registry)
}

fn merge_repository_analysis(
    base: &mut RepositoryAnalysisOutput,
    mut overlay: RepositoryAnalysisOutput,
) {
    match (base.repository.take(), overlay.repository.take()) {
        (None, None) => {}
        (Some(base_record), None) => {
            base.repository = Some(base_record);
        }
        (None, Some(overlay_record)) => {
            base.repository = Some(overlay_record);
        }
        (Some(base_record), Some(overlay_record)) => {
            base.repository = Some(merge_repository_record(base_record, overlay_record));
        }
    }
    base.modules.append(&mut overlay.modules);
    base.symbols.append(&mut overlay.symbols);
    base.imports.append(&mut overlay.imports);
    base.examples.append(&mut overlay.examples);
    base.docs.append(&mut overlay.docs);
    base.relations.append(&mut overlay.relations);
    base.diagnostics.append(&mut overlay.diagnostics);
}

fn merge_repository_record(base: RepositoryRecord, overlay: RepositoryRecord) -> RepositoryRecord {
    RepositoryRecord {
        repo_id: if overlay.repo_id.is_empty() {
            base.repo_id
        } else {
            overlay.repo_id
        },
        name: if overlay.name.is_empty() {
            base.name
        } else {
            overlay.name
        },
        path: if overlay.path.is_empty() {
            base.path
        } else {
            overlay.path
        },
        url: overlay.url.or(base.url),
        revision: overlay.revision.or(base.revision),
        version: overlay.version.or(base.version),
        uuid: overlay.uuid.or(base.uuid),
        dependencies: if overlay.dependencies.is_empty() {
            base.dependencies
        } else {
            overlay.dependencies
        },
    }
}

fn hydrate_repository_record(
    record: &mut RepositoryRecord,
    repository: &RegisteredRepository,
    repository_root: &Path,
    checkout_metadata: Option<&LocalCheckoutMetadata>,
) {
    if record.repo_id.trim().is_empty() {
        record.repo_id = repository.id.clone();
    }
    if record.name.trim().is_empty() {
        record.name = repository.id.clone();
    }
    if record.path.trim().is_empty() {
        record.path = repository_root.display().to_string();
    }
    if record.url.is_none() {
        record.url = repository
            .url
            .clone()
            .or_else(|| checkout_metadata.and_then(|metadata| metadata.remote_url.clone()));
    }
    if record.revision.is_none() {
        record.revision = checkout_metadata.and_then(|metadata| metadata.revision.clone());
    }
}

fn dedupe_relations(relations: &mut Vec<RelationRecord>) {
    let mut seen = BTreeSet::new();
    relations.retain(|relation| {
        seen.insert((
            relation.repo_id.clone(),
            relation.source_id.clone(),
            relation.target_id.clone(),
            relation_kind_key(relation.kind),
        ))
    });
}

fn relation_kind_key(kind: RelationKind) -> &'static str {
    match kind {
        RelationKind::Contains => "contains",
        RelationKind::Calls => "calls",
        RelationKind::Uses => "uses",
        RelationKind::Documents => "documents",
        RelationKind::ExampleOf => "example_of",
        RelationKind::Declares => "declares",
        RelationKind::Implements => "implements",
        RelationKind::Imports => "imports",
    }
}

pub use helpers::relation_kind_label;
pub use projection::*;
pub use search::*;
pub use sync::*;

#[cfg(test)]
mod refinement_tests {
    use std::path::Path;

    use crate::analyzers::config::{RegisteredRepository, RepositoryRefreshPolicy};
    use crate::analyzers::query::{RefineEntityDocRequest, RefineEntityDocResponse};
    use crate::analyzers::records::RepositoryRecord;
    use crate::git::checkout::LocalCheckoutMetadata;

    use super::{hydrate_repository_record, merge_repository_record};

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
}
