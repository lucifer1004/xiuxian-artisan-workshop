use crate::search::SearchBuildLease;
use crate::search::coordinator::SearchCompactionReason;
use crate::search::service::core::types::SearchPlaneService;

pub(super) const COMPACTION_STARVATION_GUARD_ENQUEUE_LAG: u64 = 3;

impl SearchPlaneService {
    pub(crate) async fn publish_ready_and_maintain(
        &self,
        lease: &SearchBuildLease,
        row_count: u64,
        fragment_count: u64,
    ) -> bool {
        if !self
            .coordinator
            .publish_ready(lease, row_count, fragment_count)
        {
            return false;
        }
        let status = self.coordinator.status_for(lease.corpus);
        self.persist_local_corpus_manifest_status(&status).await;
        true
    }

    pub(super) const fn local_compaction_is_aged(
        reason: SearchCompactionReason,
        enqueue_sequence: u64,
        current_sequence: u64,
    ) -> bool {
        matches!(reason, SearchCompactionReason::RowDeltaRatio)
            && current_sequence.saturating_sub(enqueue_sequence)
                >= COMPACTION_STARVATION_GUARD_ENQUEUE_LAG
    }
}
