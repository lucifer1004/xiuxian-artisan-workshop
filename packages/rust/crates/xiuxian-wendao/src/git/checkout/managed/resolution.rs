use std::fs;

use chrono::Utc;
use git2::Repository;

use crate::analyzers::config::RegisteredRepository;
use crate::analyzers::config::{RepositoryRef, RepositoryRefreshPolicy};
use crate::analyzers::errors::RepoIntelligenceError;
use crate::git::checkout::managed::retry::{
    clone_bare_with_retry, ensure_remote_url, fetch_origin_with_retry,
    is_retryable_remote_error_message, open_bare_with_retry, open_checkout_with_retry,
    probe_remote_target_revision_with_retry, should_fetch,
};
use crate::git::checkout::{
    RepositoryLifecycleState, RepositorySyncMode, ResolvedRepositorySource,
    ResolvedRepositorySourceKind, lock, metadata, namespace, refs,
};

#[allow(clippy::too_many_lines)]
pub(crate) fn resolve_managed_checkout(
    repository: &RegisteredRepository,
    mode: RepositorySyncMode,
) -> Result<ResolvedRepositorySource, RepoIntelligenceError> {
    let upstream_url = repository
        .url
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| RepoIntelligenceError::MissingRepositorySource {
            repo_id: repository.id.clone(),
        })?;
    let mirror_root = namespace::managed_mirror_root_for(repository);
    let checkout_root = namespace::managed_checkout_root_for(repository);

    if (!mirror_root.exists() || !checkout_root.exists())
        && matches!(mode, RepositorySyncMode::Status)
    {
        return Ok(ResolvedRepositorySource {
            checkout_root,
            mirror_root: Some(mirror_root),
            mirror_revision: None,
            tracking_revision: None,
            last_fetched_at: None,
            drift_state: crate::analyzers::query::RepoSyncDriftState::Unknown,
            mirror_state: RepositoryLifecycleState::Missing,
            checkout_state: RepositoryLifecycleState::Missing,
            source_kind: ResolvedRepositorySourceKind::ManagedRemote,
        });
    }

    let _checkout_lock = (!matches!(mode, RepositorySyncMode::Status))
        .then(|| lock::acquire_managed_checkout_lock(repository))
        .transpose()?;

    if let Some(parent) = mirror_root.parent() {
        fs::create_dir_all::<&std::path::Path>(parent).map_err(|error| {
            RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to create managed mirror dir `{}`: {error}",
                    parent.display()
                ),
            }
        })?;
    }
    if let Some(parent) = checkout_root.parent() {
        fs::create_dir_all::<&std::path::Path>(parent).map_err(|error| {
            RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to create managed checkout dir `{}`: {error}",
                    parent.display()
                ),
            }
        })?;
    }

    let mirror_existed = mirror_root.exists();
    let (mirror_repository, mirror_mutated) = if mirror_existed {
        let repo = open_bare_with_retry(&mirror_root).map_err(|error| {
            RepoIntelligenceError::InvalidRepositoryPath {
                repo_id: repository.id.clone(),
                path: mirror_root.display().to_string(),
                reason: format!("failed to open managed mirror as bare git repository: {error}"),
            }
        })?;
        let remote_updated = if matches!(mode, RepositorySyncMode::Status) {
            false
        } else {
            ensure_remote_url(&repo, "origin", upstream_url).map_err(|error| {
                RepoIntelligenceError::AnalysisFailed {
                    message: format!(
                        "failed to align managed mirror `{}` remote with `{upstream_url}`: {error}",
                        repository.id
                    ),
                }
            })?
        };
        let current_mirror_revision =
            desired_managed_checkout_revision(&repo, repository.git_ref.as_ref());
        let probed_target_revision = if matches!(mode, RepositorySyncMode::Ensure)
            && matches!(repository.refresh, RepositoryRefreshPolicy::Fetch)
            && !remote_updated
            && !matches!(repository.git_ref, Some(RepositoryRef::Commit(_)))
        {
            Some(probe_remote_target_revision_with_retry(
                &repo,
                repository.git_ref.as_ref(),
            ))
        } else {
            None
        };
        let mirror_requires_fetch = mirror_requires_fetch(
            mode,
            repository.refresh,
            repository.git_ref.as_ref(),
            remote_updated,
            current_mirror_revision.as_deref(),
            probed_target_revision
                .as_ref()
                .and_then(|result| result.as_ref().ok())
                .and_then(|revision| revision.as_deref()),
        );
        if let Some(Err(error)) = probed_target_revision.as_ref() {
            metadata::record_managed_remote_probe_failure(
                &mirror_root,
                error.message(),
                is_retryable_remote_error_message(error.message()),
            )
            .map_err(|record_error| RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to record managed mirror probe failure for `{}`: {record_error}",
                    repository.id
                ),
            })?;
        }
        if mirror_requires_fetch {
            fetch_origin_with_retry(&repo).map_err(|error| {
                RepoIntelligenceError::AnalysisFailed {
                    message: format!(
                        "failed to refresh managed mirror `{}` from `{upstream_url}`: {error}",
                        repository.id
                    ),
                }
            })?;
            metadata::clear_managed_remote_probe_state(&mirror_root).map_err(|error| {
                RepoIntelligenceError::AnalysisFailed {
                    message: format!(
                        "failed to clear managed mirror probe state for `{}` after fetch: {error}",
                        repository.id
                    ),
                }
            })?;
        } else if let Some(Ok(target_revision)) = probed_target_revision.as_ref() {
            metadata::record_managed_remote_probe_state(&mirror_root, target_revision.as_deref())
                .map_err(|error| RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to record managed mirror probe state for `{}`: {error}",
                    repository.id
                ),
            })?;
        }
        (repo, remote_updated || mirror_requires_fetch)
    } else {
        (
            clone_bare_with_retry(upstream_url, &mirror_root).map_err(|error| {
                RepoIntelligenceError::AnalysisFailed {
                    message: format!(
                        "failed to clone mirror for repository `{}` from `{upstream_url}`: {error}",
                        repository.id
                    ),
                }
            })?,
            true,
        )
    };
    let mirror_revision = metadata::resolve_head_revision(&mirror_repository);
    let mirror_state = mirror_lifecycle_state_for(mode, mirror_existed, mirror_mutated);
    let desired_checkout_revision =
        desired_managed_checkout_revision(&mirror_repository, repository.git_ref.as_ref());
    let checkout_existed = checkout_root.exists();
    let mirror_origin = std::fs::canonicalize(&mirror_root)
        .unwrap_or_else(|_| mirror_root.clone())
        .display()
        .to_string();
    let (repository_handle, checkout_mutated) = if checkout_existed {
        let repo = open_checkout_with_retry(&checkout_root).map_err(|error| {
            RepoIntelligenceError::InvalidRepositoryPath {
                repo_id: repository.id.clone(),
                path: checkout_root.display().to_string(),
                reason: format!("failed to open managed checkout as git repository: {error}"),
            }
        })?;
        let current_checkout_revision = metadata::resolve_head_revision(&repo);
        let remote_updated = if matches!(mode, RepositorySyncMode::Status) {
            false
        } else {
            ensure_remote_url(&repo, "origin", mirror_origin.as_str()).map_err(|error| {
                RepoIntelligenceError::AnalysisFailed {
                    message: format!(
                        "failed to align managed checkout `{}` remote with mirror `{mirror_origin}`: {error}",
                        repository.id
                    ),
                }
            })?
        };
        let checkout_requires_fetch = mirror_mutated
            || checkout_requires_fetch(
                mode,
                repository.refresh,
                remote_updated,
                current_checkout_revision.as_deref(),
                desired_checkout_revision.as_deref(),
            );
        if checkout_requires_fetch {
            fetch_origin_with_retry(&repo).map_err(|error| RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to refresh managed checkout `{}` from mirror `{mirror_origin}`: {error}",
                    repository.id
                ),
            })?;
        }
        let checkout_requires_head_sync = !matches!(mode, RepositorySyncMode::Status)
            && current_checkout_revision.as_deref() != desired_checkout_revision.as_deref();
        if checkout_requires_head_sync {
            refs::sync_checkout_head(&repo, repository.git_ref.as_ref()).map_err(|error| {
                RepoIntelligenceError::AnalysisFailed {
                    message: format!(
                        "failed to materialize requested git ref for `{}`: {error}",
                        repository.id
                    ),
                }
            })?;
        }
        (
            repo,
            remote_updated || checkout_requires_fetch || checkout_requires_head_sync,
        )
    } else {
        (
            Repository::clone(&mirror_origin, &checkout_root).map_err(|error| {
                RepoIntelligenceError::AnalysisFailed {
                    message: format!(
                        "failed to materialize managed checkout `{}` from mirror `{mirror_origin}`: {error}",
                        repository.id
                    ),
                }
            })?,
            true,
        )
    };

    if !matches!(mode, RepositorySyncMode::Status)
        && !checkout_existed
        && desired_checkout_revision.is_some()
    {
        refs::sync_checkout_head(&repository_handle, repository.git_ref.as_ref()).map_err(
            |error| RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to materialize requested git ref for `{}`: {error}",
                    repository.id
                ),
            },
        )?;
    }

    let revision = metadata::resolve_head_revision(&repository_handle);
    let tracking_revision =
        metadata::resolve_tracking_revision(&repository_handle, repository.git_ref.as_ref());
    let checkout_state = checkout_lifecycle_state_for(mode, checkout_existed, checkout_mutated);
    let fetched_at = metadata::discover_last_fetched_at(&mirror_root)
        .or_else(|| (!matches!(mode, RepositorySyncMode::Status)).then(|| Utc::now().to_rfc3339()));

    Ok(ResolvedRepositorySource {
        checkout_root: checkout_root.clone(),
        mirror_root: Some(mirror_root),
        mirror_revision: mirror_revision.clone(),
        tracking_revision: tracking_revision.clone(),
        last_fetched_at: fetched_at,
        drift_state: metadata::compute_managed_drift_state(
            &repository_handle,
            revision.as_deref(),
            tracking_revision.as_deref(),
            mirror_revision.as_deref(),
        ),
        mirror_state,
        checkout_state,
        source_kind: ResolvedRepositorySourceKind::ManagedRemote,
    })
}

