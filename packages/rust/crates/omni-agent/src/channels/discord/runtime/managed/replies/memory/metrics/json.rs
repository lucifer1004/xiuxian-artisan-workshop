use serde_json::json;

use crate::agent::MemoryRecallMetricsSnapshot;

pub(in super::super) fn format_memory_recall_metrics_json(
    metrics: MemoryRecallMetricsSnapshot,
) -> serde_json::Value {
    json!({
        "captured_at_unix_ms": metrics.captured_at_unix_ms,
        "planned_total": metrics.planned_total,
        "injected_total": metrics.injected_total,
        "skipped_total": metrics.skipped_total,
        "completed_total": metrics.completed_total,
        "selected_total": metrics.selected_total,
        "injected_items_total": metrics.injected_items_total,
        "context_chars_injected_total": metrics.context_chars_injected_total,
        "pipeline_duration_ms_total": metrics.pipeline_duration_ms_total,
        "avg_pipeline_duration_ms": metrics.avg_pipeline_duration_ms,
        "avg_selected_per_completed": metrics.avg_selected_per_completed,
        "avg_injected_per_injected": metrics.avg_injected_per_injected,
        "injected_rate": metrics.injected_rate,
        "embedding_success_total": metrics.embedding_success_total,
        "embedding_timeout_total": metrics.embedding_timeout_total,
        "embedding_cooldown_reject_total": metrics.embedding_cooldown_reject_total,
        "embedding_unavailable_total": metrics.embedding_unavailable_total,
        "latency_buckets_ms": {
            "le_10ms": metrics.latency_buckets.le_10ms,
            "le_25ms": metrics.latency_buckets.le_25ms,
            "le_50ms": metrics.latency_buckets.le_50ms,
            "le_100ms": metrics.latency_buckets.le_100ms,
            "le_250ms": metrics.latency_buckets.le_250ms,
            "le_500ms": metrics.latency_buckets.le_500ms,
            "gt_500ms": metrics.latency_buckets.gt_500ms,
        },
    })
}
