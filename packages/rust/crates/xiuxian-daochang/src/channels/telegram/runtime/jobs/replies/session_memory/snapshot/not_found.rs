use serde_json::json;

use crate::agent::{MemoryRecallMetricsSnapshot, MemoryRuntimeStatusSnapshot};
use crate::channels::telegram::runtime::jobs::replies::session_memory::metrics::format_memory_recall_metrics_json;
use crate::channels::telegram::runtime::jobs::replies::session_memory::runtime_status::{
    format_memory_runtime_status_json, format_memory_runtime_status_lines,
};

pub(in crate::channels::telegram::runtime::jobs) fn format_memory_recall_not_found(
    runtime_status: MemoryRuntimeStatusSnapshot,
    session_scope: &str,
) -> String {
    let mut lines = vec![
        "## Session Memory".to_string(),
        "No memory recall snapshot found for this session yet.".to_string(),
        format!("- Session scope: `{session_scope}`"),
        "".to_string(),
        "### Persistence".to_string(),
    ];
    lines.extend(format_memory_runtime_status_lines(runtime_status));
    lines.extend([
        "".to_string(),
        "### Next Step".to_string(),
        "- Send at least one normal turn first (non-command message).".to_string(),
        "- Then run `/session memory` again.".to_string(),
    ]);
    lines.join("\n")
}

pub(in crate::channels::telegram::runtime::jobs) fn format_memory_recall_not_found_json(
    metrics: MemoryRecallMetricsSnapshot,
    runtime_status: MemoryRuntimeStatusSnapshot,
    session_scope: &str,
) -> String {
    json!({
        "kind": "session_memory",
        "available": false,
        "session_scope": session_scope,
        "status": "not_found",
        "hint": "Run at least one normal turn first (non-command message).",
        "runtime": format_memory_runtime_status_json(runtime_status),
        "metrics": format_memory_recall_metrics_json(metrics),
    })
    .to_string()
}

pub(in crate::channels::telegram::runtime::jobs) fn format_memory_recall_not_found_telegram(
    runtime_status: MemoryRuntimeStatusSnapshot,
    session_scope: &str,
) -> String {
    format_memory_recall_not_found(runtime_status, session_scope)
}
