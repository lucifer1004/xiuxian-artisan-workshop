use crate::agent::MemoryRecallMetricsSnapshot;

pub(in super::super) fn format_memory_recall_metrics_lines(
    metrics: MemoryRecallMetricsSnapshot,
) -> Vec<String> {
    vec![
        format!("- `planned_total={}`", metrics.planned_total),
        format!(
            "- `completed_total={}` / `injected={}` / `skipped={}`",
            metrics.completed_total, metrics.injected_total, metrics.skipped_total
        ),
        format!(
            "- `selected_total={}` / `injected_items_total={}`",
            metrics.selected_total, metrics.injected_items_total
        ),
        format!(
            "- `context_chars_injected_total={}`",
            metrics.context_chars_injected_total
        ),
        format!(
            "- `avg_pipeline_duration_ms={:.2}` / `total_pipeline_duration_ms={}`",
            metrics.avg_pipeline_duration_ms, metrics.pipeline_duration_ms_total
        ),
        format!(
            "- `injected_rate={:.3}` / `avg_selected_per_completed={:.3}` / `avg_injected_per_injected={:.3}`",
            metrics.injected_rate,
            metrics.avg_selected_per_completed,
            metrics.avg_injected_per_injected
        ),
        format!(
            "- `embedding_success_total={}` / `embedding_timeout_total={}` / `embedding_cooldown_reject_total={}` / `embedding_unavailable_total={}`",
            metrics.embedding_success_total,
            metrics.embedding_timeout_total,
            metrics.embedding_cooldown_reject_total,
            metrics.embedding_unavailable_total
        ),
        format!(
            "- `latency_buckets_ms`: `<=10:{}` `<=25:{}` `<=50:{}` `<=100:{}` `<=250:{}` `<=500:{}` `>500:{}`",
            metrics.latency_buckets.le_10ms,
            metrics.latency_buckets.le_25ms,
            metrics.latency_buckets.le_50ms,
            metrics.latency_buckets.le_100ms,
            metrics.latency_buckets.le_250ms,
            metrics.latency_buckets.le_500ms,
            metrics.latency_buckets.gt_500ms
        ),
    ]
}
