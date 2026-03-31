use crate::{Agent, MemoryRecallMetricsSnapshot, SessionMemoryRecallDecision};

impl Agent {
    pub(crate) async fn record_memory_recall_plan_metrics(&self) {
        let mut guard = self.memory_recall_metrics.write().await;
        guard.observe_plan();
    }

    pub(crate) async fn record_memory_recall_result_metrics(
        &self,
        decision: SessionMemoryRecallDecision,
        recalled_selected: usize,
        recalled_injected: usize,
        context_chars_injected: usize,
        pipeline_duration_ms: u64,
    ) {
        let mut guard = self.memory_recall_metrics.write().await;
        guard.observe_result(
            decision,
            recalled_selected,
            recalled_injected,
            context_chars_injected,
            pipeline_duration_ms,
        );
    }

    pub(crate) async fn record_memory_embedding_success_metric(&self) {
        let mut guard = self.memory_recall_metrics.write().await;
        guard.observe_embedding_success();
    }

    pub(crate) async fn record_memory_embedding_timeout_metric(&self) {
        let mut guard = self.memory_recall_metrics.write().await;
        guard.observe_embedding_timeout();
    }

    pub(crate) async fn record_memory_embedding_cooldown_reject_metric(&self) {
        let mut guard = self.memory_recall_metrics.write().await;
        guard.observe_embedding_cooldown_reject();
    }

    pub(crate) async fn record_memory_embedding_unavailable_metric(&self) {
        let mut guard = self.memory_recall_metrics.write().await;
        guard.observe_embedding_unavailable();
    }

    /// Return current memory-recall metrics snapshot.
    pub async fn inspect_memory_recall_metrics(&self) -> MemoryRecallMetricsSnapshot {
        let guard = self.memory_recall_metrics.read().await;
        (*guard).snapshot()
    }
}
