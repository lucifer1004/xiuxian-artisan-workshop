use crate::SessionMemoryRecallDecision;

use super::types::{MemoryRecallLatencyBucketsSnapshot, MemoryRecallMetricsSnapshot};
use super::util::{now_unix_ms, ratio_as_f32};

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct MemoryRecallMetricsState {
    planned_total: u64,
    injected_total: u64,
    skipped_total: u64,
    selected_total: u64,
    injected_items_total: u64,
    context_chars_injected_total: u64,
    pipeline_duration_ms_total: u64,
    latency_buckets: MemoryRecallLatencyBucketsSnapshot,
    embedding_success_total: u64,
    embedding_timeout_total: u64,
    embedding_cooldown_reject_total: u64,
    embedding_unavailable_total: u64,
}

impl MemoryRecallMetricsState {
    pub(crate) fn observe_plan(&mut self) {
        self.planned_total = self.planned_total.saturating_add(1);
    }

    pub(crate) fn observe_result(
        &mut self,
        decision: SessionMemoryRecallDecision,
        recalled_selected: usize,
        recalled_injected: usize,
        context_chars_injected: usize,
        pipeline_duration_ms: u64,
    ) {
        match decision {
            SessionMemoryRecallDecision::Injected => {
                self.injected_total = self.injected_total.saturating_add(1);
            }
            SessionMemoryRecallDecision::Skipped => {
                self.skipped_total = self.skipped_total.saturating_add(1);
            }
        }

        self.selected_total = self.selected_total.saturating_add(recalled_selected as u64);
        self.injected_items_total = self
            .injected_items_total
            .saturating_add(recalled_injected as u64);
        self.context_chars_injected_total = self
            .context_chars_injected_total
            .saturating_add(context_chars_injected as u64);
        self.pipeline_duration_ms_total = self
            .pipeline_duration_ms_total
            .saturating_add(pipeline_duration_ms);
        self.observe_latency_bucket(pipeline_duration_ms);
    }

    fn observe_latency_bucket(&mut self, duration_ms: u64) {
        if duration_ms <= 10 {
            self.latency_buckets.le_10ms = self.latency_buckets.le_10ms.saturating_add(1);
        } else if duration_ms <= 25 {
            self.latency_buckets.le_25ms = self.latency_buckets.le_25ms.saturating_add(1);
        } else if duration_ms <= 50 {
            self.latency_buckets.le_50ms = self.latency_buckets.le_50ms.saturating_add(1);
        } else if duration_ms <= 100 {
            self.latency_buckets.le_100ms = self.latency_buckets.le_100ms.saturating_add(1);
        } else if duration_ms <= 250 {
            self.latency_buckets.le_250ms = self.latency_buckets.le_250ms.saturating_add(1);
        } else if duration_ms <= 500 {
            self.latency_buckets.le_500ms = self.latency_buckets.le_500ms.saturating_add(1);
        } else {
            self.latency_buckets.gt_500ms = self.latency_buckets.gt_500ms.saturating_add(1);
        }
    }

    pub(crate) fn observe_embedding_success(&mut self) {
        self.embedding_success_total = self.embedding_success_total.saturating_add(1);
    }

    pub(crate) fn observe_embedding_timeout(&mut self) {
        self.embedding_timeout_total = self.embedding_timeout_total.saturating_add(1);
    }

    pub(crate) fn observe_embedding_cooldown_reject(&mut self) {
        self.embedding_cooldown_reject_total =
            self.embedding_cooldown_reject_total.saturating_add(1);
    }

    pub(crate) fn observe_embedding_unavailable(&mut self) {
        self.embedding_unavailable_total = self.embedding_unavailable_total.saturating_add(1);
    }

    pub(crate) fn snapshot(self) -> MemoryRecallMetricsSnapshot {
        let completed_total = self.injected_total.saturating_add(self.skipped_total);
        MemoryRecallMetricsSnapshot {
            captured_at_unix_ms: now_unix_ms(),
            planned_total: self.planned_total,
            injected_total: self.injected_total,
            skipped_total: self.skipped_total,
            completed_total,
            selected_total: self.selected_total,
            injected_items_total: self.injected_items_total,
            context_chars_injected_total: self.context_chars_injected_total,
            pipeline_duration_ms_total: self.pipeline_duration_ms_total,
            avg_pipeline_duration_ms: ratio_as_f32(
                self.pipeline_duration_ms_total,
                completed_total,
            ),
            avg_selected_per_completed: ratio_as_f32(self.selected_total, completed_total),
            avg_injected_per_injected: ratio_as_f32(self.injected_items_total, self.injected_total),
            injected_rate: ratio_as_f32(self.injected_total, completed_total),
            latency_buckets: self.latency_buckets,
            embedding_success_total: self.embedding_success_total,
            embedding_timeout_total: self.embedding_timeout_total,
            embedding_cooldown_reject_total: self.embedding_cooldown_reject_total,
            embedding_unavailable_total: self.embedding_unavailable_total,
        }
    }
}
