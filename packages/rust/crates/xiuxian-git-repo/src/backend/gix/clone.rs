use std::path::Path;

use gix::remote::Direction;

use super::constants::{MIRROR_FETCH_REFSPEC, ORIGIN_REMOTE_NAME};
use super::error::{BackendError, boxed_error, error_message};
use super::interrupt::run_interruptible_remote_operation;
use super::retry::retry_remote_operation;
use super::types::RepositoryHandle;

pub(crate) fn clone_bare_with_retry(
    upstream_url: &str,
    mirror_root: &Path,
) -> Result<RepositoryHandle, BackendError> {
    retry_remote_operation(|| clone_bare_once(upstream_url, mirror_root))
}

pub(crate) fn clone_checkout_from_mirror(
    mirror_origin: &str,
    checkout_root: &Path,
) -> Result<RepositoryHandle, BackendError> {
    run_interruptible_remote_operation("clone checkout from mirror", |should_interrupt| {
        let mut prepare = gix::prepare_clone(mirror_origin, checkout_root)
            .map_err(error_message)?
            .with_remote_name(ORIGIN_REMOTE_NAME)
            .map_err(error_message)?;
        let (mut checkout, _fetch_outcome) = prepare
            .fetch_then_checkout(gix::progress::Discard, should_interrupt)
            .map_err(error_message)?;
        let (repository, _checkout_outcome) = checkout
            .main_worktree(gix::progress::Discard, should_interrupt)
            .map_err(error_message)?;
        Ok(repository)
    })
}

fn clone_bare_once(
    upstream_url: &str,
    mirror_root: &Path,
) -> Result<RepositoryHandle, BackendError> {
    run_interruptible_remote_operation("clone bare mirror", |should_interrupt| {
        let mut prepare = gix::prepare_clone_bare(upstream_url, mirror_root)
            .map_err(error_message)?
            .with_remote_name(ORIGIN_REMOTE_NAME)
            .map_err(error_message)?
            .configure_remote(|remote| {
                let remote = remote
                    .with_refspecs(Some(MIRROR_FETCH_REFSPEC), Direction::Fetch)
                    .map_err(boxed_error)?;
                Ok(remote.with_fetch_tags(gix::remote::fetch::Tags::All))
            });
        let (repository, _outcome) = prepare
            .fetch_only(gix::progress::Discard, should_interrupt)
            .map_err(error_message)?;
        Ok(repository)
    })
}
