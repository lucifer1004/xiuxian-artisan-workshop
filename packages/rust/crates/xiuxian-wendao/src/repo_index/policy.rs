/// Effective repo-index runtime policy values for diagnostics and performance
/// probes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RepoIndexPolicyDebugSnapshot {
    pub analysis_timeout_secs: u64,
    pub sync_timeout_secs: u64,
    pub sync_retry_budget: usize,
}

#[must_use]
pub(crate) fn repo_index_policy_debug_snapshot() -> RepoIndexPolicyDebugSnapshot {
    RepoIndexPolicyDebugSnapshot {
        analysis_timeout_secs: super::state::repo_index_analysis_timeout().as_secs(),
        sync_timeout_secs: super::state::repo_index_sync_timeout().as_secs(),
        sync_retry_budget: super::state::repo_index_sync_requeue_attempt_limit(),
    }
}
