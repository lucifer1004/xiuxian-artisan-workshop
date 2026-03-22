use std::path::Path;

use crate::analyzers::cache::{
    ValkeyAnalysisCache, build_repository_analysis_cache_key, load_cached_repository_analysis,
    store_cached_repository_analysis,
};
use crate::analyzers::config::RegisteredRepository;
use crate::analyzers::errors::RepoIntelligenceError;
use crate::analyzers::plugin::{AnalysisContext, PluginLinkContext, RepositoryAnalysisOutput};
use crate::analyzers::registry::PluginRegistry;
use crate::analyzers::skeptic;
use crate::git::checkout::{
    CheckoutSyncMode, ResolvedRepositorySourceKind, discover_checkout_metadata,
    resolve_repository_source,
};

use super::bootstrap::bootstrap_builtin_registry;
use super::merge::{hydrate_repository_record, merge_repository_analysis};
use super::registry::load_registered_repository;
use super::relation_dedupe::dedupe_relations;

/// Analyze one repository from configuration into normalized records.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when config loading or repository
/// analysis fails.
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
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when config loading or repository
/// analysis fails.
pub fn analyze_repository_from_config(
    repo_id: &str,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    analyze_repository_from_config_with_registry(repo_id, config_path, cwd, &registry)
}

/// Analyze one already-resolved registered repository.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
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
    if let Some(ref cache) = valkey_cache
        && let Some(cached) = cache.get(repository, revision)?
    {
        store_cached_repository_analysis(cache_key, &cached)?;
        return Ok(cached);
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
            symbol.verification_state.clone_from(&Some(state.clone()));
        }
    }

    if let Some(ref cache) = valkey_cache {
        cache.set(repository, revision, output.clone())?;
    }
    store_cached_repository_analysis(cache_key, &output)?;

    Ok(output)
}

/// Analyze one already-resolved registered repository.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn analyze_registered_repository(
    repository: &RegisteredRepository,
    cwd: &Path,
) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    analyze_registered_repository_with_registry(repository, cwd, &registry)
}
