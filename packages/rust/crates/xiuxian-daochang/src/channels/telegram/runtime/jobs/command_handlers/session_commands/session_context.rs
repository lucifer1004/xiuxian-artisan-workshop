use std::sync::Arc;

use tokio::sync::mpsc;

use crate::agent::Agent;
use crate::channels::traits::{Channel, ChannelMessage};
use crate::jobs::{
    JobCompletion, JobCompletionKind, JobHealthState, JobManager, JobMetricsSnapshot, JobState,
    JobStatusSnapshot,
};

use super::super::commands::{
    ResumeContextCommand, is_jobs_summary_command, is_reset_context_command,
    parse_background_prompt, parse_job_status_command, parse_resume_context_command,
};

pub(super) async fn handle_inbound_message(
    msg: ChannelMessage,
    channel: &Arc<dyn Channel>,
    foreground_tx: &mpsc::Sender<ChannelMessage>,
    job_manager: &Arc<JobManager>,
    agent: &Arc<Agent>,
) -> bool {
    let session_id = format!("{}:{}", msg.channel, msg.session_key);

    if is_reset_context_command(&msg.content) {
        let response = match agent.reset_context_window(&session_id).await {
            Ok(stats) => format!(
                "Session context reset.\nmessages_cleared={} summary_segments_cleared={}\nUse `/resume` to restore this session context.\nLong-term memory and knowledge stores are unchanged.",
                stats.messages, stats.summary_segments
            ),
            Err(error) => format!("Failed to reset session context: {error}"),
        };
        if let Err(error) = channel.send(&response, &msg.recipient).await {
            tracing::error!("Failed to send reset context response: {error}");
        }
        return true;
    }

    if let Some(resume_command) = parse_resume_context_command(&msg.content) {
        let response = match resume_command {
            ResumeContextCommand::Restore => match agent.resume_context_window(&session_id).await {
                Ok(Some(stats)) => format!(
                    "Session context restored.\nmessages_restored={} summary_segments_restored={}",
                    stats.messages, stats.summary_segments
                ),
                Ok(None) => {
                    "No saved session context snapshot found. Use `/reset` or `/clear` first."
                        .to_string()
                }
                Err(error) => format!("Failed to restore session context: {error}"),
            },
            ResumeContextCommand::Status => {
                match agent.peek_context_window_backup(&session_id).await {
                    Ok(Some(info)) => {
                        let mut lines = vec![
                            "Saved session context snapshot:".to_string(),
                            format!("messages={}", info.messages),
                            format!("summary_segments={}", info.summary_segments),
                        ];
                        if let Some(saved_at_unix_ms) = info.saved_at_unix_ms {
                            lines.push(format!("saved_at_unix_ms={saved_at_unix_ms}"));
                        }
                        if let Some(saved_age_secs) = info.saved_age_secs {
                            lines.push(format!("saved_age_secs={saved_age_secs}"));
                        }
                        lines.push("Use `/resume` to restore.".to_string());
                        lines.join("\n")
                    }
                    Ok(None) => "No saved session context snapshot found.".to_string(),
                    Err(error) => format!("Failed to inspect session context snapshot: {error}"),
                }
            }
        };
        if let Err(error) = channel.send(&response, &msg.recipient).await {
            tracing::error!("Failed to send resume context response: {error}");
        }
        return true;
    }

    if let Some(job_id) = parse_job_status_command(&msg.content) {
        let status_msg = match job_manager.get_status(&job_id).await {
            Some(snapshot) => format_job_status(&snapshot),
            None => format!("job `{job_id}` was not found"),
        };
        if let Err(error) = channel.send(&status_msg, &msg.recipient).await {
            tracing::error!("Failed to send job status: {error}");
        }
        return true;
    }

    if is_jobs_summary_command(&msg.content) {
        let metrics = job_manager.metrics().await;
        if let Err(error) = channel
            .send(&format_job_metrics(&metrics), &msg.recipient)
            .await
        {
            tracing::error!("Failed to send job metrics: {error}");
        }
        return true;
    }

    if let Some(prompt) = parse_background_prompt(&msg.content) {
        match job_manager
            .submit(&session_id, msg.recipient.clone(), prompt)
            .await
        {
            Ok(job_id) => {
                let ack = format!(
                    "Queued background job `{job_id}`.\nUse `/job {job_id}` for status, `/jobs` for queue health."
                );
                if let Err(error) = channel.send(&ack, &msg.recipient).await {
                    tracing::error!("Failed to send background ack: {error}");
                }
            }
            Err(error) => {
                let _ = channel
                    .send(
                        &format!("Failed to queue background job: {error}"),
                        &msg.recipient,
                    )
                    .await;
            }
        }
        return true;
    }

    if foreground_tx.send(msg).await.is_err() {
        tracing::error!("Foreground dispatcher is unavailable");
        return false;
    }
    true
}

