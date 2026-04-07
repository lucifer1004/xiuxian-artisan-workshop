use std::thread;
use std::time::Duration;

use crate::spec::RepoRefreshPolicy;
use crate::sync::SyncMode;

use super::error::BackendError;
use super::tuning::{
    default_managed_git_open_retry_attempts, default_managed_git_open_retry_delay,
    default_managed_remote_retry_attempts,
};

const MANAGED_REMOTE_RETRY_ATTEMPTS_ENV: &str = "XIUXIAN_GIT_REPO_MANAGED_REMOTE_RETRY_ATTEMPTS";
const MANAGED_GIT_OPEN_RETRY_ATTEMPTS_ENV: &str =
    "XIUXIAN_GIT_REPO_MANAGED_GIT_OPEN_RETRY_ATTEMPTS";
const MANAGED_GIT_OPEN_RETRY_DELAY_MS_ENV: &str =
    "XIUXIAN_GIT_REPO_MANAGED_GIT_OPEN_RETRY_DELAY_MS";

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
    let max_attempts = managed_git_open_retry_attempts();
    let retry_delay = managed_git_open_retry_delay();
    let mut attempts = 0usize;
    loop {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error)
                if attempts + 1 < max_attempts
                    && retryable_git_open_error_message(error.message()) =>
            {
                attempts += 1;
                thread::sleep(retry_delay);
            }
            Err(error) => return Err(error),
        }
    }
}

pub(super) fn retry_remote_operation<T>(
    mut operation: impl FnMut() -> Result<T, BackendError>,
) -> Result<T, BackendError> {
    let retry_attempt_limit = managed_remote_retry_attempts();
    let mut attempt = 1usize;
    loop {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error) => {
                if attempt >= retry_attempt_limit
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

fn managed_remote_retry_attempts() -> usize {
    managed_remote_retry_attempts_with_lookup(&|key| std::env::var(key).ok())
}

fn managed_remote_retry_attempts_with_lookup(lookup: &dyn Fn(&str) -> Option<String>) -> usize {
    lookup(MANAGED_REMOTE_RETRY_ATTEMPTS_ENV)
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or_else(default_managed_remote_retry_attempts)
}

fn managed_git_open_retry_attempts() -> usize {
    managed_git_open_retry_attempts_with_lookup(&|key| std::env::var(key).ok())
}

fn managed_git_open_retry_attempts_with_lookup(lookup: &dyn Fn(&str) -> Option<String>) -> usize {
    lookup(MANAGED_GIT_OPEN_RETRY_ATTEMPTS_ENV)
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or_else(default_managed_git_open_retry_attempts)
}

fn managed_git_open_retry_delay() -> Duration {
    managed_git_open_retry_delay_with_lookup(&|key| std::env::var(key).ok())
}

fn managed_git_open_retry_delay_with_lookup(lookup: &dyn Fn(&str) -> Option<String>) -> Duration {
    lookup(MANAGED_GIT_OPEN_RETRY_DELAY_MS_ENV)
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map_or_else(default_managed_git_open_retry_delay, Duration::from_millis)
}

pub(super) fn retry_delay_for_attempt(attempt: usize) -> Duration {
    match attempt {
        0 | 1 => Duration::from_millis(250),
        2 => Duration::from_millis(500),
        _ => Duration::from_secs(1),
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        managed_git_open_retry_attempts_with_lookup, managed_git_open_retry_delay_with_lookup,
        managed_remote_retry_attempts_with_lookup,
    };
    use crate::backend::gix::tuning::{
        default_managed_git_open_retry_attempts, default_managed_git_open_retry_delay,
        default_managed_remote_retry_attempts,
    };

    #[test]
    fn managed_remote_retry_attempts_default_when_env_is_missing() {
        assert_eq!(
            managed_remote_retry_attempts_with_lookup(&|_| None),
            default_managed_remote_retry_attempts()
        );
    }

    #[test]
    fn managed_remote_retry_attempts_use_positive_override() {
        assert_eq!(
            managed_remote_retry_attempts_with_lookup(&|key| {
                (key == "XIUXIAN_GIT_REPO_MANAGED_REMOTE_RETRY_ATTEMPTS").then(|| "6".to_string())
            }),
            6
        );
    }

    #[test]
    fn managed_remote_retry_attempts_ignore_invalid_override() {
        assert_eq!(
            managed_remote_retry_attempts_with_lookup(&|key| {
                (key == "XIUXIAN_GIT_REPO_MANAGED_REMOTE_RETRY_ATTEMPTS")
                    .then(|| "invalid".to_string())
            }),
            default_managed_remote_retry_attempts()
        );
    }

    #[test]
    fn managed_git_open_retry_attempts_default_when_env_is_missing() {
        assert_eq!(
            managed_git_open_retry_attempts_with_lookup(&|_| None),
            default_managed_git_open_retry_attempts()
        );
    }

    #[test]
    fn managed_git_open_retry_attempts_use_positive_override() {
        assert_eq!(
            managed_git_open_retry_attempts_with_lookup(&|key| {
                (key == "XIUXIAN_GIT_REPO_MANAGED_GIT_OPEN_RETRY_ATTEMPTS").then(|| "7".to_string())
            }),
            7
        );
    }

    #[test]
    fn managed_git_open_retry_delay_default_when_env_is_missing() {
        assert_eq!(
            managed_git_open_retry_delay_with_lookup(&|_| None),
            default_managed_git_open_retry_delay()
        );
    }

    #[test]
    fn managed_git_open_retry_delay_uses_positive_override() {
        assert_eq!(
            managed_git_open_retry_delay_with_lookup(&|key| {
                (key == "XIUXIAN_GIT_REPO_MANAGED_GIT_OPEN_RETRY_DELAY_MS")
                    .then(|| "180".to_string())
            }),
            Duration::from_millis(180)
        );
    }

    #[test]
    fn managed_git_open_retry_delay_ignores_invalid_override() {
        assert_eq!(
            managed_git_open_retry_delay_with_lookup(&|key| {
                (key == "XIUXIAN_GIT_REPO_MANAGED_GIT_OPEN_RETRY_DELAY_MS")
                    .then(|| "invalid".to_string())
            }),
            default_managed_git_open_retry_delay()
        );
    }
}
