//! Memory-recall metrics helpers exposed for integration tests.

use crate::agent::memory_recall_metrics as internal;
use crate::{MemoryRecallMetricsSnapshot, SessionMemoryRecallDecision};

/// Test-facing mutable memory-recall metrics state.
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryRecallMetricsState {
    inner: internal::MemoryRecallMetricsState,
}

impl MemoryRecallMetricsState {
    /// Observe one recall planning attempt.
    pub fn observe_plan(&mut self) {
        self.inner.observe_plan();
    }

    /// Observe one recall execution result.
    pub fn observe_result(
        &mut self,
        decision: SessionMemoryRecallDecision,
        recalled_selected: usize,
        recalled_injected: usize,
        context_chars_injected: usize,
        pipeline_duration_ms: u64,
    ) {
        self.inner.observe_result(
            decision,
            recalled_selected,
            recalled_injected,
            context_chars_injected,
            pipeline_duration_ms,
        );
    }

    /// Observe one successful embedding call.
    pub fn observe_embedding_success(&mut self) {
        self.inner.observe_embedding_success();
    }

    /// Observe one embedding timeout event.
    pub fn observe_embedding_timeout(&mut self) {
        self.inner.observe_embedding_timeout();
    }

    /// Observe one embedding cooldown reject event.
    pub fn observe_embedding_cooldown_reject(&mut self) {
        self.inner.observe_embedding_cooldown_reject();
    }

    /// Observe one embedding unavailable event.
    pub fn observe_embedding_unavailable(&mut self) {
        self.inner.observe_embedding_unavailable();
    }

    /// Materialize a metrics snapshot from current counters.
    #[must_use]
    pub fn snapshot(self) -> MemoryRecallMetricsSnapshot {
        self.inner.snapshot()
    }
}

/// Safe division helper used by metrics ratios.
#[must_use]
pub fn ratio_as_f32(numerator: u64, denominator: u64) -> f32 {
    internal::ratio_as_f32(numerator, denominator)
}
