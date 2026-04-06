use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use gix::bstr::ByteSlice;
use gix::refs::Target;
use gix::refs::transaction::{Change, LogChange, PreviousValue, RefEdit, RefLog};

use super::error::{BackendError, error_message};
use super::types::RepositoryHandle;

pub(crate) fn checkout_detached_to_revision(
    repository: &mut RepositoryHandle,
    revision: &str,
) -> Result<(), BackendError> {
    let workdir = repository
        .workdir()
        .ok_or_else(|| {
            BackendError::new(format!(
                "repository `{}` is bare and cannot serve as checkout",
                repository.git_dir().display()
            ))
        })?
        .to_path_buf();
    let current_head_target = repository
        .find_reference("HEAD")
        .map_err(error_message)?
        .target()
        .into_owned();
    let head_name = "HEAD".try_into().map_err(error_message)?;
    let current_index = repository
        .index_or_load_from_head_or_empty()
        .map_err(error_message)?
        .into_owned();
    let (target_commit_id, target_tree_id) = {
        let target_commit = repository
            .rev_parse_single(revision)
            .map_err(error_message)?
            .object()
            .map_err(error_message)?
            .peel_to_commit()
            .map_err(error_message)?;
        (
            target_commit.id,
            target_commit.tree_id().map_err(error_message)?.detach(),
        )
    };
    let mut target_index = repository
        .index_from_tree(&target_tree_id)
        .map_err(error_message)?;

    remove_tracked_paths_absent_from_target(&current_index, &target_index, &workdir)?;

    let should_interrupt = AtomicBool::new(false);
    let mut options = repository
        .checkout_options(gix::worktree::stack::state::attributes::Source::IdMapping)
        .map_err(error_message)?;
    options.overwrite_existing = true;
    let files = gix::progress::Discard;
    let bytes = gix::progress::Discard;
    let outcome = gix::worktree::state::checkout(
        &mut target_index,
        &workdir,
        repository
            .objects
            .clone()
            .into_arc()
            .map_err(error_message)?,
        &files,
        &bytes,
        &should_interrupt,
        options,
    )
    .map_err(error_message)?;
    ensure_checkout_outcome_clean(&outcome, revision)?;

    target_index
        .write(gix::index::write::Options::default())
        .map_err(error_message)?;
    repository
        .committer_or_set_generic_fallback()
        .map_err(error_message)?;
    repository
        .edit_reference(RefEdit {
            change: Change::Update {
                log: LogChange {
                    mode: RefLog::AndReference,
                    force_create_reflog: false,
                    message: format!("checkout: moving to {revision}").into(),
                },
                expected: PreviousValue::MustExistAndMatch(current_head_target),
                new: Target::Object(target_commit_id),
            },
            name: head_name,
            deref: false,
        })
        .map_err(error_message)?;
    Ok(())
}

fn remove_tracked_paths_absent_from_target(
    current_index: &gix::index::File,
    target_index: &gix::index::File,
    workdir: &Path,
) -> Result<(), BackendError> {
    let stale_paths: Vec<PathBuf> = current_index
        .entries_with_paths_by_filter_map(|path, _entry| {
            target_index
                .entry_index_by_path(path)
                .err()
                .map(|_| gix::path::from_bstr(path).into_owned())
        })
        .map(|(_path, path)| path)
        .collect();

    for stale_path in stale_paths {
        remove_path_if_present(workdir, &stale_path)?;
    }
    Ok(())
}

fn remove_path_if_present(workdir: &Path, relative_path: &Path) -> Result<(), BackendError> {
    let absolute_path = workdir.join(relative_path);
    match fs::symlink_metadata(&absolute_path) {
        Ok(metadata) => {
            if metadata.file_type().is_dir() && !metadata.file_type().is_symlink() {
                return Err(BackendError::new(format!(
                    "stale tracked path `{}` is backed by a directory; refusing recursive removal during detached checkout",
                    absolute_path.display()
                )));
            }
            fs::remove_file(&absolute_path).map_err(|error| {
                BackendError::new(format!(
                    "failed to remove stale tracked path `{}`: {error}",
                    absolute_path.display()
                ))
            })?;
            prune_empty_parents(workdir, absolute_path.parent())?;
            Ok(())
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(BackendError::new(format!(
            "failed to inspect stale tracked path `{}`: {error}",
            absolute_path.display()
        ))),
    }
}

fn prune_empty_parents(workdir: &Path, mut current: Option<&Path>) -> Result<(), BackendError> {
    while let Some(path) = current {
        if path == workdir {
            break;
        }
        match fs::remove_dir(path) {
            Ok(()) => current = path.parent(),
            Err(error)
                if matches!(
                    error.kind(),
                    ErrorKind::DirectoryNotEmpty | ErrorKind::NotFound
                ) =>
            {
                break;
            }
            Err(error) => {
                return Err(BackendError::new(format!(
                    "failed to prune empty parent directory `{}`: {error}",
                    path.display()
                )));
            }
        }
    }
    Ok(())
}

fn ensure_checkout_outcome_clean(
    outcome: &gix::worktree::state::checkout::Outcome,
    revision: &str,
) -> Result<(), BackendError> {
    if let Some(collision) = outcome.collisions.first() {
        return Err(BackendError::new(format!(
            "failed to checkout detached revision `{revision}` due to worktree collision at `{}`",
            gix::path::from_bstr(collision.path.as_bstr()).display()
        )));
    }
    if let Some(error) = outcome.errors.first() {
        return Err(BackendError::new(format!(
            "failed to checkout detached revision `{revision}` at `{}`: {}",
            gix::path::from_bstr(error.path.as_bstr()).display(),
            error.error
        )));
    }
    if let Some(path) = outcome.delayed_paths_unknown.first() {
        return Err(BackendError::new(format!(
            "failed to checkout detached revision `{revision}`: delayed filter path `{}` was unknown",
            gix::path::from_bstr(path.as_bstr()).display()
        )));
    }
    if let Some(path) = outcome.delayed_paths_unprocessed.first() {
        return Err(BackendError::new(format!(
            "failed to checkout detached revision `{revision}`: delayed filter path `{}` remained unprocessed",
            gix::path::from_bstr(path.as_bstr()).display()
        )));
    }
    Ok(())
}
