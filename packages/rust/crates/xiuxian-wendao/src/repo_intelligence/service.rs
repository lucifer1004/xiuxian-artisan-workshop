use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use chrono::Utc;

use super::checkout::{
    LocalCheckoutMetadata, RepositoryLifecycleState, RepositorySyncMode as CheckoutSyncMode,
    ResolvedRepositorySource, ResolvedRepositorySourceKind, discover_checkout_metadata,
    resolve_repository_source,
};
use super::config::{RegisteredRepository, load_repo_intelligence_config};
use super::errors::RepoIntelligenceError;
use super::julia::JuliaRepoIntelligencePlugin;
use super::modelica::ModelicaRepoIntelligencePlugin;
use super::plugin::{AnalysisContext, PluginLinkContext, RepositoryAnalysisOutput};
use super::projection::{
    build_projected_page, build_projected_page_family_cluster, build_projected_page_family_context,
    build_projected_page_family_search, build_projected_page_index_node,
    build_projected_page_index_tree, build_projected_page_index_tree_search,
    build_projected_page_index_trees, build_projected_page_navigation,
    build_projected_page_navigation_search, build_projected_page_search, build_projected_pages,
    build_projected_retrieval, build_projected_retrieval_context, build_projected_retrieval_hit,
};
use super::query::{
    DocCoverageQuery, DocCoverageResult, ExampleSearchHit, ExampleSearchQuery, ExampleSearchResult,
    ModuleSearchHit, ModuleSearchQuery, ModuleSearchResult, RepoOverviewQuery, RepoOverviewResult,
    RepoBacklinkItem,
    RepoProjectedPageFamilyClusterQuery, RepoProjectedPageFamilyClusterResult,
    RepoProjectedPageFamilyContextQuery, RepoProjectedPageFamilyContextResult,
    RepoProjectedPageFamilySearchQuery, RepoProjectedPageFamilySearchResult,
    RepoProjectedPageIndexNodeQuery, RepoProjectedPageIndexNodeResult,
    RepoProjectedPageIndexTreeQuery, RepoProjectedPageIndexTreeResult,
    RepoProjectedPageIndexTreeSearchQuery, RepoProjectedPageIndexTreeSearchResult,
    RepoProjectedPageIndexTreesQuery, RepoProjectedPageIndexTreesResult,
    RepoProjectedPageNavigationQuery, RepoProjectedPageNavigationResult,
    RepoProjectedPageNavigationSearchQuery, RepoProjectedPageNavigationSearchResult,
    RepoProjectedPageQuery, RepoProjectedPageResult, RepoProjectedPageSearchQuery,
    RepoProjectedPageSearchResult, RepoProjectedPagesQuery, RepoProjectedPagesResult,
    RepoProjectedRetrievalContextQuery, RepoProjectedRetrievalContextResult,
    RepoProjectedRetrievalHitQuery, RepoProjectedRetrievalHitResult, RepoProjectedRetrievalQuery,
    RepoProjectedRetrievalResult, RepoSourceKind, RepoSyncDriftState, RepoSyncFreshnessSummary,
    RepoSyncHealthState, RepoSyncLifecycleSummary, RepoSyncMode, RepoSyncQuery, RepoSyncResult,
    RepoSyncRevisionSummary, RepoSyncStalenessState, RepoSyncState, RepoSyncStatusSummary,
    SymbolSearchHit, SymbolSearchQuery, SymbolSearchResult,
};
use super::records::{DocRecord, ModuleRecord, RelationKind, RelationRecord, SymbolRecord};
use super::registry::PluginRegistry;

/// Build a plugin registry with all built-in Repo Intelligence analyzers.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] if a built-in plugin cannot be registered.
pub fn bootstrap_builtin_registry() -> Result<PluginRegistry, RepoIntelligenceError> {
    let mut registry = PluginRegistry::new();
    registry.register(JuliaRepoIntelligencePlugin::default())?;
    registry.register(ModelicaRepoIntelligencePlugin)?;
    Ok(registry)
}

/// Load one registered repository from configuration by id.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when configuration loading fails or the
/// repository id is not registered.
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
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when configuration loading, plugin
/// resolution, or repository analysis fails.
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
/// Returns [`RepoIntelligenceError`] when configuration loading, plugin
/// resolution, or repository analysis fails.
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
/// Returns [`RepoIntelligenceError`] when plugin resolution or repository analysis fails.
pub fn analyze_registered_repository_with_registry(
    repository: &RegisteredRepository,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
    let repository_source = resolve_repository_source(&repository, cwd, CheckoutSyncMode::Ensure)?;
    let repository_root = repository_source.checkout_root.clone();
    if repository.plugins.is_empty() {
        return Err(RepoIntelligenceError::MissingRequiredPlugin {
            repo_id: repository.id.clone(),
            plugin_id: "julia".to_string(),
        });
    }

    let plugins = registry.resolve_for_repository(&repository)?;
    if plugins.is_empty() {
        return Err(RepoIntelligenceError::MissingRequiredPlugin {
            repo_id: repository.id.clone(),
            plugin_id: "julia".to_string(),
        });
    }

    let context = AnalysisContext {
        repository: repository.clone(),
        repository_root: repository_root.clone(),
    };

    let mut merged = RepositoryAnalysisOutput::default();
    for plugin in &plugins {
        let output = plugin.analyze_repository(&context, &repository_root)?;
        merge_repository_analysis(&mut merged, output);
    }

    if merged.repository.is_none() {
        return Err(RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "repo `{}` produced no repository record during analysis",
                repository.id
            ),
        });
    }

    let link_context = PluginLinkContext {
        repository: repository.clone(),
        repository_root: repository_root.clone(),
        modules: merged.modules.clone(),
        symbols: merged.symbols.clone(),
        examples: merged.examples.clone(),
        docs: merged.docs.clone(),
    };
    for plugin in &plugins {
        merged
            .relations
            .extend(plugin.enrich_relations(&link_context)?);
    }
    dedupe_relations(&mut merged.relations);

    Ok(merged)
}

/// Analyze one already-resolved registered repository.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when plugin resolution or repository analysis fails.
pub fn analyze_registered_repository(
    repository: &RegisteredRepository,
    cwd: &Path,
) -> Result<RepositoryAnalysisOutput, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    analyze_registered_repository_with_registry(repository, cwd, &registry)
}

