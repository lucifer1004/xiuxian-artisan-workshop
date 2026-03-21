use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Utc};
use git2::build::{CheckoutBuilder, RepoBuilder};
use git2::{AutotagOption, BranchType, FetchOptions, Oid, Repository};
use xiuxian_config_core::resolve_cache_home;

use super::config::{RegisteredRepository, RepositoryRefreshPolicy};
use super::errors::RepoIntelligenceError;
use super::query::RepoSyncDriftState;

static MANAGED_REPO_SYNC_LOCKS: OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> =
    OnceLock::new();
const STALE_CHECKOUT_INDEX_LOCK_AGE: Duration = Duration::from_secs(120);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LocalCheckoutMetadata {
    pub(crate) revision: Option<String>,
    pub(crate) remote_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedRepositorySourceKind {
    LocalCheckout,
    ManagedRemote,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepositorySyncMode {
    Ensure,
    Refresh,
    Status,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepositoryLifecycleState {
    NotApplicable,
    Missing,
    Validated,
    Observed,
    Created,
    Reused,
    Refreshed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResolvedRepositorySource {
    pub(crate) checkout_root: PathBuf,
    pub(crate) mirror_root: Option<PathBuf>,
    pub(crate) source_kind: ResolvedRepositorySourceKind,
    pub(crate) mirror_state: RepositoryLifecycleState,
    pub(crate) checkout_state: RepositoryLifecycleState,
    pub(crate) last_fetched_at: Option<String>,
    pub(crate) mirror_revision: Option<String>,
    pub(crate) tracking_revision: Option<String>,
    pub(crate) drift_state: RepoSyncDriftState,
}

pub(crate) fn resolve_local_checkout_root(
    repo_id: &str,
    configured_path: &Path,
) -> Result<PathBuf, RepoIntelligenceError> {
    if !configured_path.is_dir() {
        return Err(RepoIntelligenceError::InvalidRepositoryPath {
            repo_id: repo_id.to_string(),
            path: configured_path.display().to_string(),
            reason: "path does not exist or is not a directory".to_string(),
        });
    }

    let canonical_path = fs::canonicalize(configured_path).map_err(|error| {
        RepoIntelligenceError::InvalidRepositoryPath {
            repo_id: repo_id.to_string(),
            path: configured_path.display().to_string(),
            reason: format!("failed to canonicalize repository path: {error}"),
        }
    })?;
    let repository = Repository::discover(&canonical_path).map_err(|error| {
        RepoIntelligenceError::InvalidRepositoryPath {
            repo_id: repo_id.to_string(),
            path: canonical_path.display().to_string(),
            reason: format!("path is not inside a git checkout: {error}"),
        }
    })?;
    let workdir =
        repository
            .workdir()
            .ok_or_else(|| RepoIntelligenceError::InvalidRepositoryPath {
                repo_id: repo_id.to_string(),
                path: canonical_path.display().to_string(),
                reason: "git repository has no working directory".to_string(),
            })?;
    let canonical_workdir = fs::canonicalize(workdir).map_err(|error| {
        RepoIntelligenceError::InvalidRepositoryPath {
            repo_id: repo_id.to_string(),
            path: workdir.display().to_string(),
            reason: format!("failed to canonicalize git workdir: {error}"),
        }
    })?;

    if canonical_path != canonical_workdir {
        return Err(RepoIntelligenceError::InvalidRepositoryPath {
            repo_id: repo_id.to_string(),
            path: canonical_path.display().to_string(),
            reason: format!(
                "path must point at the git checkout root `{}`",
                canonical_workdir.display()
            ),
        });
    }

    Ok(canonical_workdir)
}

pub(crate) fn resolve_repository_source(
    repository: &RegisteredRepository,
    cwd: &Path,
    sync_mode: RepositorySyncMode,
) -> Result<ResolvedRepositorySource, RepoIntelligenceError> {
    if let Some(path) = repository.path.as_ref() {
        return Ok(ResolvedRepositorySource {
            checkout_root: resolve_local_checkout_root(&repository.id, path)?,
            mirror_root: None,
            source_kind: ResolvedRepositorySourceKind::LocalCheckout,
            mirror_state: RepositoryLifecycleState::NotApplicable,
            checkout_state: RepositoryLifecycleState::Validated,
            last_fetched_at: None,
            mirror_revision: None,
            tracking_revision: None,
            drift_state: RepoSyncDriftState::NotApplicable,
        });
    }
    if repository.url.is_some() {
        let sync_lock = managed_repository_sync_lock(repository, cwd)?;
        let _sync_guard = sync_lock
            .lock()
            .map_err(|_| RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "managed repository sync lock is poisoned for repo `{}`",
                    repository.id
                ),
            })?;
        if matches!(sync_mode, RepositorySyncMode::Status) {
            return inspect_managed_repository_source(repository, cwd);
        }
        return materialize_managed_checkout(repository, cwd, sync_mode);
    }

    Err(RepoIntelligenceError::MissingRepositorySource {
        repo_id: repository.id.clone(),
    })
}

fn inspect_managed_repository_source(
    repository: &RegisteredRepository,
    cwd: &Path,
) -> Result<ResolvedRepositorySource, RepoIntelligenceError> {
    let mirror_root = managed_mirror_root(cwd, repository.id.as_str())?;
    let (mirror_root, mirror_state) = inspect_managed_mirror(repository, mirror_root.as_path())?;
    let checkout_root = managed_checkout_root(cwd, repository.id.as_str())?;
    let (checkout_root, checkout_state) =
        inspect_managed_checkout(repository, checkout_root.as_path())?;
    let mirror_revision = discover_mirror_revision(
        repository,
        mirror_root.as_path(),
        Some(checkout_root.as_path()),
    );
    let last_fetched_at = discover_last_fetched_at(mirror_root.as_path());
    let tracking_revision = discover_tracking_revision(repository, checkout_root.as_path());
    let drift_state = classify_managed_drift(
        checkout_root.as_path(),
        mirror_revision.as_deref(),
        tracking_revision.as_deref(),
    );

    Ok(ResolvedRepositorySource {
        checkout_root,
        mirror_root: Some(mirror_root),
        source_kind: ResolvedRepositorySourceKind::ManagedRemote,
        mirror_state,
        checkout_state,
        last_fetched_at,
        mirror_revision,
        tracking_revision,
        drift_state,
    })
}

fn inspect_managed_mirror(
    repository: &RegisteredRepository,
    root: &Path,
) -> Result<(PathBuf, RepositoryLifecycleState), RepoIntelligenceError> {
    if !root.exists() {
        return Ok((root.to_path_buf(), RepositoryLifecycleState::Missing));
    }

    let canonical_root =
        fs::canonicalize(root).map_err(|error| RepoIntelligenceError::InvalidRepositoryPath {
            repo_id: repository.id.clone(),
            path: root.display().to_string(),
            reason: format!("failed to canonicalize managed mirror: {error}"),
        })?;
    Repository::open_bare(&canonical_root).map_err(|error| {
        RepoIntelligenceError::InvalidRepositoryPath {
            repo_id: repository.id.clone(),
            path: canonical_root.display().to_string(),
            reason: format!("failed to open managed mirror: {error}"),
        }
    })?;

    Ok((canonical_root, RepositoryLifecycleState::Observed))
}

fn inspect_managed_checkout(
    repository: &RegisteredRepository,
    root: &Path,
) -> Result<(PathBuf, RepositoryLifecycleState), RepoIntelligenceError> {
    if !root.exists() {
        return Ok((root.to_path_buf(), RepositoryLifecycleState::Missing));
    }

    let checkout_root = resolve_local_checkout_root(&repository.id, root)?;
    Ok((checkout_root, RepositoryLifecycleState::Observed))
}

pub(crate) fn discover_checkout_metadata(repository_root: &Path) -> Option<LocalCheckoutMetadata> {
    let repository = Repository::discover(repository_root).ok()?;
    let revision = repository
        .head()
        .ok()
        .and_then(|head| head.target())
        .map(|oid| oid.to_string());
    let remote_url = repository
        .find_remote("origin")
        .ok()
        .and_then(|remote| remote.url().map(str::to_string))
        .or_else(|| {
            repository.remotes().ok().and_then(|remotes| {
                remotes.iter().flatten().find_map(|name| {
                    repository
                        .find_remote(name)
                        .ok()
                        .and_then(|remote| remote.url().map(str::to_string))
                })
            })
        });

    Some(LocalCheckoutMetadata {
        revision,
        remote_url,
    })
}

fn materialize_managed_checkout(
    repository: &RegisteredRepository,
    cwd: &Path,
    sync_mode: RepositorySyncMode,
) -> Result<ResolvedRepositorySource, RepoIntelligenceError> {
    let url = repository.url.as_deref().ok_or_else(|| {
        RepoIntelligenceError::MissingRepositorySource {
            repo_id: repository.id.clone(),
        }
    })?;
    let mirror_root = managed_mirror_root(cwd, repository.id.as_str())?;
    let mirror_state = prepare_managed_mirror(repository, url, mirror_root.as_path(), sync_mode)?;
    let canonical_mirror_root =
        fs::canonicalize(&mirror_root).map_err(|error| RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "failed to canonicalize managed mirror for repo `{}`: {error}",
                repository.id
            ),
        })?;
    let managed_root = managed_checkout_root(cwd, repository.id.as_str())?;
    let mirror_origin = canonical_mirror_root.display().to_string();

    if managed_root.exists() {
        let checkout_root = resolve_local_checkout_root(&repository.id, managed_root.as_path())?;
        let git_repository = Repository::open(&checkout_root).map_err(|error| {
            RepoIntelligenceError::InvalidRepositoryPath {
                repo_id: repository.id.clone(),
                path: checkout_root.display().to_string(),
                reason: format!("failed to open managed checkout: {error}"),
            }
        })?;
        ensure_origin_remote(&git_repository, mirror_origin.as_str()).map_err(|error| {
            RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to align mirror remote for managed checkout `{}`: {error}",
                    checkout_root.display()
                ),
            }
        })?;
        if should_refresh_managed_remote(repository, sync_mode) {
            refresh_managed_checkout(&git_repository, repository)?;
        }
        let mirror_revision = discover_mirror_revision(
            repository,
            canonical_mirror_root.as_path(),
            Some(checkout_root.as_path()),
        );
        let last_fetched_at = discover_last_fetched_at(canonical_mirror_root.as_path());
        let tracking_revision = discover_tracking_revision(repository, checkout_root.as_path());
        let drift_state = classify_managed_drift(
            checkout_root.as_path(),
            mirror_revision.as_deref(),
            tracking_revision.as_deref(),
        );
        return Ok(ResolvedRepositorySource {
            checkout_root,
            mirror_root: Some(canonical_mirror_root),
            source_kind: ResolvedRepositorySourceKind::ManagedRemote,
            mirror_state,
            checkout_state: if should_refresh_managed_remote(repository, sync_mode) {
                RepositoryLifecycleState::Refreshed
            } else {
                RepositoryLifecycleState::Reused
            },
            last_fetched_at,
            mirror_revision,
            tracking_revision,
            drift_state,
        });
    }

    fs::create_dir_all(
        managed_root
            .parent()
            .expect("managed checkout has a parent"),
    )
    .map_err(|error| RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "failed to create managed checkout parent for repo `{}`: {error}",
            repository.id
        ),
    })?;
    clone_managed_checkout(repository, mirror_origin.as_str(), managed_root.as_path())?;

    let checkout_root =
        fs::canonicalize(&managed_root).map_err(|error| RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "failed to canonicalize managed checkout for repo `{}`: {error}",
                repository.id
            ),
        })?;
    let mirror_revision = discover_mirror_revision(
        repository,
        canonical_mirror_root.as_path(),
        Some(checkout_root.as_path()),
    );
    let last_fetched_at = discover_last_fetched_at(canonical_mirror_root.as_path());
    let tracking_revision = discover_tracking_revision(repository, checkout_root.as_path());
    let drift_state = classify_managed_drift(
        checkout_root.as_path(),
        mirror_revision.as_deref(),
        tracking_revision.as_deref(),
    );
    Ok(ResolvedRepositorySource {
        checkout_root,
        mirror_root: Some(canonical_mirror_root),
        source_kind: ResolvedRepositorySourceKind::ManagedRemote,
        mirror_state,
        checkout_state: RepositoryLifecycleState::Created,
        last_fetched_at,
        mirror_revision,
        tracking_revision,
        drift_state,
    })
}

