use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use xiuxian_git_repo::{
    MaterializedRepo, RepoDriftState, RepoLifecycleState, RepoSourceKind as GitRepoSourceKind,
    RevisionChangeKind, RevisionPathChange, diff_checkout_revisions, discover_checkout_metadata,
};
use xiuxian_wendao_julia::julia_parser_summary_allows_safe_incremental_file_for_repository;

use crate::analyzers::cache::{
    FingerprintMode, ValkeyAnalysisCache, analysis_fingerprint_mode,
    build_repository_analysis_cache_key, change_affects_analysis_identity,
    load_cached_repository_analysis_for_revision, store_cached_repository_analysis,
};
use crate::analyzers::errors::RepoIntelligenceError;
use crate::analyzers::plugin::{AnalysisContext, RepoSourceFile, RepositoryAnalysisOutput};
use crate::analyzers::service::{
    IncrementalApplyContext, analyze_changed_files, apply_incremental_plugin_outputs,
};
use crate::analyzers::{RegisteredRepository, RepoSourceKind, RepoSyncResult};
use crate::repo_index::state::coordinator::RepoIndexCoordinator;
use crate::repo_index::state::language::is_supported_code_path;

pub(crate) enum PreparedIncrementalAnalysis {
    RefreshOnly,
    Analysis(Box<RepositoryAnalysisOutput>),
}

impl RepoIndexCoordinator {
    pub(crate) fn prepare_incremental_analysis(
        &self,
        repository: &RegisteredRepository,
        sync_result: &RepoSyncResult,
        previous_revision: Option<&str>,
    ) -> Result<Option<PreparedIncrementalAnalysis>, RepoIntelligenceError> {
        let Some(current_revision) = sync_result.revision.as_deref() else {
            return Ok(None);
        };
        let Some(previous_revision) =
            previous_revision.filter(|revision| *revision != current_revision)
        else {
            return Ok(None);
        };

        let diff = diff_checkout_revisions(
            Path::new(sync_result.checkout_path.as_str()),
            previous_revision,
            current_revision,
        )
        .map_err(|error| RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "repo `{}` failed to diff `{previous_revision}` -> `{current_revision}`: {error}",
                repository.id
            ),
        })?;

        if diff.is_empty() {
            return Ok(Some(PreparedIncrementalAnalysis::RefreshOnly));
        }

        let plugin_ids = sorted_plugin_ids(repository);
        let analysis_changes = diff
            .changes
            .iter()
            .filter(|change| change_affects_analysis(change, plugin_ids.as_slice()))
            .cloned()
            .collect::<Vec<_>>();
        if analysis_changes.is_empty() {
            return Self::prepare_non_analysis_incremental(
                repository,
                sync_result,
                previous_revision,
                plugin_ids.as_slice(),
                &diff.changes,
            );
        }

        self.prepare_safe_julia_incremental(
            repository,
            sync_result,
            previous_revision,
            plugin_ids.as_slice(),
            analysis_changes.as_slice(),
        )
    }

    fn prepare_non_analysis_incremental(
        repository: &RegisteredRepository,
        sync_result: &RepoSyncResult,
        previous_revision: &str,
        plugin_ids: &[String],
        diff_changes: &[RevisionPathChange],
    ) -> Result<Option<PreparedIncrementalAnalysis>, RepoIntelligenceError> {
        if !touches_supported_code_paths(diff_changes) {
            return Ok(Some(PreparedIncrementalAnalysis::RefreshOnly));
        }

        let analysis = Self::load_previous_analysis_for_revision(
            repository,
            sync_result,
            plugin_ids,
            previous_revision,
        )?
        .ok_or_else(|| RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "repo `{}` could not recover cached analysis for revision `{previous_revision}`",
                repository.id
            ),
        })?;
        Self::store_current_analysis(repository, sync_result, &analysis)?;

        Ok(Some(PreparedIncrementalAnalysis::Analysis(Box::new(
            analysis,
        ))))
    }

    fn prepare_safe_julia_incremental(
        &self,
        repository: &RegisteredRepository,
        sync_result: &RepoSyncResult,
        previous_revision: &str,
        plugin_ids: &[String],
        analysis_changes: &[RevisionPathChange],
    ) -> Result<Option<PreparedIncrementalAnalysis>, RepoIntelligenceError> {
        let plugins = self.plugin_registry.resolve_for_repository(repository)?;
        if plugins.len() != 1 || plugins[0].id() != "julia" {
            return Ok(None);
        }

        let deleted_paths = analysis_changes
            .iter()
            .filter(|change| matches!(change.kind, RevisionChangeKind::Deleted))
            .map(|change| change.path.clone())
            .collect::<BTreeSet<_>>();
        if !deleted_paths.is_empty() {
            return Ok(None);
        }

        let Some(changed_files) = collect_safe_incremental_julia_files(
            repository,
            Path::new(sync_result.checkout_path.as_str()),
            analysis_changes,
        )?
        else {
            return Ok(None);
        };

        let Some(mut analysis) = Self::load_previous_analysis_for_revision(
            repository,
            sync_result,
            plugin_ids,
            previous_revision,
        )?
        else {
            return Ok(None);
        };

        let analysis_context = AnalysisContext {
            repository: repository.clone(),
            repository_root: PathBuf::from(sync_result.checkout_path.as_str()),
        };
        let overlays =
            analyze_changed_files(&analysis_context, &plugins[0], changed_files.as_slice())?;
        let checkout_metadata =
            discover_checkout_metadata(Path::new(sync_result.checkout_path.as_str()));
        let changed_paths = analysis_changes
            .iter()
            .map(|change| change.path.clone())
            .collect::<BTreeSet<_>>();
        apply_incremental_plugin_outputs(
            &IncrementalApplyContext {
                repository,
                repository_root: Path::new(sync_result.checkout_path.as_str()),
                checkout_metadata: checkout_metadata.as_ref(),
                plugins: plugins.as_slice(),
            },
            &mut analysis,
            overlays,
            &changed_paths,
            &deleted_paths,
        )?;
        Self::store_current_analysis(repository, sync_result, &analysis)?;

        Ok(Some(PreparedIncrementalAnalysis::Analysis(Box::new(
            analysis,
        ))))
    }

    fn load_previous_analysis_for_revision(
        repository: &RegisteredRepository,
        sync_result: &RepoSyncResult,
        plugin_ids: &[String],
        previous_revision: &str,
    ) -> Result<Option<RepositoryAnalysisOutput>, RepoIntelligenceError> {
        let checkout_root = sync_result.checkout_path.as_str();
        if let Some(cached) = load_cached_repository_analysis_for_revision(
            repository.id.as_str(),
            checkout_root,
            plugin_ids,
            previous_revision,
        )? {
            return Ok(Some(cached));
        }

        let Some(cache) = ValkeyAnalysisCache::new()? else {
            return Ok(None);
        };
        let Some(cached) = cache.get_for_revision(
            repository.id.as_str(),
            checkout_root,
            plugin_ids,
            previous_revision,
        ) else {
            return Ok(None);
        };

        Ok(Some(cached))
    }

    fn store_current_analysis(
        repository: &RegisteredRepository,
        sync_result: &RepoSyncResult,
        analysis: &RepositoryAnalysisOutput,
    ) -> Result<(), RepoIntelligenceError> {
        let checkout_root = Path::new(sync_result.checkout_path.as_str());
        let checkout_metadata = discover_checkout_metadata(checkout_root);
        let cache_key = build_repository_analysis_cache_key(
            repository,
            &materialized_repo_from_sync_result(sync_result),
            checkout_metadata.as_ref(),
        );
        store_cached_repository_analysis(cache_key.clone(), analysis)?;
        if let Some(cache) = ValkeyAnalysisCache::new()? {
            cache.set(&cache_key, analysis);
        }
        Ok(())
    }
}

