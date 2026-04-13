use std::time::Duration;

use super::machine::{
    default_repo_index_analysis_timeout, default_repo_index_sync_concurrency,
    default_repo_index_sync_requeue_attempts, default_repo_index_sync_timeout,
};
use crate::analyzers::RepoIntelligenceError;
use xiuxian_config_core::lookup_positive_parsed;

const REPO_INDEX_ANALYSIS_TIMEOUT_ENV: &str = "XIUXIAN_WENDAO_REPO_INDEX_ANALYSIS_TIMEOUT_SECS";
const REPO_INDEX_SYNC_TIMEOUT_ENV: &str = "XIUXIAN_WENDAO_REPO_INDEX_SYNC_TIMEOUT_SECS";
const REPO_INDEX_SYNC_CONCURRENCY_ENV: &str = "XIUXIAN_WENDAO_REPO_INDEX_SYNC_CONCURRENCY";
const REPO_INDEX_SYNC_REQUEUE_ATTEMPTS_ENV: &str =
    "XIUXIAN_WENDAO_REPO_INDEX_SYNC_REQUEUE_ATTEMPTS";

pub(crate) fn repo_index_sync_concurrency_limit() -> usize {
    repo_index_sync_concurrency_limit_with_lookup(&|key| std::env::var(key).ok())
}

pub(crate) fn repo_index_analysis_timeout() -> Duration {
    repo_index_analysis_timeout_with_lookup(&|key| std::env::var(key).ok())
}

pub(crate) fn repo_index_sync_timeout() -> Duration {
    repo_index_sync_timeout_with_lookup(&|key| std::env::var(key).ok())
}

pub(crate) fn repo_index_sync_requeue_attempt_limit() -> usize {
    repo_index_sync_requeue_attempt_limit_with_lookup(&|key| std::env::var(key).ok())
}

fn repo_index_analysis_timeout_with_lookup(lookup: &dyn Fn(&str) -> Option<String>) -> Duration {
    lookup_positive_parsed::<u64>(REPO_INDEX_ANALYSIS_TIMEOUT_ENV, lookup)
        .map_or_else(default_repo_index_analysis_timeout, Duration::from_secs)
}

fn repo_index_sync_timeout_with_lookup(lookup: &dyn Fn(&str) -> Option<String>) -> Duration {
    lookup_positive_parsed::<u64>(REPO_INDEX_SYNC_TIMEOUT_ENV, lookup)
        .map_or_else(default_repo_index_sync_timeout, Duration::from_secs)
}

fn repo_index_sync_concurrency_limit_with_lookup(lookup: &dyn Fn(&str) -> Option<String>) -> usize {
    lookup_positive_parsed::<usize>(REPO_INDEX_SYNC_CONCURRENCY_ENV, lookup)
        .unwrap_or_else(default_repo_index_sync_concurrency)
}

fn repo_index_sync_requeue_attempt_limit_with_lookup(
    lookup: &dyn Fn(&str) -> Option<String>,
) -> usize {
    lookup_positive_parsed::<usize>(REPO_INDEX_SYNC_REQUEUE_ATTEMPTS_ENV, lookup)
        .unwrap_or_else(default_repo_index_sync_requeue_attempts)
}

pub(crate) fn should_retry_sync_failure(error: &RepoIntelligenceError, retry_count: usize) -> bool {
    retry_count < repo_index_sync_requeue_attempt_limit() && is_retryable_sync_failure(error)
}

pub(crate) fn should_penalize_adaptive_concurrency(error: &RepoIntelligenceError) -> bool {
    !matches!(
        error,
        RepoIntelligenceError::DuplicatePlugin { .. }
            | RepoIntelligenceError::MissingPlugin { .. }
            | RepoIntelligenceError::MissingRepoIntelligencePlugins { .. }
            | RepoIntelligenceError::MissingRequiredPlugin { .. }
            | RepoIntelligenceError::UnknownRepository { .. }
            | RepoIntelligenceError::MissingRepositoryPath { .. }
            | RepoIntelligenceError::MissingRepositorySource { .. }
            | RepoIntelligenceError::InvalidRepositoryPath { .. }
            | RepoIntelligenceError::UnsupportedRepositoryLayout { .. }
            | RepoIntelligenceError::PendingRepositoryIndex { .. }
            | RepoIntelligenceError::UnknownProjectedPage { .. }
            | RepoIntelligenceError::UnknownProjectedGap { .. }
            | RepoIntelligenceError::UnknownProjectedPageFamilyCluster { .. }
            | RepoIntelligenceError::UnknownProjectedPageIndexNode { .. }
            | RepoIntelligenceError::ConfigLoad { .. }
    )
}

fn is_retryable_sync_failure(error: &RepoIntelligenceError) -> bool {
    let message = match error {
        RepoIntelligenceError::AnalysisFailed { message } => message.as_str(),
        RepoIntelligenceError::InvalidRepositoryPath { reason, .. } => reason.as_str(),
        _ => return false,
    }
    .to_ascii_lowercase();
    [
        "can't assign requested address",
        "failed to connect to github.com",
        "failed to resolve address",
        "connection reset by peer",
        "temporary failure in name resolution",
        "resource temporarily unavailable",
        "operation timed out",
        "timed out",
        "too many open files",
    ]
    .iter()
    .any(|needle| message.contains(needle))
}

#[cfg(test)]
#[path = "../../../../tests/unit/repo_index/state/task/policy.rs"]
mod tests;
