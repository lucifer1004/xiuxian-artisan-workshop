use std::time::Duration;

const MAX_SYNC_CONCURRENCY: usize = 16;
const MIN_ANALYSIS_TIMEOUT_SECS: u64 = 45;
const MAX_ANALYSIS_TIMEOUT_SECS: u64 = 180;
const ANALYSIS_TIMEOUT_BASE_SECS: u64 = 24;
const ANALYSIS_TIMEOUT_SECS_PER_CORE: u64 = 3;
const MIN_SYNC_TIMEOUT_SECS: u64 = 90;
const MAX_SYNC_TIMEOUT_SECS: u64 = 240;
const SYNC_TIMEOUT_BASE_SECS: u64 = 40;
const SYNC_TIMEOUT_SECS_PER_WORKER: u64 = 10;
const MAX_SYNC_REQUEUE_ATTEMPTS: usize = 3;

pub(crate) fn default_repo_index_sync_concurrency() -> usize {
    default_repo_index_sync_concurrency_for_parallelism(available_parallelism())
}

pub(crate) fn default_repo_index_sync_concurrency_for_parallelism(parallelism: usize) -> usize {
    let parallelism = parallelism.max(1);
    if parallelism == 1 {
        return 1;
    }

    parallelism
        .saturating_mul(2)
        .div_ceil(3)
        .clamp(2, MAX_SYNC_CONCURRENCY)
}

pub(crate) fn default_repo_index_sync_timeout() -> Duration {
    default_repo_index_sync_timeout_for_parallelism(available_parallelism())
}

pub(crate) fn default_repo_index_analysis_timeout() -> Duration {
    default_repo_index_analysis_timeout_for_parallelism(available_parallelism())
}

pub(crate) fn default_repo_index_sync_requeue_attempts() -> usize {
    default_repo_index_sync_requeue_attempts_for_parallelism(available_parallelism())
}

pub(crate) fn default_repo_index_sync_timeout_for_parallelism(parallelism: usize) -> Duration {
    let concurrency = u64::try_from(default_repo_index_sync_concurrency_for_parallelism(
        parallelism,
    ))
    .unwrap_or(u64::MAX);
    let timeout_secs = SYNC_TIMEOUT_BASE_SECS
        .saturating_add(concurrency.saturating_mul(SYNC_TIMEOUT_SECS_PER_WORKER))
        .clamp(MIN_SYNC_TIMEOUT_SECS, MAX_SYNC_TIMEOUT_SECS);
    Duration::from_secs(timeout_secs)
}

pub(crate) fn default_repo_index_analysis_timeout_for_parallelism(parallelism: usize) -> Duration {
    let parallelism = u64::try_from(parallelism.max(1)).unwrap_or(u64::MAX);
    let timeout_secs = ANALYSIS_TIMEOUT_BASE_SECS
        .saturating_add(parallelism.saturating_mul(ANALYSIS_TIMEOUT_SECS_PER_CORE))
        .clamp(MIN_ANALYSIS_TIMEOUT_SECS, MAX_ANALYSIS_TIMEOUT_SECS);
    Duration::from_secs(timeout_secs)
}

pub(crate) fn default_repo_index_sync_requeue_attempts_for_parallelism(
    parallelism: usize,
) -> usize {
    parallelism
        .max(1)
        .div_ceil(8)
        .clamp(1, MAX_SYNC_REQUEUE_ATTEMPTS)
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
        default_repo_index_analysis_timeout_for_parallelism,
        default_repo_index_sync_concurrency_for_parallelism,
        default_repo_index_sync_requeue_attempts_for_parallelism,
        default_repo_index_sync_timeout_for_parallelism,
    };

    #[test]
    fn repo_index_sync_concurrency_scales_with_parallelism() {
        assert_eq!(default_repo_index_sync_concurrency_for_parallelism(1), 1);
        assert_eq!(default_repo_index_sync_concurrency_for_parallelism(2), 2);
        assert_eq!(default_repo_index_sync_concurrency_for_parallelism(4), 3);
        assert_eq!(default_repo_index_sync_concurrency_for_parallelism(12), 8);
    }

    #[test]
    fn repo_index_sync_concurrency_caps_on_large_hosts() {
        assert_eq!(default_repo_index_sync_concurrency_for_parallelism(24), 16);
        assert_eq!(default_repo_index_sync_concurrency_for_parallelism(64), 16);
    }

    #[test]
    fn repo_index_sync_timeout_tracks_machine_tuned_sync_budget() {
        assert_eq!(
            default_repo_index_sync_timeout_for_parallelism(1),
            Duration::from_secs(90)
        );
        assert_eq!(
            default_repo_index_sync_timeout_for_parallelism(12),
            Duration::from_secs(120)
        );
        assert_eq!(
            default_repo_index_sync_timeout_for_parallelism(24),
            Duration::from_secs(200)
        );
    }

    #[test]
    fn repo_index_analysis_timeout_scales_with_parallelism() {
        assert_eq!(
            default_repo_index_analysis_timeout_for_parallelism(1),
            Duration::from_secs(45)
        );
        assert_eq!(
            default_repo_index_analysis_timeout_for_parallelism(12),
            Duration::from_secs(60)
        );
        assert_eq!(
            default_repo_index_analysis_timeout_for_parallelism(32),
            Duration::from_secs(120)
        );
    }

    #[test]
    fn repo_index_sync_requeue_attempts_scale_with_parallelism() {
        assert_eq!(
            default_repo_index_sync_requeue_attempts_for_parallelism(1),
            1
        );
        assert_eq!(
            default_repo_index_sync_requeue_attempts_for_parallelism(8),
            1
        );
        assert_eq!(
            default_repo_index_sync_requeue_attempts_for_parallelism(12),
            2
        );
        assert_eq!(
            default_repo_index_sync_requeue_attempts_for_parallelism(24),
            3
        );
        assert_eq!(
            default_repo_index_sync_requeue_attempts_for_parallelism(64),
            3
        );
    }
}