fn prepare_managed_mirror(
    repository: &RegisteredRepository,
    url: &str,
    destination: &Path,
    sync_mode: RepositorySyncMode,
) -> Result<RepositoryLifecycleState, RepoIntelligenceError> {
    if destination.exists() {
        let mirror = Repository::open_bare(destination).map_err(|error| {
            RepoIntelligenceError::InvalidRepositoryPath {
                repo_id: repository.id.clone(),
                path: destination.display().to_string(),
                reason: format!("failed to open managed mirror: {error}"),
            }
        })?;
        ensure_origin_remote(&mirror, url).map_err(|error| {
            RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "failed to align origin for managed mirror `{}`: {error}",
                    destination.display()
                ),
            }
        })?;
        if should_refresh_managed_remote(repository, sync_mode) {
            refresh_managed_mirror(&mirror, repository)?;
            return Ok(RepositoryLifecycleState::Refreshed);
        }
        return Ok(RepositoryLifecycleState::Reused);
    }

    fs::create_dir_all(destination.parent().expect("managed mirror has a parent")).map_err(
        |error| RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "failed to create managed mirror parent for repo `{}`: {error}",
                repository.id
            ),
        },
    )?;
    clone_managed_mirror(repository, url, destination)?;
    Ok(RepositoryLifecycleState::Created)
}

