use crate::search::SearchBuildLease;
use crate::search::coordinator::SearchCompactionReason;
use crate::search::service::core::types::SearchPlaneService;

pub(super) const COMPACTION_STARVATION_GUARD_ENQUEUE_LAG: u64 = 3;

impl SearchPlaneService {
    pub(crate) fn publish_ready_and_maintain(
        &self,
        lease: &SearchBuildLease,
        row_count: u64,
        fragment_count: u64,
    ) -> bool {
        self.coordinator
            .publish_ready(lease, row_count, fragment_count)
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
