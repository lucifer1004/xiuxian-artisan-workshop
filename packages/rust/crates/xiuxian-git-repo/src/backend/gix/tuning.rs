use std::time::Duration;

const MIN_MANAGED_REMOTE_RETRY_ATTEMPTS: usize = 2;
const MAX_MANAGED_REMOTE_RETRY_ATTEMPTS: usize = 5;
const MANAGED_REMOTE_RETRY_ATTEMPTS_CORES_PER_STEP: usize = 6;
const MIN_MANAGED_GIT_OPEN_RETRY_ATTEMPTS: usize = 3;
const MAX_MANAGED_GIT_OPEN_RETRY_ATTEMPTS: usize = 8;
const MANAGED_GIT_OPEN_RETRY_ATTEMPTS_CORES_PER_STEP: usize = 4;
const MIN_MANAGED_GIT_OPEN_RETRY_DELAY_MS: u64 = 50;
const MAX_MANAGED_GIT_OPEN_RETRY_DELAY_MS: u64 = 150;
const MANAGED_GIT_OPEN_RETRY_DELAY_BASE_MS: u64 = 40;
const MANAGED_GIT_OPEN_RETRY_DELAY_MS_PER_CORE: u64 = 5;
const MIN_REMOTE_OPERATION_TIMEOUT_SECS: u64 = 45;
const MAX_REMOTE_OPERATION_TIMEOUT_SECS: u64 = 90;
const REMOTE_OPERATION_TIMEOUT_BASE_SECS: u64 = 30;
const REMOTE_OPERATION_TIMEOUT_SECS_PER_CORE: u64 = 3;

pub(crate) fn default_managed_remote_retry_attempts() -> usize {
    default_managed_remote_retry_attempts_for_parallelism(available_parallelism())
}

pub(crate) fn default_managed_remote_retry_attempts_for_parallelism(parallelism: usize) -> usize {
    1usize
        .saturating_add(
            parallelism
                .max(1)
                .div_ceil(MANAGED_REMOTE_RETRY_ATTEMPTS_CORES_PER_STEP),
        )
        .clamp(
            MIN_MANAGED_REMOTE_RETRY_ATTEMPTS,
            MAX_MANAGED_REMOTE_RETRY_ATTEMPTS,
        )
}

pub(crate) fn default_managed_git_open_retry_attempts() -> usize {
    default_managed_git_open_retry_attempts_for_parallelism(available_parallelism())
}

pub(crate) fn default_managed_git_open_retry_attempts_for_parallelism(parallelism: usize) -> usize {
    2usize
        .saturating_add(
            parallelism
                .max(1)
                .div_ceil(MANAGED_GIT_OPEN_RETRY_ATTEMPTS_CORES_PER_STEP),
        )
        .clamp(
            MIN_MANAGED_GIT_OPEN_RETRY_ATTEMPTS,
            MAX_MANAGED_GIT_OPEN_RETRY_ATTEMPTS,
        )
}

pub(crate) fn default_managed_git_open_retry_delay() -> Duration {
    default_managed_git_open_retry_delay_for_parallelism(available_parallelism())
}

pub(crate) fn default_managed_git_open_retry_delay_for_parallelism(parallelism: usize) -> Duration {
    let parallelism = u64::try_from(parallelism.max(1)).unwrap_or(u64::MAX);
    let delay_ms = MANAGED_GIT_OPEN_RETRY_DELAY_BASE_MS
        .saturating_add(parallelism.saturating_mul(MANAGED_GIT_OPEN_RETRY_DELAY_MS_PER_CORE))
        .clamp(
            MIN_MANAGED_GIT_OPEN_RETRY_DELAY_MS,
            MAX_MANAGED_GIT_OPEN_RETRY_DELAY_MS,
        );
    Duration::from_millis(delay_ms)
}

pub(crate) fn default_remote_operation_timeout() -> Duration {
    default_remote_operation_timeout_for_parallelism(available_parallelism())
}

pub(crate) fn default_remote_operation_timeout_for_parallelism(parallelism: usize) -> Duration {
    let parallelism = u64::try_from(parallelism.max(1)).unwrap_or(u64::MAX);
    let timeout_secs = REMOTE_OPERATION_TIMEOUT_BASE_SECS
        .saturating_add(parallelism.saturating_mul(REMOTE_OPERATION_TIMEOUT_SECS_PER_CORE))
        .clamp(
            MIN_REMOTE_OPERATION_TIMEOUT_SECS,
            MAX_REMOTE_OPERATION_TIMEOUT_SECS,
        );
    Duration::from_secs(timeout_secs)
}

fn available_parallelism() -> usize {
    std::thread::available_parallelism()
        .map(std::num::NonZeroUsize::get)
        .unwrap_or(1)
        .max(1)
}

#[cfg(test)]
mod tests {
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
}