/// Build a repository synchronization result from resolved source state.
#[must_use]
pub(crate) fn build_repo_sync(
    query: &RepoSyncQuery,
    repository: &RegisteredRepository,
    source: &ResolvedRepositorySource,
    metadata: Option<LocalCheckoutMetadata>,
) -> RepoSyncResult {
    let metadata = metadata.unwrap_or_default();
    let checked_at = Utc::now();
    let source_kind = match source.source_kind {
        ResolvedRepositorySourceKind::LocalCheckout => RepoSourceKind::LocalCheckout,
        ResolvedRepositorySourceKind::ManagedRemote => RepoSourceKind::ManagedRemote,
    };
    let mirror_state = repo_sync_state(source.mirror_state);
    let checkout_state = repo_sync_state(source.checkout_state);
    let checked_at_string = checked_at.to_rfc3339();
    let last_fetched_at = source.last_fetched_at.clone();
    let mirror_revision = source.mirror_revision.clone();
    let tracking_revision = source.tracking_revision.clone();
    let upstream_url = repository.url.clone().or(metadata.remote_url);
    let drift_state = source.drift_state;
    let health_state = repo_sync_health_state(source);
    let staleness_state = repo_sync_staleness_state(source, checked_at);
    let revision = metadata.revision;
    let status_summary = repo_sync_status_summary(
        source_kind,
        mirror_state,
        checkout_state,
        checked_at_string.as_str(),
        last_fetched_at.as_deref(),
        mirror_revision.as_deref(),
        tracking_revision.as_deref(),
        drift_state,
        health_state,
        staleness_state,
        revision.as_deref(),
    );

    RepoSyncResult {
        repo_id: query.repo_id.clone(),
        mode: query.mode,
        source_kind,
        refresh: repository.refresh,
        mirror_state,
        checkout_state,
        checkout_path: source.checkout_root.display().to_string(),
        mirror_path: source
            .mirror_root
            .as_ref()
            .map(|path| path.display().to_string()),
        checked_at: checked_at_string,
        last_fetched_at,
        mirror_revision,
        tracking_revision,
        upstream_url,
        drift_state,
        health_state,
        staleness_state,
        status_summary,
        revision,
    }
}

/// Load configuration, synchronize one repository source, and return source state.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when configuration loading or repository
/// source preparation fails.
pub fn repo_sync_from_config(
    query: &RepoSyncQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoSyncResult, RepoIntelligenceError> {
    let repository = load_registered_repository(&query.repo_id, config_path, cwd)?;
    repo_sync_for_registered_repository(query, &repository, cwd)
}

/// Synchronize one already-resolved registered repository and return source state.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository source preparation fails.
pub fn repo_sync_for_registered_repository(
    query: &RepoSyncQuery,
    repository: &RegisteredRepository,
    cwd: &Path,
) -> Result<RepoSyncResult, RepoIntelligenceError> {
    let source = resolve_repository_source(repository, cwd, checkout_sync_mode(query.mode))?;
    let metadata = discover_checkout_metadata(&source.checkout_root);
    Ok(build_repo_sync(query, repository, &source, metadata))
}

fn checkout_sync_mode(mode: RepoSyncMode) -> CheckoutSyncMode {
    match mode {
        RepoSyncMode::Ensure => CheckoutSyncMode::Ensure,
        RepoSyncMode::Refresh => CheckoutSyncMode::Refresh,
        RepoSyncMode::Status => CheckoutSyncMode::Status,
    }
}

fn repo_sync_state(state: RepositoryLifecycleState) -> RepoSyncState {
    match state {
        RepositoryLifecycleState::NotApplicable => RepoSyncState::NotApplicable,
        RepositoryLifecycleState::Missing => RepoSyncState::Missing,
        RepositoryLifecycleState::Validated => RepoSyncState::Validated,
        RepositoryLifecycleState::Observed => RepoSyncState::Observed,
        RepositoryLifecycleState::Created => RepoSyncState::Created,
        RepositoryLifecycleState::Reused => RepoSyncState::Reused,
        RepositoryLifecycleState::Refreshed => RepoSyncState::Refreshed,
    }
}

fn repo_sync_health_state(source: &ResolvedRepositorySource) -> RepoSyncHealthState {
    match source.source_kind {
        ResolvedRepositorySourceKind::LocalCheckout => RepoSyncHealthState::Healthy,
        ResolvedRepositorySourceKind::ManagedRemote => {
            if matches!(source.mirror_state, RepositoryLifecycleState::Missing)
                || matches!(source.checkout_state, RepositoryLifecycleState::Missing)
            {
                return RepoSyncHealthState::MissingAssets;
            }

            match source.drift_state {
                RepoSyncDriftState::NotApplicable | RepoSyncDriftState::InSync => {
                    RepoSyncHealthState::Healthy
                }
                RepoSyncDriftState::Ahead => RepoSyncHealthState::HasLocalCommits,
                RepoSyncDriftState::Behind => RepoSyncHealthState::NeedsRefresh,
                RepoSyncDriftState::Diverged => RepoSyncHealthState::Diverged,
                RepoSyncDriftState::Unknown => RepoSyncHealthState::Unknown,
            }
        }
    }
}

fn repo_sync_staleness_state(
    source: &ResolvedRepositorySource,
    checked_at: chrono::DateTime<Utc>,
) -> RepoSyncStalenessState {
    match source.source_kind {
        ResolvedRepositorySourceKind::LocalCheckout => RepoSyncStalenessState::NotApplicable,
        ResolvedRepositorySourceKind::ManagedRemote => {
            let Some(last_fetched_at) = source.last_fetched_at.as_deref() else {
                return RepoSyncStalenessState::Unknown;
            };
            let Ok(last_fetched_at) = chrono::DateTime::parse_from_rfc3339(last_fetched_at) else {
                return RepoSyncStalenessState::Unknown;
            };
            let age = checked_at.signed_duration_since(last_fetched_at.with_timezone(&Utc));
            if age < chrono::Duration::zero() {
                return RepoSyncStalenessState::Unknown;
            }
            if age < chrono::Duration::hours(1) {
                RepoSyncStalenessState::Fresh
            } else if age < chrono::Duration::hours(24) {
                RepoSyncStalenessState::Aging
            } else {
                RepoSyncStalenessState::Stale
            }
        }
    }
}

fn repo_sync_status_summary(
    source_kind: RepoSourceKind,
    mirror_state: RepoSyncState,
    checkout_state: RepoSyncState,
    checked_at: &str,
    last_fetched_at: Option<&str>,
    mirror_revision: Option<&str>,
    tracking_revision: Option<&str>,
    drift_state: RepoSyncDriftState,
    health_state: RepoSyncHealthState,
    staleness_state: RepoSyncStalenessState,
    checkout_revision: Option<&str>,
) -> RepoSyncStatusSummary {
    let lifecycle = RepoSyncLifecycleSummary {
        source_kind,
        mirror_state,
        checkout_state,
        mirror_ready: !matches!(
            mirror_state,
            RepoSyncState::Missing | RepoSyncState::NotApplicable
        ),
        checkout_ready: !matches!(checkout_state, RepoSyncState::Missing),
    };
    let freshness = RepoSyncFreshnessSummary {
        checked_at: checked_at.to_string(),
        last_fetched_at: last_fetched_at.map(str::to_string),
        staleness_state,
    };
    let revisions = RepoSyncRevisionSummary {
        checkout_revision: checkout_revision.map(str::to_string),
        mirror_revision: mirror_revision.map(str::to_string),
        tracking_revision: tracking_revision.map(str::to_string),
        aligned_with_mirror: checkout_revision.is_some() && checkout_revision == mirror_revision,
    };

    RepoSyncStatusSummary {
        lifecycle,
        freshness,
        revisions,
        health_state,
        drift_state,
        attention_required: !matches!(health_state, RepoSyncHealthState::Healthy)
            || matches!(
                staleness_state,
                RepoSyncStalenessState::Stale | RepoSyncStalenessState::Unknown
            ),
    }
}

/// Build a repository overview result from normalized analysis records.
#[must_use]
pub fn build_repo_overview(
    query: &RepoOverviewQuery,
    analysis: &RepositoryAnalysisOutput,
) -> RepoOverviewResult {
    let repository = analysis
        .repository
        .as_ref()
        .expect("repository record is required");
    RepoOverviewResult {
        repo_id: query.repo_id.clone(),
        display_name: repository.name.clone(),
        revision: repository.revision.clone(),
        module_count: analysis.modules.len(),
        symbol_count: analysis.symbols.len(),
        example_count: analysis.examples.len(),
        doc_count: analysis.docs.len(),
        hierarchical_uri: Some(repo_hierarchical_uri(query.repo_id.as_str())),
        hierarchy: Some(vec!["repo".to_string(), query.repo_id.clone()]),
    }
}

/// Load configuration, analyze one repository, and return its overview.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_overview_from_config_with_registry(
    query: &RepoOverviewQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoOverviewResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_repo_overview(query, &analysis))
}

