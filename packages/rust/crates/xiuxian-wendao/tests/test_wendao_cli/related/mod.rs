#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::implicit_clone,
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::manual_string_new,
    clippy::needless_raw_string_hashes,
    clippy::format_push_string,
    clippy::map_unwrap_or,
    clippy::unnecessary_to_owned,
    clippy::too_many_lines
)]
use super::*;

mod related_command_accepts_ppr_flags;
mod related_verbose_includes_diagnostics;

fn assert_related_verbose_diagnostics(payload: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let diagnostics = payload
        .get("diagnostics")
        .ok_or("missing diagnostics payload")?;
    assert_eq!(diagnostics.get("alpha").and_then(Value::as_f64), Some(0.9));
    assert_eq!(
        diagnostics.get("max_iter").and_then(Value::as_u64),
        Some(64)
    );
    assert_eq!(diagnostics.get("tol").and_then(Value::as_f64), Some(1e-6));
    assert!(
        diagnostics
            .get("iteration_count")
            .and_then(Value::as_u64)
            .is_some()
    );
    assert!(
        diagnostics
            .get("final_residual")
            .and_then(Value::as_f64)
            .is_some()
    );
    assert!(
        diagnostics
            .get("candidate_count")
            .and_then(Value::as_u64)
            .is_some()
    );
    assert!(
        diagnostics
            .get("candidate_cap")
            .and_then(Value::as_u64)
            .is_some()
    );
    assert!(
        diagnostics
            .get("candidate_capped")
            .and_then(Value::as_bool)
            .is_some()
    );
    assert!(
        diagnostics
            .get("graph_node_count")
            .and_then(Value::as_u64)
            .is_some()
    );
    assert_eq!(
        diagnostics.get("subgraph_count").and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        diagnostics
            .get("partition_max_node_count")
            .and_then(Value::as_u64),
        Some(8)
    );
    assert_eq!(
        diagnostics
            .get("partition_min_node_count")
            .and_then(Value::as_u64),
        Some(8)
    );
    assert_eq!(
        diagnostics
            .get("partition_avg_node_count")
            .and_then(Value::as_f64),
        Some(8.0)
    );
    assert!(
        diagnostics
            .get("total_duration_ms")
            .and_then(Value::as_f64)
            .is_some()
    );
    assert!(
        diagnostics
            .get("partition_duration_ms")
            .and_then(Value::as_f64)
            .is_some()
    );
    assert!(
        diagnostics
            .get("kernel_duration_ms")
            .and_then(Value::as_f64)
            .is_some()
    );
    assert!(
        diagnostics
            .get("fusion_duration_ms")
            .and_then(Value::as_f64)
            .is_some()
    );
    assert_eq!(
        diagnostics.get("subgraph_mode").and_then(Value::as_str),
        Some("force")
    );
    assert_eq!(
        diagnostics
            .get("horizon_restricted")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert!(
        diagnostics
            .get("time_budget_ms")
            .and_then(Value::as_f64)
            .is_some()
    );
    assert!(
        diagnostics
            .get("timed_out")
            .and_then(Value::as_bool)
            .is_some()
    );
    Ok(())
}

fn assert_related_verbose_monitor(payload: &Value) -> Result<(), Box<dyn std::error::Error>> {
    let phases = payload
        .get("phases")
        .and_then(Value::as_array)
        .ok_or("missing monitor phases")?;
    assert!(
        phases.iter().any(|row| {
            row.get("phase").and_then(Value::as_str) == Some("link_graph.related.ppr")
        })
    );
    assert!(phases.iter().any(|row| {
        row.get("phase").and_then(Value::as_str) == Some("link_graph.related.subgraph.partition")
    }));
    assert!(phases.iter().any(|row| {
        row.get("phase").and_then(Value::as_str) == Some("link_graph.related.subgraph.fusion")
    }));
    assert!(phases.iter().any(|row| {
        row.get("phase").and_then(Value::as_str) == Some("link_graph.overlay.promoted")
    }));
    assert!(
        payload
            .get("monitor")
            .and_then(|row| row.get("bottlenecks"))
            .and_then(|row| row.get("slowest_phase"))
            .is_some()
    );

    let promoted_overlay = payload
        .get("promoted_overlay")
        .ok_or("missing promoted_overlay payload")?;
    assert!(
        promoted_overlay
            .get("applied")
            .and_then(Value::as_bool)
            .is_some()
    );
    assert_eq!(
        promoted_overlay.get("source").and_then(Value::as_str),
        Some("valkey.suggested_link_recent_latest")
    );
    Ok(())
}