fn should_refresh_managed_remote(
    repository: &RegisteredRepository,
    sync_mode: RepositorySyncMode,
) -> bool {
    matches!(sync_mode, RepositorySyncMode::Refresh)
        || repository.refresh == RepositoryRefreshPolicy::Fetch
}

fn discover_mirror_revision(
    repository: &RegisteredRepository,
    mirror_root: &Path,
    checkout_root: Option<&Path>,
) -> Option<String> {
    let mirror = Repository::open_bare(mirror_root).ok()?;
    if let Some(branch) = tracked_branch_name(repository, checkout_root) {
        let reference_name = format!("refs/heads/{branch}");
        if let Ok(reference) = mirror.find_reference(&reference_name) {
            return reference.target().map(|oid| oid.to_string());
        }
    }

    mirror
        .head()
        .ok()
        .and_then(|head| head.target())
        .map(|oid| oid.to_string())
}

fn discover_last_fetched_at(mirror_root: &Path) -> Option<String> {
    let candidates = [mirror_root.join("FETCH_HEAD"), mirror_root.join("HEAD")];
    for candidate in candidates {
        if let Some(timestamp) = metadata_modified_rfc3339(candidate.as_path()) {
            return Some(timestamp);
        }
    }

    metadata_modified_rfc3339(mirror_root)
}