/// Load configuration, analyze one repository, and return its overview.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_overview_from_config(
    query: &RepoOverviewQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoOverviewResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_overview_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build a module search result from normalized analysis records.
#[must_use]
pub fn build_module_search(
    query: &ModuleSearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> ModuleSearchResult {
    let normalized_query = query.query.trim().to_ascii_lowercase();
    let limit = query.limit.max(1);
    let backlink_lookup = documents_backlink_lookup(&analysis.relations, &analysis.docs);
    let projection_lookup = projection_page_lookup(analysis);
    let mut matches = analysis
        .modules
        .iter()
        .filter_map(|module| {
            let qualified_name = module.qualified_name.to_ascii_lowercase();
            let path = module.path.to_ascii_lowercase();
            let score = module_match_score(
                normalized_query.as_str(),
                qualified_name.as_str(),
                path.as_str(),
            )?;
            Some((score, module.clone()))
        })
        .collect::<Vec<_>>();

    matches.sort_by(|(left_score, _left_module), (right_score, _right_module)| {
        left_score.cmp(right_score)
    });

    let selected = matches.into_iter().take(limit).collect::<Vec<_>>();
    let modules = selected
        .iter()
        .map(|(_score, module)| module.clone())
        .collect::<Vec<_>>();
    let module_hits = selected
        .into_iter()
        .enumerate()
        .map(|(index, (raw_score, module))| {
            let normalized_score = normalized_rank_score(raw_score, 3);
            let module_id = module.module_id.clone();
            let module_path = module.path.clone();
            let (implicit_backlinks, implicit_backlink_items) =
                backlinks_for(module_id.as_str(), &backlink_lookup);
            ModuleSearchHit {
                module,
                score: Some(normalized_score),
                rank: Some(index + 1),
                saliency_score: Some(normalized_score),
                hierarchical_uri: Some(record_hierarchical_uri(
                    query.repo_id.as_str(),
                    "module",
                    module_id.as_str(),
                )),
                hierarchy: hierarchy_segments_from_path(module_path.as_str()),
                implicit_backlinks,
                implicit_backlink_items,
                projection_page_ids: projection_pages_for(module_id.as_str(), &projection_lookup),
            }
        })
        .collect::<Vec<_>>();

    ModuleSearchResult {
        repo_id: query.repo_id.clone(),
        modules,
        module_hits,
    }
}

/// Load configuration, analyze one repository, and return matching modules.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn module_search_from_config_with_registry(
    query: &ModuleSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<ModuleSearchResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_module_search(query, &analysis))
}

/// Load configuration, analyze one repository, and return matching modules.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn module_search_from_config(
    query: &ModuleSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<ModuleSearchResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    module_search_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build a symbol search result from normalized analysis records.
