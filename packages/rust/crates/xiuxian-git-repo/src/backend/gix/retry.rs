use std::thread;
use std::time::Duration;

use crate::spec::RepoRefreshPolicy;
use crate::sync::SyncMode;

use super::constants::{
    MANAGED_GIT_OPEN_RETRY_ATTEMPTS, MANAGED_GIT_OPEN_RETRY_DELAY, MANAGED_REMOTE_RETRY_ATTEMPTS,
};
use super::error::BackendError;

pub(crate) fn should_fetch(refresh: RepoRefreshPolicy, mode: SyncMode) -> bool {
    matches!(mode, SyncMode::Refresh)
        || (matches!(mode, SyncMode::Ensure) && matches!(refresh, RepoRefreshPolicy::Fetch))
}

pub(crate) fn is_retryable_remote_error_message(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    [
        "can't assign requested address",
        "failed to connect",
        "could not connect",
        "timed out",
        "timeout",
        "temporary failure",
        "connection reset",
        "connection refused",
        "connection aborted",
        "network is unreachable",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub(super) fn retry_git_open_operation<T>(
    mut operation: impl FnMut() -> Result<T, BackendError>,
) -> Result<T, BackendError> {
    let mut attempts = 0usize;
    loop {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error)
                if attempts + 1 < MANAGED_GIT_OPEN_RETRY_ATTEMPTS
                    && retryable_git_open_error_message(error.message()) =>
            {
                attempts += 1;
                thread::sleep(MANAGED_GIT_OPEN_RETRY_DELAY);
            }
            Err(error) => return Err(error),
        }
    }
}

pub(super) fn retry_remote_operation<T>(
    mut operation: impl FnMut() -> Result<T, BackendError>,
) -> Result<T, BackendError> {
    let mut attempt = 1usize;
    loop {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error) => {
                if attempt >= MANAGED_REMOTE_RETRY_ATTEMPTS
                    || !is_retryable_remote_error_message(error.message())
                {
                    return Err(error);
                }
                thread::sleep(retry_delay_for_attempt(attempt));
                attempt += 1;
            }
        }
    }
}

fn retryable_git_open_error_message(message: &str) -> bool {
    message.to_ascii_lowercase().contains("too many open files")
}

pub(super) fn retry_delay_for_attempt(attempt: usize) -> Duration {
    match attempt {
        0 | 1 => Duration::from_millis(250),
        2 => Duration::from_millis(500),
        _ => Duration::from_secs(1),
    }
}