fn tracked_branch_name(
    repository: &RegisteredRepository,
    checkout_root: Option<&Path>,
) -> Option<String> {
    repository
        .git_ref
        .as_ref()
        .map(|git_ref| git_ref.as_str().to_string())
        .or_else(|| {
            checkout_root.and_then(|root| {
                Repository::open(root)
                    .ok()
                    .and_then(|git_repository| target_checkout_branch(&git_repository, repository))
            })
        })
}

fn discover_tracking_revision(
    repository: &RegisteredRepository,
    checkout_root: &Path,
) -> Option<String> {
    let git_repository = Repository::open(checkout_root).ok()?;
    let branch = target_checkout_branch(&git_repository, repository)?;
    let reference_name = format!("refs/remotes/origin/{branch}");
    git_repository
        .find_reference(&reference_name)
        .ok()
        .and_then(|reference| reference.target())
        .map(|oid| oid.to_string())
}

fn classify_managed_drift(
    checkout_root: &Path,
    mirror_revision: Option<&str>,
    tracking_revision: Option<&str>,
) -> RepoSyncDriftState {
    let checkout_revision =
        discover_checkout_metadata(checkout_root).and_then(|metadata| metadata.revision);
    let Some(checkout_revision) = checkout_revision.as_deref() else {
        return RepoSyncDriftState::Unknown;
    };
    let Some(mirror_revision) = mirror_revision else {
        return RepoSyncDriftState::Unknown;
    };

    if checkout_revision == mirror_revision {
        return RepoSyncDriftState::InSync;
    }

    let Some(tracking_revision) = tracking_revision else {
        return RepoSyncDriftState::Unknown;
    };
    let Ok(git_repository) = Repository::open(checkout_root) else {
        return RepoSyncDriftState::Unknown;
    };
    let Ok(local_oid) = Oid::from_str(checkout_revision) else {
        return RepoSyncDriftState::Unknown;
    };
    let Ok(tracking_oid) = Oid::from_str(tracking_revision) else {
        return RepoSyncDriftState::Unknown;
    };

    if tracking_revision == mirror_revision {
        return match git_repository.graph_ahead_behind(local_oid, tracking_oid) {
            Ok((ahead, 0)) if ahead > 0 => RepoSyncDriftState::Ahead,
            Ok((0, behind)) if behind > 0 => RepoSyncDriftState::Behind,
            Ok((ahead, behind)) if ahead > 0 && behind > 0 => RepoSyncDriftState::Diverged,
            _ => RepoSyncDriftState::Unknown,
        };
    }

    if tracking_revision == checkout_revision {
        return RepoSyncDriftState::Behind;
    }

    RepoSyncDriftState::Diverged
}

