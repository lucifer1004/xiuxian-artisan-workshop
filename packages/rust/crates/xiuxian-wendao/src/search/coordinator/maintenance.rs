use crate::search::{SearchCorpusKind, SearchPlaneCoordinator};

use super::state::timestamp_now;

impl SearchPlaneCoordinator {
    /// Record that a staging or active epoch was successfully prewarmed.
    pub(crate) fn mark_prewarm_complete(&self, corpus: SearchCorpusKind, epoch: u64) -> bool {
        let mut state = self
            .state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(runtime) = state.get_mut(&corpus) else {
            return false;
        };
        if runtime.status.staging_epoch != Some(epoch) && runtime.status.active_epoch != Some(epoch)
        {
            return false;
        }

        let now = timestamp_now();
        runtime.status.maintenance.prewarm_running = false;
        runtime.status.maintenance.last_prewarmed_at = Some(now.clone());
        runtime.status.maintenance.last_prewarmed_epoch = Some(epoch);
        runtime.status.updated_at = Some(now);
        true
    }

    /// Record that a staging or active epoch prewarm has started.
    pub(crate) fn mark_prewarm_running(&self, corpus: SearchCorpusKind, epoch: u64) -> bool {
        self.set_prewarm_running(corpus, epoch, true)
    }

    /// Record that a staging or active epoch prewarm is no longer running.
    pub(crate) fn clear_prewarm_running(&self, corpus: SearchCorpusKind, epoch: u64) -> bool {
        self.set_prewarm_running(corpus, epoch, false)
    }

    fn set_prewarm_running(&self, corpus: SearchCorpusKind, epoch: u64, running: bool) -> bool {
        let mut state = self
            .state
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(runtime) = state.get_mut(&corpus) else {
            return false;
        };
        if runtime.status.staging_epoch != Some(epoch) && runtime.status.active_epoch != Some(epoch)
        {
            return false;
        }

        runtime.status.maintenance.prewarm_running = running;
        runtime.status.updated_at = Some(timestamp_now());
        true
    }
}
