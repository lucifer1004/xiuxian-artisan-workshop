use std::fs;

use chrono::Utc;
use git2::{AutotagOption, FetchOptions, Repository, build::RepoBuilder};

use super::{
    RepositoryLifecycleState, RepositorySyncMode, ResolvedRepositorySource,
    ResolvedRepositorySourceKind,
};
use crate::analyzers::config::{RegisteredRepository, RepositoryRefreshPolicy};
use crate::analyzers::errors::RepoIntelligenceError;

#[allow(clippy::too_many_lines)]
pub(super) fn resolve_managed_checkout(
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
    let mirror_root = super::namespace::managed_mirror_root_for(repository);
    let checkout_root = super::namespace::managed_checkout_root_for(repository);

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
        .then(|| super::lock::acquire_managed_checkout_lock(repository))
        .transpose()?;

    if let Some(parent) = mirror_root.parent() {
        fs::create_dir_all(parent).map_err(|error| RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "failed to create managed mirror dir `{}`: {error}",
                parent.display()
            ),
        })?;
    }
    if let Some(parent) = checkout_root.parent() {
        fs::create_dir_all(parent).map_err(|error| RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "failed to create managed checkout dir `{}`: {error}",
                parent.display()
            ),
        })?;
    }

    let mirror_existed = mirror_root.exists();
    let (mirror_repository, mirror_remote_updated) = if mirror_existed {
        let repo = Repository::open_bare(&mirror_root).map_err(|error| {
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
        if remote_updated || should_fetch(repository.refresh, mode) {
            fetch_origin(&repo).map_err(|error| RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to refresh managed mirror `{}` from `{upstream_url}`: {error}",
                    repository.id
                ),
            })?;
        }
        (repo, remote_updated)
    } else {
        let mut builder = RepoBuilder::new();
        builder.bare(true);
        (
            builder.clone(upstream_url, &mirror_root).map_err(|error| {
                RepoIntelligenceError::AnalysisFailed {
                    message: format!(
                        "failed to clone mirror for repository `{}` from `{upstream_url}`: {error}",
                        repository.id
                    ),
                }
            })?,
            false,
        )
    };
    let mirror_revision = super::metadata::resolve_head_revision(&mirror_repository);
    let mirror_state = lifecycle_state_for(mode, mirror_existed, repository.refresh);
    let mirror_synchronized =
        !mirror_existed || mirror_remote_updated || should_fetch(repository.refresh, mode);

    let checkout_existed = checkout_root.exists();
    let mirror_origin = std::fs::canonicalize(&mirror_root)
        .unwrap_or_else(|_| mirror_root.clone())
        .display()
        .to_string();
    let repository_handle = if checkout_existed {
        let repo = Repository::open(&checkout_root).map_err(|error| {
            RepoIntelligenceError::InvalidRepositoryPath {
                repo_id: repository.id.clone(),
                path: checkout_root.display().to_string(),
                reason: format!("failed to open managed checkout as git repository: {error}"),
            }
        })?;
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
        if remote_updated || mirror_synchronized || should_fetch(repository.refresh, mode) {
            fetch_origin(&repo).map_err(|error| RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to refresh managed checkout `{}` from mirror `{mirror_origin}`: {error}",
                    repository.id
                ),
            })?;
        }
        repo
    } else {
        Repository::clone(&mirror_origin, &checkout_root).map_err(|error| {
            RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to materialize managed checkout `{}` from mirror `{mirror_origin}`: {error}",
                    repository.id
                ),
            }
        })?
    };

    if !matches!(mode, RepositorySyncMode::Status) {
        super::refs::sync_checkout_head(&repository_handle, repository.git_ref.as_ref()).map_err(
            |error| RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to materialize requested git ref for `{}`: {error}",
                    repository.id
                ),
            },
        )?;
    }

    let revision = super::metadata::resolve_head_revision(&repository_handle);
    let tracking_revision =
        super::metadata::resolve_tracking_revision(&repository_handle, repository.git_ref.as_ref());
    let checkout_state = lifecycle_state_for(mode, checkout_existed, repository.refresh);
    let fetched_at = super::metadata::discover_last_fetched_at(&mirror_root)
        .or_else(|| (!matches!(mode, RepositorySyncMode::Status)).then(|| Utc::now().to_rfc3339()));

    Ok(ResolvedRepositorySource {
        checkout_root: checkout_root.clone(),
        mirror_root: Some(mirror_root),
        mirror_revision: mirror_revision.clone(),
        tracking_revision: tracking_revision.clone(),
        last_fetched_at: fetched_at,
        drift_state: super::metadata::compute_managed_drift_state(
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

fn should_fetch(refresh: RepositoryRefreshPolicy, mode: RepositorySyncMode) -> bool {
    matches!(mode, RepositorySyncMode::Refresh)
        || (matches!(mode, RepositorySyncMode::Ensure)
            && matches!(refresh, RepositoryRefreshPolicy::Fetch))
}

fn fetch_origin(repository: &Repository) -> Result<(), git2::Error> {
    let mut remote = repository.find_remote("origin")?;
    let mut options = FetchOptions::new();
    options.download_tags(AutotagOption::All);
    let refspecs: &[&str] = if repository.is_bare() {
        &["+refs/heads/*:refs/heads/*", "+refs/tags/*:refs/tags/*"]
    } else {
        &[
            "+refs/heads/*:refs/remotes/origin/*",
            "+HEAD:refs/remotes/origin/HEAD",
            "+refs/tags/*:refs/tags/*",
        ]
    };
    remote.fetch(refspecs, Some(&mut options), None)?;
    Ok(())
}

fn lifecycle_state_for(
    mode: RepositorySyncMode,
    existed: bool,
    refresh: RepositoryRefreshPolicy,
) -> RepositoryLifecycleState {
    if !existed {
        return RepositoryLifecycleState::Created;
    }
    if should_fetch(refresh, mode) {
        return RepositoryLifecycleState::Refreshed;
    }
    if matches!(mode, RepositorySyncMode::Status) {
        RepositoryLifecycleState::Observed
    } else {
        RepositoryLifecycleState::Reused
    }
}

fn ensure_remote_url(
    repository: &Repository,
    remote_name: &str,
    expected_url: &str,
) -> Result<bool, git2::Error> {
    match current_remote_url(repository, remote_name) {
        Some(current) if current == expected_url => Ok(false),
        Some(_) => {
            repository.remote_set_url(remote_name, expected_url)?;
            Ok(true)
        }
        None => {
            repository.remote(remote_name, expected_url)?;
            Ok(true)
        }
    }
}

pub(super) fn current_remote_url(repository: &Repository, remote_name: &str) -> Option<String> {
    repository
        .find_remote(remote_name)
        .ok()
        .and_then(|remote| remote.url().map(str::to_string))
}