fn repo_cache_root(cwd: &Path) -> Result<PathBuf, RepoIntelligenceError> {
    let cache_home =
        resolve_cache_home(Some(cwd)).ok_or_else(|| RepoIntelligenceError::AnalysisFailed {
            message: "failed to resolve cache home for repo intelligence".to_string(),
        })?;
    Ok(cache_home.join("xiuxian-wendao").join("repo-intelligence"))
}

fn managed_mirror_root(cwd: &Path, repo_id: &str) -> Result<PathBuf, RepoIntelligenceError> {
    Ok(repo_cache_root(cwd)?
        .join("mirrors")
        .join(format!("{}.git", sanitize_repo_id(repo_id))))
}

fn managed_checkout_root(cwd: &Path, repo_id: &str) -> Result<PathBuf, RepoIntelligenceError> {
    Ok(repo_cache_root(cwd)?
        .join("repos")
        .join(sanitize_repo_id(repo_id)))
}

fn sanitize_repo_id(repo_id: &str) -> String {
    repo_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn managed_repository_sync_lock(
    repository: &RegisteredRepository,
    cwd: &Path,
) -> Result<Arc<Mutex<()>>, RepoIntelligenceError> {
    let checkout_root = managed_checkout_root(cwd, repository.id.as_str())?;
    let key = checkout_root.to_string_lossy().into_owned();
    let lock_registry = MANAGED_REPO_SYNC_LOCKS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut registry_guard =
        lock_registry
            .lock()
            .map_err(|_| RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "managed repository lock registry is poisoned for repo `{}`",
                    repository.id
                ),
            })?;
    Ok(registry_guard
        .entry(key)
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone())
}

fn clone_managed_checkout(
    repository: &RegisteredRepository,
    source: &str,
    destination: &Path,
) -> Result<Repository, RepoIntelligenceError> {
    let mut fetch_options = FetchOptions::new();
    fetch_options.download_tags(AutotagOption::All);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fetch_options);
    if let Some(git_ref) = repository.git_ref.as_ref() {
        builder.branch(git_ref.as_str());
    }

    builder
        .clone(source, destination)
        .map_err(|error| RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "failed to clone managed checkout for repo `{}` from `{source}`: {error}",
                repository.id
            ),
        })
}

fn clone_managed_mirror(
    repository: &RegisteredRepository,
    url: &str,
    destination: &Path,
) -> Result<Repository, RepoIntelligenceError> {
    let mut fetch_options = FetchOptions::new();
    fetch_options.download_tags(AutotagOption::All);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(fetch_options);
    builder.bare(true);

    builder
        .clone(url, destination)
        .map_err(|error| RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "failed to clone managed mirror for repo `{}` from `{url}`: {error}",
                repository.id
            ),
        })
}

fn ensure_origin_remote(repository: &Repository, url: &str) -> Result<(), git2::Error> {
    match repository.find_remote("origin") {
        Ok(remote) if remote.url() == Some(url) => Ok(()),
        Ok(_) => repository.remote_set_url("origin", url),
        Err(error) if error.code() == git2::ErrorCode::NotFound => {
            repository.remote("origin", url).map(|_| ())
        }
        Err(error) => Err(error),
    }
}

fn refresh_managed_mirror(
    git_repository: &Repository,
    repository: &RegisteredRepository,
) -> Result<(), RepoIntelligenceError> {
    let mut remote = git_repository.find_remote("origin").map_err(|error| {
        RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "failed to find origin remote for managed mirror `{}`: {error}",
                repository.id
            ),
        }
    })?;
    let mut fetch_options = FetchOptions::new();
    fetch_options.download_tags(AutotagOption::All);
    let refspecs = ["+refs/heads/*:refs/heads/*", "+refs/tags/*:refs/tags/*"];
    remote
        .fetch(&refspecs, Some(&mut fetch_options), None)
        .map_err(|error| RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "failed to refresh managed mirror `{}` from origin: {error}",
                repository.id
            ),
        })
}