fn mirror_lifecycle_state_for(
    mode: RepositorySyncMode,
    existed: bool,
    mutated: bool,
) -> RepositoryLifecycleState {
    if !existed {
        return RepositoryLifecycleState::Created;
    }
    if matches!(mode, RepositorySyncMode::Status) {
        return RepositoryLifecycleState::Observed;
    }
    if mutated {
        RepositoryLifecycleState::Refreshed
    } else {
        RepositoryLifecycleState::Reused
    }
}

fn checkout_lifecycle_state_for(
    mode: RepositorySyncMode,
    existed: bool,
    mutated: bool,
) -> RepositoryLifecycleState {
    if !existed {
        return RepositoryLifecycleState::Created;
    }
    if matches!(mode, RepositorySyncMode::Status) {
        return RepositoryLifecycleState::Observed;
    }
    if mutated {
        RepositoryLifecycleState::Refreshed
    } else {
        RepositoryLifecycleState::Reused
    }
}

fn desired_managed_checkout_revision(
    mirror_repository: &Repository,
    git_ref: Option<&RepositoryRef>,
) -> Option<String> {
    match git_ref {
        Some(RepositoryRef::Branch(branch)) => mirror_repository
            .find_reference(format!("refs/heads/{branch}").as_str())
            .ok()
            .and_then(|reference| reference.target().map(|oid| oid.to_string())),
        Some(RepositoryRef::Tag(tag)) => mirror_repository
            .find_reference(format!("refs/tags/{tag}").as_str())
            .ok()
            .and_then(|reference| reference.target().map(|oid| oid.to_string())),
        Some(RepositoryRef::Commit(sha)) => Some(sha.clone()),
        None => metadata::resolve_head_revision(mirror_repository),
    }
}

