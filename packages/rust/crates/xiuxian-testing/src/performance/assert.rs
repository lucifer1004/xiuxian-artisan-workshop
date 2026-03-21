use std::fmt::Write;

use crate::performance::types::{PerfBudget, PerfReport};

/// Assert that a report meets a performance budget.
///
/// # Panics
///
/// Panics with a unified multi-line failure message when one or more budget
/// thresholds are exceeded.
#[track_caller]
pub fn assert_perf_budget(report: &PerfReport, budget: &PerfBudget) {
    let mut violations = Vec::new();

    if let Some(limit) = budget.max_p50_latency_ms
        && report.quantiles.p50_ms > limit
    {
        violations.push(format!(
            "p50 latency exceeded: actual={:.3}ms budget<={:.3}ms",
            report.quantiles.p50_ms, limit
        ));
    }

    if let Some(limit) = budget.max_p95_latency_ms
        && report.quantiles.p95_ms > limit
    {
        violations.push(format!(
            "p95 latency exceeded: actual={:.3}ms budget<={:.3}ms",
            report.quantiles.p95_ms, limit
        ));
    }

    if let Some(limit) = budget.max_p99_latency_ms
        && report.quantiles.p99_ms > limit
    {
        violations.push(format!(
            "p99 latency exceeded: actual={:.3}ms budget<={:.3}ms",
            report.quantiles.p99_ms, limit
        ));
    }

    if let Some(limit) = budget.min_throughput_qps
        && report.summary.throughput_qps < limit
    {
        violations.push(format!(
            "throughput below floor: actual={:.3}qps budget>={:.3}qps",
            report.summary.throughput_qps, limit
        ));
    }

    if let Some(limit) = budget.max_error_rate
        && report.summary.error_rate > limit
    {
        violations.push(format!(
            "error rate exceeded: actual={:.5} budget<={:.5}",
            report.summary.error_rate, limit
        ));
    }

    if violations.is_empty() {
        return;
    }

    let mut message = String::new();
    let _ = writeln!(message, "performance budget gate failed");
    let _ = writeln!(message, "suite: {}", report.suite);
    let _ = writeln!(message, "case: {}", report.case);
    let _ = writeln!(message, "mode: {}", report.mode);
    let _ = writeln!(
        message,
        "summary: p50={:.3}ms p95={:.3}ms p99={:.3}ms throughput={:.3}qps error_rate={:.5}",
        report.quantiles.p50_ms,
        report.quantiles.p95_ms,
        report.quantiles.p99_ms,
        report.summary.throughput_qps,
        report.summary.error_rate
    );
    let _ = writeln!(
        message,
        "counts: total={} success={} timeout={} error={}",
        report.summary.total_ops,
        report.summary.success_ops,
        report.summary.timeout_ops,
        report.summary.error_ops
    );
    let _ = writeln!(message, "violations:");
    for violation in violations {
        let _ = writeln!(message, "- {violation}");
    }
    if let Some(path) = &report.report_path {
        let _ = writeln!(message, "report_path: {path}");
    }

    panic!("{message}");
}
