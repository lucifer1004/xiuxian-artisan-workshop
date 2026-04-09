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
            (key == "XIUXIAN_GIT_REPO_MANAGED_REMOTE_RETRY_ATTEMPTS").then(|| "invalid".to_string())
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
            (key == "XIUXIAN_GIT_REPO_MANAGED_GIT_OPEN_RETRY_DELAY_MS").then(|| "180".to_string())
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
