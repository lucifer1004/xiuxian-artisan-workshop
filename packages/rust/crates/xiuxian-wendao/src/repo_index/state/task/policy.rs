use std::time::Duration;

use super::machine::{
    default_repo_index_analysis_timeout, default_repo_index_sync_concurrency,
    default_repo_index_sync_requeue_attempts, default_repo_index_sync_timeout,
};
use crate::analyzers::RepoIntelligenceError;

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
    lookup(REPO_INDEX_ANALYSIS_TIMEOUT_ENV)
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map_or_else(default_repo_index_analysis_timeout, Duration::from_secs)
}

fn repo_index_sync_timeout_with_lookup(lookup: &dyn Fn(&str) -> Option<String>) -> Duration {
    lookup(REPO_INDEX_SYNC_TIMEOUT_ENV)
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map_or_else(default_repo_index_sync_timeout, Duration::from_secs)
}

fn repo_index_sync_concurrency_limit_with_lookup(lookup: &dyn Fn(&str) -> Option<String>) -> usize {
    lookup(REPO_INDEX_SYNC_CONCURRENCY_ENV)
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or_else(default_repo_index_sync_concurrency)
}

fn repo_index_sync_requeue_attempt_limit_with_lookup(
    lookup: &dyn Fn(&str) -> Option<String>,
) -> usize {
    lookup(REPO_INDEX_SYNC_REQUEUE_ATTEMPTS_ENV)
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
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
mod tests {
    use super::{
        repo_index_analysis_timeout_with_lookup, repo_index_sync_concurrency_limit_with_lookup,
        repo_index_sync_requeue_attempt_limit_with_lookup, repo_index_sync_timeout_with_lookup,
        should_penalize_adaptive_concurrency, should_retry_sync_failure,
    };
    use crate::analyzers::RepoIntelligenceError;
    use crate::repo_index::state::task::machine::{
        default_repo_index_analysis_timeout, default_repo_index_sync_concurrency,
        default_repo_index_sync_requeue_attempts, default_repo_index_sync_timeout,
    };

    #[test]
    fn repo_index_analysis_timeout_defaults_when_env_is_missing() {
        let timeout = repo_index_analysis_timeout_with_lookup(&|_| None);
        assert_eq!(timeout, default_repo_index_analysis_timeout());
    }

    #[test]
    fn repo_index_analysis_timeout_uses_positive_override() {
        let timeout = repo_index_analysis_timeout_with_lookup(&|key| {
            (key == "XIUXIAN_WENDAO_REPO_INDEX_ANALYSIS_TIMEOUT_SECS").then(|| "75".to_string())
        });
        assert_eq!(timeout.as_secs(), 75);
    }

    #[test]
    fn repo_index_analysis_timeout_ignores_invalid_override() {
        let timeout = repo_index_analysis_timeout_with_lookup(&|key| {
            (key == "XIUXIAN_WENDAO_REPO_INDEX_ANALYSIS_TIMEOUT_SECS")
                .then(|| "invalid".to_string())
        });
        assert_eq!(timeout, default_repo_index_analysis_timeout());
    }

    #[test]
    fn repo_index_sync_timeout_defaults_when_env_is_missing() {
        let timeout = repo_index_sync_timeout_with_lookup(&|_| None);
        assert_eq!(timeout, default_repo_index_sync_timeout());
    }

    #[test]
    fn repo_index_sync_timeout_uses_positive_override() {
        let timeout = repo_index_sync_timeout_with_lookup(&|key| {
            (key == "XIUXIAN_WENDAO_REPO_INDEX_SYNC_TIMEOUT_SECS").then(|| "240".to_string())
        });
        assert_eq!(timeout.as_secs(), 240);
    }

    #[test]
    fn repo_index_sync_timeout_ignores_invalid_override() {
        let timeout = repo_index_sync_timeout_with_lookup(&|key| {
            (key == "XIUXIAN_WENDAO_REPO_INDEX_SYNC_TIMEOUT_SECS").then(|| "invalid".to_string())
        });
        assert_eq!(timeout, default_repo_index_sync_timeout());
    }

    #[test]
    fn repo_index_sync_concurrency_limit_defaults_when_env_is_missing() {
        let limit = repo_index_sync_concurrency_limit_with_lookup(&|_| None);
        assert_eq!(limit, default_repo_index_sync_concurrency());
    }

    #[test]
    fn repo_index_sync_concurrency_limit_uses_positive_override() {
        let limit = repo_index_sync_concurrency_limit_with_lookup(&|key| {
            (key == "XIUXIAN_WENDAO_REPO_INDEX_SYNC_CONCURRENCY").then(|| "3".to_string())
        });
        assert_eq!(limit, 3);
    }

    #[test]
    fn repo_index_sync_concurrency_limit_ignores_invalid_override() {
        let limit = repo_index_sync_concurrency_limit_with_lookup(&|key| {
            (key == "XIUXIAN_WENDAO_REPO_INDEX_SYNC_CONCURRENCY").then(|| "invalid".to_string())
        });
        assert_eq!(limit, default_repo_index_sync_concurrency());
    }

    #[test]
    fn repo_index_sync_requeue_attempt_limit_defaults_when_env_is_missing() {
        let limit = repo_index_sync_requeue_attempt_limit_with_lookup(&|_| None);
        assert_eq!(limit, default_repo_index_sync_requeue_attempts());
    }

    #[test]
    fn repo_index_sync_requeue_attempt_limit_uses_positive_override() {
        let limit = repo_index_sync_requeue_attempt_limit_with_lookup(&|key| {
            (key == "XIUXIAN_WENDAO_REPO_INDEX_SYNC_REQUEUE_ATTEMPTS").then(|| "4".to_string())
        });
        assert_eq!(limit, 4);
    }

    #[test]
    fn repo_index_sync_requeue_attempt_limit_ignores_invalid_override() {
        let limit = repo_index_sync_requeue_attempt_limit_with_lookup(&|key| {
            (key == "XIUXIAN_WENDAO_REPO_INDEX_SYNC_REQUEUE_ATTEMPTS")
                .then(|| "invalid".to_string())
        });
        assert_eq!(limit, default_repo_index_sync_requeue_attempts());
    }

    #[test]
    fn retryable_sync_failure_matches_transient_network_transport_errors() {
        let error = RepoIntelligenceError::AnalysisFailed {
            message: "failed to refresh managed mirror `DifferentialEquations.jl` from `https://github.com/SciML/DifferentialEquations.jl.git`: failed to connect to github.com: Can't assign requested address; class=Os (2)".to_string(),
        };
        assert!(should_retry_sync_failure(&error, 0));
    }

    #[test]
    fn retryable_sync_failure_stops_after_retry_budget_is_exhausted() {
        let error = RepoIntelligenceError::AnalysisFailed {
            message:
                "failed to refresh managed mirror `DifferentialEquations.jl`: operation timed out"
                    .to_string(),
        };
        let retry_limit = repo_index_sync_requeue_attempt_limit_with_lookup(&|_| None);
        assert!(!should_retry_sync_failure(&error, retry_limit));
    }

    #[test]
    fn retryable_sync_failure_rejects_non_transport_errors() {
        let error = RepoIntelligenceError::MissingRepositorySource {
            repo_id: "DifferentialEquations.jl".to_string(),
        };
        assert!(!should_retry_sync_failure(&error, 0));
    }

    #[test]
    fn retryable_sync_failure_matches_descriptor_pressure_errors() {
        let error = RepoIntelligenceError::AnalysisFailed {
            message: "failed to acquire managed checkout lock `/tmp/example.lock`: Too many open files (os error 24)".to_string(),
        };
        assert!(should_retry_sync_failure(&error, 0));
    }

    #[test]
    fn retryable_sync_failure_matches_retryable_invalid_repository_path_reasons() {
        let error = RepoIntelligenceError::InvalidRepositoryPath {
            repo_id: "DifferentialEquations.jl".to_string(),
            path: "/tmp/example.git".to_string(),
            reason: "failed to open managed mirror as bare git repository: could not open '/tmp/example.git/config': Too many open files; class=Os (2)".to_string(),
        };
        assert!(should_retry_sync_failure(&error, 0));
    }

    #[test]
    fn unsupported_layout_does_not_penalize_adaptive_concurrency() {
        let error = RepoIntelligenceError::UnsupportedRepositoryLayout {
            repo_id: "Sundials.jl".to_string(),
            message: "missing Project.toml".to_string(),
        };

        assert!(!should_penalize_adaptive_concurrency(&error));
    }

    #[test]
    fn analysis_failures_still_penalize_adaptive_concurrency() {
        let error = RepoIntelligenceError::AnalysisFailed {
            message: "transport timed out".to_string(),
        };

        assert!(should_penalize_adaptive_concurrency(&error));
    }
}