fn checkout_requires_fetch(
    mode: RepositorySyncMode,
    refresh: RepositoryRefreshPolicy,
    remote_updated: bool,
    current_checkout_revision: Option<&str>,
    desired_checkout_revision: Option<&str>,
) -> bool {
    if matches!(
        mode,
        RepositorySyncMode::Status | RepositorySyncMode::Refresh
    ) {
        return matches!(mode, RepositorySyncMode::Refresh);
    }
    if remote_updated {
        return true;
    }
    if !should_fetch(refresh, mode) {
        return false;
    }
    current_checkout_revision != desired_checkout_revision
}

fn mirror_requires_fetch(
    mode: RepositorySyncMode,
    refresh: RepositoryRefreshPolicy,
    git_ref: Option<&RepositoryRef>,
    remote_updated: bool,
    current_mirror_revision: Option<&str>,
    probed_target_revision: Option<&str>,
) -> bool {
    if matches!(mode, RepositorySyncMode::Refresh) {
        return true;
    }
    if matches!(mode, RepositorySyncMode::Status) {
        return false;
    }
    if remote_updated {
        return true;
    }
    if !should_fetch(refresh, mode) {
        return false;
    }

    match git_ref {
        Some(RepositoryRef::Commit(commit)) => current_mirror_revision != Some(commit.as_str()),
        _ => probed_target_revision != current_mirror_revision,
    }
}