#[must_use]
pub fn build_symbol_search(
    query: &SymbolSearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> SymbolSearchResult {
    let normalized_query = query.query.trim().to_ascii_lowercase();
    let limit = query.limit.max(1);
    let backlink_lookup = documents_backlink_lookup(&analysis.relations, &analysis.docs);
    let projection_lookup = projection_page_lookup(analysis);
    let mut matches = analysis
        .symbols
        .iter()
        .filter_map(|symbol| {
            let name = symbol.name.to_ascii_lowercase();
            let qualified_name = symbol.qualified_name.to_ascii_lowercase();
            let path = symbol.path.to_ascii_lowercase();
            let signature = symbol
                .signature
                .as_deref()
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();
            let score = symbol_match_score(
                normalized_query.as_str(),
                name.as_str(),
                qualified_name.as_str(),
                path.as_str(),
                signature.as_str(),
            )?;
            Some((score, symbol.clone()))
        })
        .collect::<Vec<_>>();

    matches.sort_by(|(left_score, left_symbol), (right_score, right_symbol)| {
        left_score
            .cmp(right_score)
            .then_with(|| left_symbol.name.cmp(&right_symbol.name))
            .then_with(|| left_symbol.qualified_name.cmp(&right_symbol.qualified_name))
            .then_with(|| left_symbol.path.cmp(&right_symbol.path))
    });

    let selected = matches.into_iter().take(limit).collect::<Vec<_>>();
    let symbols = selected
        .iter()
        .map(|(_score, symbol)| symbol.clone())
        .collect::<Vec<_>>();
    let symbol_hits = selected
        .into_iter()
        .enumerate()
        .map(|(index, (raw_score, symbol))| {
            let normalized_score = normalized_rank_score(raw_score, 7);
            let audit_status = symbol.audit_status.clone();
            let verification_state = audit_status.as_deref().map(|status| match status {
                "verified" => "verified".to_string(),
                "approved" => "verified".to_string(),
                _ => "unverified".to_string(),
            });
            let symbol_id = symbol.symbol_id.clone();
            let symbol_path = symbol.path.clone();
            let (implicit_backlinks, implicit_backlink_items) =
                backlinks_for(symbol_id.as_str(), &backlink_lookup);
            SymbolSearchHit {
                symbol,
                score: Some(normalized_score),
                rank: Some(index + 1),
                saliency_score: Some(normalized_score),
                hierarchical_uri: Some(record_hierarchical_uri(
                    query.repo_id.as_str(),
                    "symbol",
                    symbol_id.as_str(),
                )),
                hierarchy: hierarchy_segments_from_path(symbol_path.as_str()),
                implicit_backlinks,
                implicit_backlink_items,
                projection_page_ids: projection_pages_for(symbol_id.as_str(), &projection_lookup),
                audit_status,
                verification_state,
            }
        })
        .collect::<Vec<_>>();

    SymbolSearchResult {
        repo_id: query.repo_id.clone(),
        symbols,
        symbol_hits,
    }
}

/// Load configuration, analyze one repository, and return matching symbols.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn symbol_search_from_config_with_registry(
    query: &SymbolSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<SymbolSearchResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_symbol_search(query, &analysis))
}

/// Load configuration, analyze one repository, and return matching symbols.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn symbol_search_from_config(
    query: &SymbolSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<SymbolSearchResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    symbol_search_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build an example search result from normalized analysis records.
#[must_use]
pub fn build_example_search(
    query: &ExampleSearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> ExampleSearchResult {
    let normalized_query = query.query.trim().to_ascii_lowercase();
    let limit = query.limit.max(1);
    let backlink_lookup = documents_backlink_lookup(&analysis.relations, &analysis.docs);
    let projection_lookup = projection_page_lookup(analysis);
    let relation_lookup = example_relation_lookup(&analysis.relations);
    let mut matches = analysis
        .examples
        .iter()
        .filter_map(|example| {
            let title = example.title.to_ascii_lowercase();
            let path = example.path.to_ascii_lowercase();
            let summary = example
                .summary
                .as_deref()
                .map(str::to_ascii_lowercase)
                .unwrap_or_default();
            let related_symbols = related_symbols_for_example(
                example.example_id.as_str(),
                &relation_lookup,
                &analysis.symbols,
            );
            let related_modules = related_modules_for_example(
                example.example_id.as_str(),
                &relation_lookup,
                &analysis.modules,
            );
            let score = example_match_score(
                normalized_query.as_str(),
                title.as_str(),
                path.as_str(),
                summary.as_str(),
                &related_symbols,
                &related_modules,
            )?;
            Some((score, example.clone()))
        })
        .collect::<Vec<_>>();

    matches.sort_by(
        |(left_score, _left_example), (right_score, _right_example)| left_score.cmp(right_score),
    );

    let selected = matches.into_iter().take(limit).collect::<Vec<_>>();
    let examples = selected
        .iter()
        .map(|(_score, example)| example.clone())
        .collect::<Vec<_>>();
    let example_hits = selected
        .into_iter()
        .enumerate()
        .map(|(index, (raw_score, example))| {
            let normalized_score = normalized_rank_score(raw_score, 10);
            let example_id = example.example_id.clone();
            let example_path = example.path.clone();
            let (implicit_backlinks, implicit_backlink_items) =
                backlinks_for(example_id.as_str(), &backlink_lookup);
            ExampleSearchHit {
                example,
                score: Some(normalized_score),
                rank: Some(index + 1),
                saliency_score: Some(normalized_score),
                hierarchical_uri: Some(record_hierarchical_uri(
                    query.repo_id.as_str(),
                    "example",
                    example_id.as_str(),
                )),
                hierarchy: hierarchy_segments_from_path(example_path.as_str()),
                implicit_backlinks,
                implicit_backlink_items,
                projection_page_ids: projection_pages_for(example_id.as_str(), &projection_lookup),
            }
        })
        .collect::<Vec<_>>();

    ExampleSearchResult {
        repo_id: query.repo_id.clone(),
        examples,
        example_hits,
    }
}

/// Load configuration, analyze one repository, and return matching examples.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn example_search_from_config_with_registry(
    query: &ExampleSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<ExampleSearchResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_example_search(query, &analysis))
}

/// Load configuration, analyze one repository, and return matching examples.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn example_search_from_config(
    query: &ExampleSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<ExampleSearchResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    example_search_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build a documentation coverage result from normalized analysis records.
#[must_use]
pub fn build_doc_coverage(
    query: &DocCoverageQuery,
    analysis: &RepositoryAnalysisOutput,
) -> DocCoverageResult {
    let scoped_module = resolve_module_scope(query.module_id.as_deref(), &analysis.modules);
    let scoped_docs = docs_in_scope(scoped_module, analysis);
    let scoped_symbols = symbols_in_scope(scoped_module, &analysis.symbols);
    let covered_symbol_ids =
        documented_symbol_ids(scoped_module, &analysis.symbols, &analysis.relations);
    let covered_symbols = scoped_symbols
        .iter()
        .filter(|symbol| covered_symbol_ids.contains(symbol.symbol_id.as_str()))
        .count();

    DocCoverageResult {
        repo_id: query.repo_id.clone(),
        module_id: scoped_module
            .map(|module| module.module_id.clone())
            .or_else(|| query.module_id.clone()),
        docs: scoped_docs,
        covered_symbols,
        uncovered_symbols: scoped_symbols.len().saturating_sub(covered_symbols),
        hierarchical_uri: Some(repo_hierarchical_uri(query.repo_id.as_str())),
        hierarchy: Some(vec!["repo".to_string(), query.repo_id.clone()]),
    }
}

/// Load configuration, analyze one repository, and return documentation coverage.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn doc_coverage_from_config_with_registry(
    query: &DocCoverageQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<DocCoverageResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_doc_coverage(query, &analysis))
}

/// Load configuration, analyze one repository, and return documentation coverage.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn doc_coverage_from_config(
    query: &DocCoverageQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<DocCoverageResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    doc_coverage_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build deterministic projected pages from normalized analysis records.
