use std::time::Duration;

use super::{
    default_managed_git_open_retry_attempts_for_parallelism,
    default_managed_git_open_retry_delay_for_parallelism,
    default_managed_remote_retry_attempts_for_parallelism,
    default_remote_operation_timeout_for_parallelism,
};

#[test]
fn managed_remote_retry_attempts_scale_with_parallelism() {
    assert_eq!(default_managed_remote_retry_attempts_for_parallelism(1), 2);
    assert_eq!(default_managed_remote_retry_attempts_for_parallelism(12), 3);
    assert_eq!(default_managed_remote_retry_attempts_for_parallelism(24), 5);
    assert_eq!(default_managed_remote_retry_attempts_for_parallelism(64), 5);
}

#[test]
fn managed_git_open_retry_attempts_scale_with_parallelism() {
    assert_eq!(
        default_managed_git_open_retry_attempts_for_parallelism(1),
        3
    );
    assert_eq!(
        default_managed_git_open_retry_attempts_for_parallelism(12),
        5
    );
    assert_eq!(
        default_managed_git_open_retry_attempts_for_parallelism(24),
        8
    );
    assert_eq!(
        default_managed_git_open_retry_attempts_for_parallelism(64),
        8
    );
}

#[test]
fn managed_git_open_retry_delay_scales_with_parallelism() {
    assert_eq!(
        default_managed_git_open_retry_delay_for_parallelism(1),
        Duration::from_millis(50)
    );
    assert_eq!(
        default_managed_git_open_retry_delay_for_parallelism(12),
        Duration::from_millis(100)
    );
    assert_eq!(
        default_managed_git_open_retry_delay_for_parallelism(24),
        Duration::from_millis(150)
    );
}

#[test]
fn default_remote_operation_timeout_respects_parallelism_floor() {
    assert_eq!(
        default_remote_operation_timeout_for_parallelism(1),
        Duration::from_secs(45)
    );
    assert_eq!(
        default_remote_operation_timeout_for_parallelism(4),
        Duration::from_secs(45)
    );
}

#[test]
fn default_remote_operation_timeout_scales_with_parallelism() {
    assert_eq!(
        default_remote_operation_timeout_for_parallelism(8),
        Duration::from_secs(54)
    );
    assert_eq!(
        default_remote_operation_timeout_for_parallelism(12),
        Duration::from_secs(66)
    );
}

#[test]
fn default_remote_operation_timeout_caps_on_large_hosts() {
    assert_eq!(
        default_remote_operation_timeout_for_parallelism(32),
        Duration::from_secs(90)
    );
}
