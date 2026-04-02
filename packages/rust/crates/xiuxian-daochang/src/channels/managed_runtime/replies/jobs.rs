use serde_json::json;

use crate::jobs::{JobHealthState, JobMetricsSnapshot, JobState, JobStatusSnapshot};

pub(crate) fn format_job_status(snapshot: &JobStatusSnapshot) -> String {
    let mut lines = vec![
        "============================================================".to_string(),
        "job-status dashboard".to_string(),
        "============================================================".to_string(),
        "Overview:".to_string(),
        format!("  job_id={}", snapshot.job_id),
        format!("  state={}", format_job_state(snapshot.state)),
        "------------------------------------------------------------".to_string(),
        "Identity:".to_string(),
        format!("  session_id={}", snapshot.session_id),
        format!("  prompt_preview={}", snapshot.prompt_preview),
        "------------------------------------------------------------".to_string(),
        "Timing:".to_string(),
        format!("  submitted_age_secs={}", snapshot.submitted_age_secs),
        format!(
            "  running_age_secs={}",
            format_optional_u64(snapshot.running_age_secs)
        ),
        format!(
            "  finished_age_secs={}",
            format_optional_u64(snapshot.finished_age_secs)
        ),
        "------------------------------------------------------------".to_string(),
        "Result:".to_string(),
    ];
    if let Some(ref output_preview) = snapshot.output_preview {
        lines.push(format!("  output_preview={output_preview}"));
    } else {
        lines.push("  output_preview=-".to_string());
    }
    if let Some(ref error) = snapshot.error {
        lines.push(format!("  error={error}"));
    } else {
        lines.push("  error=-".to_string());
    }
    lines.extend([
        "------------------------------------------------------------".to_string(),
        "Hints:".to_string(),
        "  jobs_dashboard=/jobs".to_string(),
        "============================================================".to_string(),
    ]);
    lines.join("\n")
}

pub(crate) fn format_job_metrics(metrics: &JobMetricsSnapshot) -> String {
    let mut lines = vec![
        "============================================================".to_string(),
        "jobs-health dashboard".to_string(),
        "============================================================".to_string(),
        "Overview:".to_string(),
        format!("  total={}", metrics.total_jobs),
        format!("  queued={}", metrics.queued),
        format!("  running={}", metrics.running),
        format!("  succeeded={}", metrics.succeeded),
        format!("  failed={}", metrics.failed),
        format!("  timed_out={}", metrics.timed_out),
        "------------------------------------------------------------".to_string(),
        "Timing:".to_string(),
        format!(
            "  oldest_queued_age_secs={}",
            format_optional_u64(metrics.oldest_queued_age_secs)
        ),
        format!(
            "  longest_running_age_secs={}",
            format_optional_u64(metrics.longest_running_age_secs)
        ),
        "------------------------------------------------------------".to_string(),
        "Health:".to_string(),
        format!("  state={}", format_job_health(metrics.health_state)),
    ];
    lines.push(format!("  hint={}", format_job_health_hint(metrics)));
    lines.push("============================================================".to_string());
    lines.join("\n")
}

pub(crate) fn format_job_not_found(job_id: &str) -> String {
    [
        "============================================================".to_string(),
        "job-status dashboard".to_string(),
        "============================================================".to_string(),
        "Overview:".to_string(),
        format!("  job_id={job_id}"),
        "  status=not_found".to_string(),
        "------------------------------------------------------------".to_string(),
        "Hints:".to_string(),
        "  jobs_dashboard=/jobs".to_string(),
        "  submit_background=/bg <prompt>".to_string(),
        "============================================================".to_string(),
    ]
    .join("\n")
}

pub(crate) fn format_optional_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "-".to_string(), |age| age.to_string())
}

pub(crate) fn format_optional_usize(value: Option<usize>) -> String {
    value.map_or_else(|| "-".to_string(), |v| v.to_string())
}

pub(crate) fn format_optional_f32(value: Option<f32>) -> String {
    value.map_or_else(|| "-".to_string(), |v| format!("{v:.3}"))
}

pub(crate) fn format_job_status_json(snapshot: &JobStatusSnapshot) -> String {
    json!({
        "kind": "job_status",
        "found": true,
        "job_id": snapshot.job_id.clone(),
        "state": format_job_state(snapshot.state),
        "session_id": snapshot.session_id.clone(),
        "prompt_preview": snapshot.prompt_preview.clone(),
        "submitted_age_secs": snapshot.submitted_age_secs,
        "running_age_secs": snapshot.running_age_secs,
        "finished_age_secs": snapshot.finished_age_secs,
        "output_preview": snapshot.output_preview.clone(),
        "error": snapshot.error.clone(),
    })
    .to_string()
}

pub(crate) fn format_job_metrics_json(metrics: &JobMetricsSnapshot) -> String {
    json!({
        "kind": "jobs_health",
        "total": metrics.total_jobs,
        "queued": metrics.queued,
        "running": metrics.running,
        "succeeded": metrics.succeeded,
        "failed": metrics.failed,
        "timed_out": metrics.timed_out,
        "oldest_queued_age_secs": metrics.oldest_queued_age_secs,
        "longest_running_age_secs": metrics.longest_running_age_secs,
        "health": format_job_health(metrics.health_state),
        "hint": format_job_health_hint(metrics),
    })
    .to_string()
}

pub(crate) fn format_job_not_found_json(job_id: &str) -> String {
    json!({
        "kind": "job_status",
        "found": false,
        "job_id": job_id,
        "status": "not_found",
    })
    .to_string()
}

pub(crate) fn format_job_health_hint(metrics: &JobMetricsSnapshot) -> &'static str {
    if metrics.queued > 0 {
        "queued backlog present; use /job <id> for drill-down"
    } else if metrics.running > 0 {
        "jobs are in progress; use /job <id> for drill-down"
    } else {
        "no active jobs"
    }
}

pub(crate) fn format_job_state(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "queued",
        JobState::Running => "running",
        JobState::Succeeded => "succeeded",
        JobState::Failed => "failed",
        JobState::TimedOut => "timed_out",
    }
}

pub(crate) fn format_job_health(state: JobHealthState) -> &'static str {
    match state {
        JobHealthState::Healthy => "healthy",
        JobHealthState::QueueStalled => "queue_stalled",
        JobHealthState::RunningStalled => "running_stalled",
    }
}