fn materialized_repo_from_sync_result(sync_result: &RepoSyncResult) -> MaterializedRepo {
    MaterializedRepo {
        checkout_root: PathBuf::from(sync_result.checkout_path.as_str()),
        mirror_root: sync_result.mirror_path.as_ref().map(PathBuf::from),
        mirror_revision: sync_result.mirror_revision.clone(),
        tracking_revision: sync_result.tracking_revision.clone(),
        last_fetched_at: sync_result.last_fetched_at.clone(),
        drift_state: RepoDriftState::NotApplicable,
        mirror_state: RepoLifecycleState::Observed,
        checkout_state: RepoLifecycleState::Observed,
        source_kind: match sync_result.source_kind {
            RepoSourceKind::LocalCheckout => GitRepoSourceKind::LocalCheckout,
            RepoSourceKind::ManagedRemote => GitRepoSourceKind::ManagedRemote,
        },
    }
}

fn sorted_plugin_ids(repository: &RegisteredRepository) -> Vec<String> {
    repository.repo_intelligence_plugin_ids()
}

fn change_affects_analysis(change: &RevisionPathChange, plugin_ids: &[String]) -> bool {
    match change.kind {
        RevisionChangeKind::Added | RevisionChangeKind::Deleted => {
            change_affects_analysis_identity(change.path.as_str(), plugin_ids, false)
                || change
                    .previous_path
                    .as_deref()
                    .is_some_and(|path| change_affects_analysis_identity(path, plugin_ids, false))
        }
        RevisionChangeKind::Modified | RevisionChangeKind::TypeChanged => {
            change_affects_analysis_identity(change.path.as_str(), plugin_ids, true)
        }
        RevisionChangeKind::Renamed | RevisionChangeKind::Copied => {
            change_affects_analysis_identity(change.path.as_str(), plugin_ids, false)
                || change
                    .previous_path
                    .as_deref()
                    .is_some_and(|path| change_affects_analysis_identity(path, plugin_ids, false))
        }
    }
}

fn touches_supported_code_paths(changes: &[RevisionPathChange]) -> bool {
    changes.iter().any(|change| {
        is_supported_code_path(change.path.as_str())
            || change
                .previous_path
                .as_deref()
                .is_some_and(is_supported_code_path)
    })
}

fn collect_safe_incremental_julia_files(
    repository: &RegisteredRepository,
    checkout_root: &Path,
    changes: &[RevisionPathChange],
) -> Result<Option<Vec<RepoSourceFile>>, RepoIntelligenceError> {
    let mut files = Vec::new();

    for change in changes {
        if !matches!(
            change.kind,
            RevisionChangeKind::Added | RevisionChangeKind::Modified
        ) {
            return Ok(None);
        }
        if !change.path.starts_with("src/") || !is_supported_code_path(change.path.as_str()) {
            return Ok(None);
        }
        if !matches!(
            analysis_fingerprint_mode(change.path.as_str(), &sorted_plugin_ids(repository)),
            Some(FingerprintMode::Contents)
        ) {
            return Ok(None);
        }

        let file_path = checkout_root.join(change.path.as_str());
        let contents = std::fs::read_to_string(&file_path).map_err(|error| {
            RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "repo `{}` failed to read changed source `{}`: {error}",
                    repository.id,
                    file_path.display()
                ),
            }
        })?;
        if !julia_parser_summary_allows_safe_incremental_file_for_repository(
            repository,
            change.path.as_str(),
            &contents,
        )? {
            return Ok(None);
        }
        files.push(RepoSourceFile {
            path: change.path.clone(),
            contents,
        });
    }

    Ok(Some(files))
}