#[must_use]
pub fn build_repo_projected_pages(
    query: &RepoProjectedPagesQuery,
    analysis: &RepositoryAnalysisOutput,
) -> RepoProjectedPagesResult {
    RepoProjectedPagesResult {
        repo_id: query.repo_id.clone(),
        pages: build_projected_pages(analysis),
    }
}

/// Load configuration, analyze one repository, and return deterministic projected pages.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_projected_pages_from_config_with_registry(
    query: &RepoProjectedPagesQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPagesResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_repo_projected_pages(query, &analysis))
}

/// Load configuration, analyze one repository, and return deterministic projected pages.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_projected_pages_from_config(
    query: &RepoProjectedPagesQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPagesResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_pages_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build one deterministic projected page from normalized analysis records.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPage`] when the requested projected page is
/// not present in the analysis output.
pub fn build_repo_projected_page(
    query: &RepoProjectedPageQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageResult, RepoIntelligenceError> {
    build_projected_page(query, analysis)
}

/// Load configuration, analyze one repository, and return one deterministic projected page.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page identifier is not present for the repository.
pub fn repo_projected_page_from_config_with_registry(
    query: &RepoProjectedPageQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_page(query, &analysis)
}

/// Load configuration, analyze one repository, and return one deterministic projected page.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page identifier is not present for the repository.
pub fn repo_projected_page_from_config(
    query: &RepoProjectedPageQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build deterministic page-family context around one stable projected page.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPage`] when the requested projected page is
