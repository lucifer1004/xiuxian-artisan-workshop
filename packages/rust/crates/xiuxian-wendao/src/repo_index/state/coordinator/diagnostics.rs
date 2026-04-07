use crate::repo_index::state::coordinator::RepoIndexCoordinator;
use crate::repo_index::state::task::AdaptiveConcurrencyDebugSnapshot;

impl RepoIndexCoordinator {
    #[must_use]
    pub(crate) fn controller_debug_snapshot(&self) -> AdaptiveConcurrencyDebugSnapshot {
        self.concurrency
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .debug_snapshot()
    }
}