pub(super) async fn push_background_completion(
    channel: &Arc<dyn Channel>,
    completion: JobCompletion,
) {
    let message = match completion.kind {
        JobCompletionKind::Succeeded { output } => {
            format!(
                "Background job `{}` completed.\n\n{}",
                completion.job_id, output
            )
        }
        JobCompletionKind::Failed { error } => {
            format!("Background job `{}` failed: {}", completion.job_id, error)
        }
        JobCompletionKind::TimedOut { timeout_secs } => format!(
            "Background job `{}` timed out after {}s.",
            completion.job_id, timeout_secs
        ),
    };
    if let Err(error) = channel.send(&message, &completion.recipient).await {
        tracing::error!("Failed to send background completion: {error}");
    }
}

fn format_job_status(snapshot: &JobStatusSnapshot) -> String {
    let mut lines = vec![
        format!("job `{}`", snapshot.job_id),
        format!("state: {}", format_job_state(snapshot.state)),
        format!("session: {}", snapshot.session_id),
        format!("prompt: {}", snapshot.prompt_preview),
        format!("submitted {}s ago", snapshot.submitted_age_secs),
    ];
    if let Some(running_age_secs) = snapshot.running_age_secs {
        lines.push(format!("running age: {}s", running_age_secs));
    }
    if let Some(finished_age_secs) = snapshot.finished_age_secs {
        lines.push(format!("finished {}s ago", finished_age_secs));
    }
    if let Some(ref output_preview) = snapshot.output_preview {
        lines.push(format!("output: {}", output_preview));
    }
    if let Some(ref error) = snapshot.error {
        lines.push(format!("error: {}", error));
    }
    lines.join("\n")
}

fn format_job_metrics(metrics: &JobMetricsSnapshot) -> String {
    let oldest_queued = metrics
        .oldest_queued_age_secs
        .map_or_else(|| "-".to_string(), |age| format!("{age}s"));
    let longest_running = metrics
        .longest_running_age_secs
        .map_or_else(|| "-".to_string(), |age| format!("{age}s"));
    format!(
        "background jobs: total={} queued={} running={} succeeded={} failed={} timed_out={}\noldest_queued={} longest_running={} health={}",
        metrics.total_jobs,
        metrics.queued,
        metrics.running,
        metrics.succeeded,
        metrics.failed,
        metrics.timed_out,
        oldest_queued,
        longest_running,
        format_job_health(metrics.health_state),
    )
}

fn format_job_state(state: JobState) -> &'static str {
    match state {
        JobState::Queued => "queued",
        JobState::Running => "running",
        JobState::Succeeded => "succeeded",
        JobState::Failed => "failed",
        JobState::TimedOut => "timed_out",
    }
}

fn format_job_health(state: JobHealthState) -> &'static str {
    match state {
        JobHealthState::Healthy => "healthy",
        JobHealthState::QueueStalled => "queue_stalled",
        JobHealthState::RunningStalled => "running_stalled",
    }
}