/// not present in the analysis output.
pub fn build_repo_projected_page_family_context(
    query: &RepoProjectedPageFamilyContextQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageFamilyContextResult, RepoIntelligenceError> {
    build_projected_page_family_context(query, analysis)
}

/// Load configuration, analyze one repository, and return deterministic page-family context.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page identifier is not present for the repository.
pub fn repo_projected_page_family_context_from_config_with_registry(
    query: &RepoProjectedPageFamilyContextQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageFamilyContextResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_page_family_context(query, &analysis)
}

/// Load configuration, analyze one repository, and return deterministic page-family context.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page identifier is not present for the repository.
pub fn repo_projected_page_family_context_from_config(
    query: &RepoProjectedPageFamilyContextQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageFamilyContextResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_family_context_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build one deterministic page-family cluster around one stable projected page.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPage`] when the requested projected page is
/// not present in the analysis output, or
/// [`RepoIntelligenceError::UnknownProjectedPageFamilyCluster`] when the requested family is not
/// present for the projected page.
pub fn build_repo_projected_page_family_cluster(
    query: &RepoProjectedPageFamilyClusterQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageFamilyClusterResult, RepoIntelligenceError> {
    build_projected_page_family_cluster(query, analysis)
}

/// Load configuration, analyze one repository, and return one deterministic page-family cluster.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page or family cluster is not present for the repository.
pub fn repo_projected_page_family_cluster_from_config_with_registry(
    query: &RepoProjectedPageFamilyClusterQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageFamilyClusterResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_page_family_cluster(query, &analysis)
}

/// Load configuration, analyze one repository, and return one deterministic page-family cluster.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page or family cluster is not present for the repository.
pub fn repo_projected_page_family_cluster_from_config(
    query: &RepoProjectedPageFamilyClusterQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageFamilyClusterResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_family_cluster_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build one deterministic page-centric Stage-2 navigation bundle.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPage`] when the requested projected page is
/// not present in the analysis output, [`RepoIntelligenceError::UnknownProjectedPageIndexNode`]
/// when the requested projected page-index node is not present for the projected page, or
/// [`RepoIntelligenceError::UnknownProjectedPageFamilyCluster`] when the requested family is not
/// present for the projected page.
pub fn build_repo_projected_page_navigation(
    query: &RepoProjectedPageNavigationQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageNavigationResult, RepoIntelligenceError> {
    build_projected_page_navigation(query, analysis)
}

/// Load configuration, analyze one repository, and return one deterministic page-centric
/// Stage-2 navigation bundle.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page, node, or family cluster is not present for the repository.
pub fn repo_projected_page_navigation_from_config_with_registry(
    query: &RepoProjectedPageNavigationQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageNavigationResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_page_navigation(query, &analysis)
}

/// Load configuration, analyze one repository, and return one deterministic page-centric
/// Stage-2 navigation bundle.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page, node, or family cluster is not present for the repository.
pub fn repo_projected_page_navigation_from_config(
    query: &RepoProjectedPageNavigationQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageNavigationResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_navigation_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build deterministic projected page-navigation search results from normalized analysis records.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when a matched projected page cannot be expanded into a
/// deterministic navigation bundle.
pub fn build_repo_projected_page_navigation_search(
    query: &RepoProjectedPageNavigationSearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageNavigationSearchResult, RepoIntelligenceError> {
    build_projected_page_navigation_search(query, analysis)
}

/// Load configuration, analyze one repository, and return deterministic projected page-navigation
/// search results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or a matched projected page
/// cannot be expanded into a deterministic navigation bundle.
pub fn repo_projected_page_navigation_search_from_config_with_registry(
    query: &RepoProjectedPageNavigationSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageNavigationSearchResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_page_navigation_search(query, &analysis)
}

/// Load configuration, analyze one repository, and return deterministic projected page-navigation
/// search results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or a matched projected page
/// cannot be expanded into a deterministic navigation bundle.
pub fn repo_projected_page_navigation_search_from_config(
    query: &RepoProjectedPageNavigationSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageNavigationSearchResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_navigation_search_from_config_with_registry(
        query,
        config_path,
        cwd,
        &registry,
    )
}

/// Build deterministic projected-page search results from normalized analysis records.
#[must_use]
pub fn build_repo_projected_page_search(
    query: &RepoProjectedPageSearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> RepoProjectedPageSearchResult {
    build_projected_page_search(query, analysis)
}

/// Load configuration, analyze one repository, and return deterministic projected-page search results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_projected_page_search_from_config_with_registry(
    query: &RepoProjectedPageSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageSearchResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_repo_projected_page_search(query, &analysis))
}

/// Load configuration, analyze one repository, and return deterministic projected-page search results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_projected_page_search_from_config(
    query: &RepoProjectedPageSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageSearchResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_search_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build one deterministic projected page-index node from normalized analysis records.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPageIndexNode`] when the requested projected
/// page-index node is not present in the analysis output.
pub fn build_repo_projected_page_index_node(
    query: &RepoProjectedPageIndexNodeQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageIndexNodeResult, RepoIntelligenceError> {
    build_projected_page_index_node(query, analysis)
}

/// Load configuration, analyze one repository, and return one deterministic projected page-index node.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page-index node identifier is not present for the repository.
pub fn repo_projected_page_index_node_from_config_with_registry(
    query: &RepoProjectedPageIndexNodeQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageIndexNodeResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_page_index_node(query, &analysis)
}

/// Load configuration, analyze one repository, and return one deterministic projected page-index node.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// page-index node identifier is not present for the repository.
pub fn repo_projected_page_index_node_from_config(
    query: &RepoProjectedPageIndexNodeQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageIndexNodeResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_index_node_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build deterministic mixed retrieval results from normalized analysis records.
#[must_use]
pub fn build_repo_projected_retrieval(
    query: &RepoProjectedRetrievalQuery,
    analysis: &RepositoryAnalysisOutput,
) -> RepoProjectedRetrievalResult {
    build_projected_retrieval(query, analysis)
}

/// Load configuration, analyze one repository, and return deterministic mixed retrieval results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_projected_retrieval_from_config_with_registry(
    query: &RepoProjectedRetrievalQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedRetrievalResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_repo_projected_retrieval(query, &analysis))
}

/// Load configuration, analyze one repository, and return deterministic mixed retrieval results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_projected_retrieval_from_config(
    query: &RepoProjectedRetrievalQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedRetrievalResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_retrieval_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build one deterministic mixed retrieval hit from normalized analysis records.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPage`] when the requested projected page is
/// not present in the analysis output, or [`RepoIntelligenceError::UnknownProjectedPageIndexNode`]
/// when the requested projected page-index node is not present for the projected page.
pub fn build_repo_projected_retrieval_hit(
    query: &RepoProjectedRetrievalHitQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedRetrievalHitResult, RepoIntelligenceError> {
    build_projected_retrieval_hit(query, analysis)
}

/// Load configuration, analyze one repository, and return one deterministic mixed retrieval hit.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// hit identifiers are not present for the repository.
pub fn repo_projected_retrieval_hit_from_config_with_registry(
    query: &RepoProjectedRetrievalHitQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedRetrievalHitResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_retrieval_hit(query, &analysis)
}

/// Load configuration, analyze one repository, and return one deterministic mixed retrieval hit.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// hit identifiers are not present for the repository.
pub fn repo_projected_retrieval_hit_from_config(
    query: &RepoProjectedRetrievalHitQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedRetrievalHitResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_retrieval_hit_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build deterministic local retrieval context around one stable Stage-2 hit.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPage`] when the requested projected page is
/// not present in the analysis output, or [`RepoIntelligenceError::UnknownProjectedPageIndexNode`]
/// when the requested projected page-index node is not present for the projected page.
pub fn build_repo_projected_retrieval_context(
    query: &RepoProjectedRetrievalContextQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedRetrievalContextResult, RepoIntelligenceError> {
    build_projected_retrieval_context(query, analysis)
}

/// Load configuration, analyze one repository, and return deterministic local retrieval context.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// hit identifiers are not present for the repository.
pub fn repo_projected_retrieval_context_from_config_with_registry(
    query: &RepoProjectedRetrievalContextQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedRetrievalContextResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_retrieval_context(query, &analysis)
}

/// Load configuration, analyze one repository, and return deterministic local retrieval context.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected
/// hit identifiers are not present for the repository.
pub fn repo_projected_retrieval_context_from_config(
    query: &RepoProjectedRetrievalContextQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedRetrievalContextResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_retrieval_context_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build one deterministic projected page-index tree from normalized analysis records.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedPage`] when the requested projected page is
/// not present in the analysis output, or another [`RepoIntelligenceError`] when projected page
/// markdown cannot be parsed into page-index trees.
pub fn build_repo_projected_page_index_tree(
    query: &RepoProjectedPageIndexTreeQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageIndexTreeResult, RepoIntelligenceError> {
    build_projected_page_index_tree(query, analysis)
}

/// Load configuration, analyze one repository, and return one deterministic projected page-index tree.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails, the requested projected page
/// identifier is not present for the repository, or projected page-index tree construction fails.
pub fn repo_projected_page_index_tree_from_config_with_registry(
    query: &RepoProjectedPageIndexTreeQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageIndexTreeResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_page_index_tree(query, &analysis)
}

/// Load configuration, analyze one repository, and return one deterministic projected page-index tree.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails, the requested projected page
/// identifier is not present for the repository, or projected page-index tree construction fails.
pub fn repo_projected_page_index_tree_from_config(
    query: &RepoProjectedPageIndexTreeQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageIndexTreeResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_index_tree_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build deterministic projected page-index tree search results from normalized analysis records.
#[must_use]
pub fn build_repo_projected_page_index_tree_search(
    query: &RepoProjectedPageIndexTreeSearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> RepoProjectedPageIndexTreeSearchResult {
    build_projected_page_index_tree_search(query, analysis)
}

/// Load configuration, analyze one repository, and return deterministic projected page-index tree search results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or projected page-index tree
/// construction fails.
pub fn repo_projected_page_index_tree_search_from_config_with_registry(
    query: &RepoProjectedPageIndexTreeSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageIndexTreeSearchResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_repo_projected_page_index_tree_search(
        query, &analysis,
    ))
}

/// Load configuration, analyze one repository, and return deterministic projected page-index tree search results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or projected page-index tree
/// construction fails.
pub fn repo_projected_page_index_tree_search_from_config(
    query: &RepoProjectedPageIndexTreeSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageIndexTreeSearchResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_index_tree_search_from_config_with_registry(
        query,
        config_path,
        cwd,
        &registry,
    )
}

/// Build deterministic page-family cluster search results from normalized analysis records.
#[must_use]
pub fn build_repo_projected_page_family_search(
    query: &RepoProjectedPageFamilySearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> RepoProjectedPageFamilySearchResult {
    build_projected_page_family_search(query, analysis)
}

/// Load configuration, analyze one repository, and return deterministic page-family cluster search results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_projected_page_family_search_from_config_with_registry(
    query: &RepoProjectedPageFamilySearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageFamilySearchResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    Ok(build_repo_projected_page_family_search(query, &analysis))
}

/// Load configuration, analyze one repository, and return deterministic page-family cluster search results.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn repo_projected_page_family_search_from_config(
    query: &RepoProjectedPageFamilySearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageFamilySearchResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_family_search_from_config_with_registry(query, config_path, cwd, &registry)
}

/// Build deterministic projected page-index trees from normalized analysis records.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when projected page markdown cannot be parsed into
/// page-index trees.
pub fn build_repo_projected_page_index_trees(
    query: &RepoProjectedPageIndexTreesQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<RepoProjectedPageIndexTreesResult, RepoIntelligenceError> {
    Ok(RepoProjectedPageIndexTreesResult {
        repo_id: query.repo_id.clone(),
        trees: build_projected_page_index_trees(analysis)?,
    })
}

/// Load configuration, analyze one repository, and return deterministic projected page-index trees.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis or projected page-index tree
/// construction fails.
pub fn repo_projected_page_index_trees_from_config_with_registry(
    query: &RepoProjectedPageIndexTreesQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<RepoProjectedPageIndexTreesResult, RepoIntelligenceError> {
    let analysis =
        analyze_repository_from_config_with_registry(&query.repo_id, config_path, cwd, registry)?;
    build_repo_projected_page_index_trees(query, &analysis)
}

/// Load configuration, analyze one repository, and return deterministic projected page-index trees.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis or projected page-index tree
/// construction fails.
pub fn repo_projected_page_index_trees_from_config(
    query: &RepoProjectedPageIndexTreesQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoProjectedPageIndexTreesResult, RepoIntelligenceError> {
    let registry = bootstrap_builtin_registry()?;
    repo_projected_page_index_trees_from_config_with_registry(query, config_path, cwd, &registry)
}

fn merge_repository_analysis(
    base: &mut RepositoryAnalysisOutput,
    mut overlay: RepositoryAnalysisOutput,
) {
    if base.repository.is_none() {
        base.repository = overlay.repository.take();
    }
    base.modules.append(&mut overlay.modules);
    base.symbols.append(&mut overlay.symbols);
    base.examples.append(&mut overlay.examples);
    base.docs.append(&mut overlay.docs);
    base.relations.append(&mut overlay.relations);
    base.diagnostics.append(&mut overlay.diagnostics);
}

fn dedupe_relations(relations: &mut Vec<RelationRecord>) {
    let mut seen = BTreeSet::new();
    relations.retain(|relation| {
        seen.insert((
            relation.source_id.clone(),
            relation.target_id.clone(),
            relation_kind_name(relation.kind),
        ))
    });
}

fn relation_kind_name(kind: RelationKind) -> &'static str {
    match kind {
        RelationKind::Contains => "contains",
        RelationKind::Declares => "declares",
        RelationKind::Uses => "uses",
        RelationKind::Implements => "implements",
        RelationKind::Documents => "documents",
        RelationKind::ExampleOf => "example_of",
    }
}

fn repo_hierarchical_uri(repo_id: &str) -> String {
    format!("repo://{repo_id}")
}

fn record_hierarchical_uri(repo_id: &str, kind: &str, record_id: &str) -> String {
    format!("repo://{repo_id}/{kind}/{record_id}")
}

fn hierarchy_segments_from_path(path: &str) -> Option<Vec<String>> {
    let segments = path
        .split('/')
        .map(str::trim)
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    (!segments.is_empty()).then_some(segments)
}

fn normalized_rank_score(raw_score: u8, worst_bucket: u8) -> f64 {
    let denominator = f64::from(worst_bucket.saturating_add(1));
    let numerator = f64::from(worst_bucket.saturating_add(1).saturating_sub(raw_score));
    (numerator / denominator).clamp(0.0, 1.0)
}

fn documents_backlink_lookup(
    relations: &[RelationRecord],
    docs: &[DocRecord],
) -> BTreeMap<String, Vec<RepoBacklinkItem>> {
    let doc_lookup = docs
        .iter()
        .map(|doc| (doc.doc_id.as_str(), doc))
        .collect::<BTreeMap<_, _>>();
    let mut lookup: BTreeMap<String, BTreeMap<String, RepoBacklinkItem>> = BTreeMap::new();
    for relation in relations
        .iter()
        .filter(|relation| relation.kind == RelationKind::Documents)
    {
        let source_id = relation.source_id.trim();
        let target_id = relation.target_id.trim();
        if source_id.is_empty() || target_id.is_empty() {
            continue;
        }
        let item = doc_lookup
            .get(source_id)
            .map(|doc| RepoBacklinkItem {
                id: doc.doc_id.clone(),
                title: Some(doc.title.clone()).filter(|title| !title.trim().is_empty()),
                path: Some(doc.path.clone()).filter(|path| !path.trim().is_empty()),
                kind: Some("documents".to_string()),
            })
            .unwrap_or_else(|| RepoBacklinkItem {
                id: source_id.to_string(),
                title: None,
                path: None,
                kind: Some("documents".to_string()),
            });
        lookup
            .entry(target_id.to_string())
            .or_default()
            .insert(item.id.clone(), item);
    }

    lookup
        .into_iter()
        .map(|(target_id, sources)| (target_id, sources.into_values().collect::<Vec<_>>()))
        .collect()
}

fn backlinks_for(
    target_id: &str,
    lookup: &BTreeMap<String, Vec<RepoBacklinkItem>>,
) -> (Option<Vec<String>>, Option<Vec<RepoBacklinkItem>>) {
    let Some(backlinks) = lookup.get(target_id) else {
        return (None, None);
    };
    let items = backlinks
        .iter()
        .filter_map(|backlink| {
            let id = backlink.id.trim();
            (!id.is_empty()).then(|| RepoBacklinkItem {
                id: id.to_string(),
                title: backlink
                    .title
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                path: backlink
                    .path
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
                kind: backlink
                    .kind
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string),
            })
        })
        .collect::<Vec<_>>();
    if items.is_empty() {
        return (None, None);
    }
    let ids = items
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    (Some(ids), Some(items))
}

fn projection_page_lookup(analysis: &RepositoryAnalysisOutput) -> BTreeMap<String, Vec<String>> {
    let mut lookup: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for page in build_projected_pages(analysis) {
        for anchor in page
            .module_ids
            .iter()
            .chain(page.symbol_ids.iter())
            .chain(page.example_ids.iter())
            .chain(page.doc_ids.iter())
        {
            lookup
                .entry(anchor.clone())
                .or_default()
                .insert(page.page_id.clone());
        }
    }

    lookup
        .into_iter()
        .map(|(anchor, page_ids)| (anchor, page_ids.into_iter().collect::<Vec<_>>()))
        .collect()
}

fn projection_pages_for(
    anchor_id: &str,
    lookup: &BTreeMap<String, Vec<String>>,
) -> Option<Vec<String>> {
    lookup.get(anchor_id).and_then(|page_ids| {
        let filtered = page_ids
            .iter()
            .map(String::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        (!filtered.is_empty()).then_some(filtered)
    })
}

fn module_match_score(query: &str, qualified_name: &str, path: &str) -> Option<u8> {
    if query.is_empty() {
        return Some(0);
    }
    if qualified_name == query {
        return Some(0);
    }
    if qualified_name.starts_with(query) {
        return Some(1);
    }
    if qualified_name.contains(query) {
        return Some(2);
    }
    if path.contains(query) {
        return Some(3);
    }
    None
}

fn symbol_match_score(
    query: &str,
    name: &str,
    qualified_name: &str,
    path: &str,
    signature: &str,
) -> Option<u8> {
    if query.is_empty() {
        return Some(0);
    }
    if name == query {
        return Some(0);
    }
    if qualified_name == query {
        return Some(1);
    }
    if name.starts_with(query) {
        return Some(2);
    }
    if qualified_name.starts_with(query) {
        return Some(3);
    }
    if name.contains(query) {
        return Some(4);
    }
    if qualified_name.contains(query) {
        return Some(5);
    }
    if signature.contains(query) {
        return Some(6);
    }
    if path.contains(query) {
        return Some(7);
    }
    None
}

fn example_match_score(
    query: &str,
    title: &str,
    path: &str,
    summary: &str,
    related_symbols: &[String],
    related_modules: &[String],
) -> Option<u8> {
    if query.is_empty() {
        return Some(0);
    }
    if title == query {
        return Some(0);
    }
    if title.starts_with(query) {
        return Some(1);
    }
    if title.contains(query) {
        return Some(2);
    }
    if related_symbols.iter().any(|candidate| candidate == query) {
        return Some(3);
    }
    if related_modules.iter().any(|candidate| candidate == query) {
        return Some(4);
    }
    if related_symbols
        .iter()
        .any(|candidate| candidate.starts_with(query))
    {
        return Some(5);
    }
    if related_modules
        .iter()
        .any(|candidate| candidate.starts_with(query))
    {
        return Some(6);
    }
    if path.contains(query) {
        return Some(7);
    }
    if summary.contains(query) {
        return Some(8);
    }
    if related_symbols
        .iter()
        .any(|candidate| candidate.contains(query))
    {
        return Some(9);
    }
    if related_modules
        .iter()
        .any(|candidate| candidate.contains(query))
    {
        return Some(10);
    }
    None
}

fn example_relation_lookup(relations: &[RelationRecord]) -> BTreeSet<(String, String)> {
    relations
        .iter()
        .filter(|relation| relation.kind == RelationKind::ExampleOf)
        .map(|relation| (relation.source_id.clone(), relation.target_id.clone()))
        .collect()
}

fn related_symbols_for_example(
    example_id: &str,
    relation_lookup: &BTreeSet<(String, String)>,
    symbols: &[SymbolRecord],
) -> Vec<String> {
    let symbol_ids = relation_lookup
        .iter()
        .filter(|(source_id, _)| source_id == example_id)
        .map(|(_, target_id)| target_id.as_str())
        .collect::<BTreeSet<_>>();

    symbols
        .iter()
        .filter(|symbol| symbol_ids.contains(symbol.symbol_id.as_str()))
        .flat_map(|symbol| {
            [
                symbol.name.to_ascii_lowercase(),
                symbol.qualified_name.to_ascii_lowercase(),
            ]
        })
        .collect()
}

fn related_modules_for_example(
    example_id: &str,
    relation_lookup: &BTreeSet<(String, String)>,
    modules: &[ModuleRecord],
) -> Vec<String> {
    let module_ids = relation_lookup
        .iter()
        .filter(|(source_id, _)| source_id == example_id)
        .map(|(_, target_id)| target_id.as_str())
        .collect::<BTreeSet<_>>();

    modules
        .iter()
        .filter(|module| module_ids.contains(module.module_id.as_str()))
        .flat_map(|module| {
            let short_name = module
                .qualified_name
                .rsplit('.')
                .next()
                .unwrap_or(module.qualified_name.as_str())
                .to_ascii_lowercase();
            [module.qualified_name.to_ascii_lowercase(), short_name]
        })
        .collect()
}

fn resolve_module_scope<'a>(
    module_selector: Option<&str>,
    modules: &'a [ModuleRecord],
) -> Option<&'a ModuleRecord> {
    let selector = module_selector?.trim();
    if selector.is_empty() {
        return None;
    }

    modules.iter().find(|module| {
        module.module_id == selector || module.qualified_name == selector || module.path == selector
    })
}

fn docs_in_scope(
    scoped_module: Option<&ModuleRecord>,
    analysis: &RepositoryAnalysisOutput,
) -> Vec<DocRecord> {
    match scoped_module {
        None => analysis.docs.clone(),
        Some(module) => {
            let mut target_ids = BTreeSet::from([module.module_id.clone()]);
            target_ids.extend(
                symbols_in_scope(Some(module), &analysis.symbols)
                    .into_iter()
                    .map(|symbol| symbol.symbol_id.clone()),
            );
            let doc_ids = analysis
                .relations
                .iter()
                .filter(|relation| {
                    relation.kind == RelationKind::Documents
                        && target_ids.contains(relation.target_id.as_str())
                })
                .map(|relation| relation.source_id.clone())
                .collect::<BTreeSet<_>>();
            analysis
                .docs
                .iter()
                .filter(|doc| doc_ids.contains(doc.doc_id.as_str()))
                .cloned()
                .collect()
        }
    }
}

fn documented_symbol_ids(
    scoped_module: Option<&ModuleRecord>,
    symbols: &[SymbolRecord],
    relations: &[RelationRecord],
) -> BTreeSet<String> {
    let scoped_symbol_ids = symbols_in_scope(scoped_module, symbols)
        .into_iter()
        .map(|symbol| symbol.symbol_id.clone())
        .collect::<BTreeSet<_>>();

    relations
        .iter()
        .filter(|relation| {
            relation.kind == RelationKind::Documents
                && scoped_symbol_ids.contains(&relation.target_id)
        })
        .map(|relation| relation.target_id.clone())
        .collect()
}

fn symbols_in_scope<'a>(
    scoped_module: Option<&ModuleRecord>,
    symbols: &'a [SymbolRecord],
) -> Vec<&'a SymbolRecord> {
    match scoped_module {
        None => symbols.iter().collect(),
        Some(module) => symbols
            .iter()
            .filter(|symbol| symbol.module_id.as_deref() == Some(module.module_id.as_str()))
            .collect(),
    }
}