fn refresh_managed_checkout(
    git_repository: &Repository,
    repository: &RegisteredRepository,
) -> Result<(), RepoIntelligenceError> {
    let mut remote = git_repository.find_remote("origin").map_err(|error| {
        RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "failed to find origin remote for managed repo `{}`: {error}",
                repository.id
            ),
        }
    })?;
    let mut fetch_options = FetchOptions::new();
    fetch_options.download_tags(AutotagOption::All);

    let refspecs = repository
        .git_ref
        .as_ref()
        .map(|git_ref| {
            vec![format!(
                "+refs/heads/{}:refs/remotes/origin/{}",
                git_ref.as_str(),
                git_ref.as_str()
            )]
        })
        .unwrap_or_default();
    let refspec_refs = refspecs.iter().map(String::as_str).collect::<Vec<_>>();
    remote
        .fetch(refspec_refs.as_slice(), Some(&mut fetch_options), None)
        .map_err(|error| RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "failed to fetch managed checkout `{}` from mirror: {error}",
                repository.id
            ),
        })?;

    if let Some(branch) = target_checkout_branch(git_repository, repository) {
        fast_forward_managed_checkout_branch(git_repository, repository, branch.as_str())?;
    }

    Ok(())
}

fn target_checkout_branch(
    repository: &Repository,
    config: &RegisteredRepository,
) -> Option<String> {
    config
        .git_ref
        .as_ref()
        .map(|git_ref| git_ref.as_str().to_string())
        .or_else(|| {
            repository.head().ok().and_then(|head| {
                if head.is_branch() {
                    head.shorthand().map(str::to_string)
                } else {
                    None
                }
            })
        })
}

fn fast_forward_managed_checkout_branch(
    git_repository: &Repository,
    repository: &RegisteredRepository,
    branch: &str,
) -> Result<(), RepoIntelligenceError> {
    match fast_forward_managed_branch(git_repository, branch) {
        Ok(()) => Ok(()),
        Err(error) if error.code() == git2::ErrorCode::Locked => {
            if clear_stale_checkout_index_lock(git_repository, repository)? {
                fast_forward_managed_branch(git_repository, branch).map_err(|retry_error| {
                    RepoIntelligenceError::AnalysisFailed {
                        message: format!(
                            "failed to fast-forward managed checkout `{}` to `{branch}` after clearing stale index lock: {retry_error}",
                            repository.id
                        ),
                    }
                })
            } else {
                Err(RepoIntelligenceError::AnalysisFailed {
                    message: format!(
                        "failed to fast-forward managed checkout `{}` to `{branch}` because checkout index is locked and lock is still active: {error}",
                        repository.id
                    ),
                })
            }
        }
        Err(error) => Err(RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "failed to fast-forward managed checkout `{}` to `{branch}`: {error}",
                repository.id
            ),
        }),
    }
}

fn clear_stale_checkout_index_lock(
    git_repository: &Repository,
    repository: &RegisteredRepository,
) -> Result<bool, RepoIntelligenceError> {
    let index_lock_path = git_repository.path().join("index.lock");
    if !index_lock_path.exists() {
        return Ok(false);
    }

    let modified = fs::metadata(&index_lock_path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "failed to inspect managed checkout index lock for repo `{}` at `{}`: {error}",
                repository.id,
                index_lock_path.display()
            ),
        })?;
    let lock_age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or_else(|_| Duration::ZERO);
    if lock_age < STALE_CHECKOUT_INDEX_LOCK_AGE {
        return Ok(false);
    }

    fs::remove_file(&index_lock_path).map_err(|error| RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "failed to remove stale managed checkout index lock for repo `{}` at `{}`: {error}",
            repository.id,
            index_lock_path.display()
        ),
    })?;
    Ok(true)
}

fn fast_forward_managed_branch(repository: &Repository, branch: &str) -> Result<(), git2::Error> {
    let remote_ref_name = format!("refs/remotes/origin/{branch}");
    let commit = repository
        .find_reference(&remote_ref_name)?
        .peel_to_commit()?;
    match repository.find_branch(branch, BranchType::Local) {
        Ok(local_branch) => {
            let mut reference = local_branch.into_reference();
            reference.set_target(commit.id(), "wendao repo intelligence fast-forward")?;
        }
        Err(error) if error.code() == git2::ErrorCode::NotFound => {
            repository.branch(branch, &commit, true)?;
        }
        Err(error) => return Err(error),
    }

    let local_ref_name = format!("refs/heads/{branch}");
    repository.set_head(&local_ref_name)?;
    let mut checkout = CheckoutBuilder::new();
    checkout.force();
    repository.checkout_head(Some(&mut checkout))?;
    Ok(())
}

fn metadata_modified_rfc3339(path: &Path) -> Option<String> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    let timestamp: DateTime<Utc> = modified.into();
    Some(timestamp.to_rfc3339())
}
