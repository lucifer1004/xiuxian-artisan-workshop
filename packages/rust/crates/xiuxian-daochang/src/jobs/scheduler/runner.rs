//! Recurring scheduler built on top of `JobManager`.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, bail};
use tokio::sync::mpsc;
use tokio::time::{Instant, MissedTickBehavior};

use crate::jobs::{JobCompletion, JobCompletionKind, JobManager};

/// Config for recurring scheduler runs.
#[derive(Debug, Clone)]
pub struct RecurringScheduleConfig {
    /// Logical schedule id for logs and session namespacing.
    pub schedule_id: String,
    /// Session prefix used by the queued jobs.
    pub session_prefix: String,
    /// Recipient identifier associated with queued jobs.
    pub recipient: String,
    /// Prompt executed on each schedule tick.
    pub prompt: String,
    /// Interval between submissions in seconds.
    pub interval_secs: u64,
    /// Optional run limit; `None` means run until Ctrl+C.
    pub max_runs: Option<u64>,
    /// Grace period to wait for in-flight completions before returning.
    pub wait_for_completion_secs: u64,
}

impl Default for RecurringScheduleConfig {
    fn default() -> Self {
        Self {
            schedule_id: "default".to_string(),
            session_prefix: "scheduler".to_string(),
            recipient: "scheduler".to_string(),
            prompt: String::new(),
            interval_secs: 300,
            max_runs: None,
            wait_for_completion_secs: 30,
        }
    }
}

/// Aggregated scheduler outcome counters.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RecurringScheduleOutcome {
    /// Number of submissions accepted by `JobManager`.
    pub submitted: u64,
    /// Number of completion events observed.
    pub completed: u64,
    /// Number of successful completions.
    pub succeeded: u64,
    /// Number of failed completions.
    pub failed: u64,
    /// Number of timed-out completions.
    pub timed_out: u64,
}

/// Run a recurring scheduler loop using an existing `JobManager`.
///
/// The loop submits one job per tick, collects completion events, and stops when:
/// - `max_runs` submissions are reached, or
/// - Ctrl+C is received.
pub async fn run_recurring_schedule(
    manager: Arc<JobManager>,
    mut completion_rx: mpsc::Receiver<JobCompletion>,
    mut config: RecurringScheduleConfig,
) -> Result<RecurringScheduleOutcome> {
    let prompt = config.prompt.trim().to_string();
    if prompt.is_empty() {
        bail!("schedule prompt cannot be empty");
    }
    if let Some(max_runs) = config.max_runs
        && max_runs == 0
    {
        bail!("max_runs must be greater than zero when provided");
    }

    config.interval_secs = config.interval_secs.max(1);
    config.wait_for_completion_secs = config.wait_for_completion_secs.max(1);
    config.schedule_id = normalize_or_default(&config.schedule_id, "default");
    config.session_prefix = normalize_or_default(&config.session_prefix, "scheduler");
    config.recipient = normalize_or_default(&config.recipient, "scheduler");

    let effective_session_prefix =
        format!("{}:schedule:{}", config.session_prefix, config.schedule_id);
    let mut ticker = tokio::time::interval(Duration::from_secs(config.interval_secs));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let mut outcome = RecurringScheduleOutcome::default();
    let mut interrupted = false;

    loop {
        let reached_limit = config
            .max_runs
            .is_some_and(|max_runs| outcome.submitted >= max_runs);
        if reached_limit || interrupted {
            break;
        }

        tokio::select! {
            _ = ticker.tick() => {
                let job_id = manager
                    .submit(
                        &effective_session_prefix,
                        config.recipient.clone(),
                        prompt.clone(),
                    )
                    .await?;
                outcome.submitted += 1;
                tracing::info!(
                    schedule_id = %config.schedule_id,
                    run = outcome.submitted,
                    interval_secs = config.interval_secs,
                    %job_id,
                    "scheduled background job queued"
                );
            }
            maybe_completion = completion_rx.recv() => {
                let Some(completion) = maybe_completion else {
                    break;
                };
                apply_completion(&mut outcome, &completion);
                tracing::info!(
                    schedule_id = %config.schedule_id,
                    job_id = %completion.job_id,
                    state = %completion_label(&completion.kind),
                    completed = outcome.completed,
                    submitted = outcome.submitted,
                    "scheduled background job completed"
                );
            }
            _ = tokio::signal::ctrl_c() => {
                interrupted = true;
                tracing::info!(
                    schedule_id = %config.schedule_id,
                    submitted = outcome.submitted,
                    "scheduler received Ctrl+C; stopping submissions"
                );
            }
        }
    }

    if outcome.completed < outcome.submitted {
        let deadline = Instant::now() + Duration::from_secs(config.wait_for_completion_secs);
        while outcome.completed < outcome.submitted {
            let now = Instant::now();
            if now >= deadline {
                break;
            }
            let wait = deadline - now;
            match tokio::time::timeout(wait, completion_rx.recv()).await {
                Ok(Some(completion)) => {
                    apply_completion(&mut outcome, &completion);
                    tracing::info!(
                        schedule_id = %config.schedule_id,
                        job_id = %completion.job_id,
                        state = %completion_label(&completion.kind),
                        completed = outcome.completed,
                        submitted = outcome.submitted,
                        "scheduled completion observed during drain"
                    );
                }
                Ok(None) | Err(_) => break,
            }
        }
    }

    if outcome.completed < outcome.submitted {
        tracing::warn!(
            schedule_id = %config.schedule_id,
            submitted = outcome.submitted,
            completed = outcome.completed,
            wait_for_completion_secs = config.wait_for_completion_secs,
            "scheduler exited before all queued jobs completed"
        );
    }

    Ok(outcome)
}

fn normalize_or_default(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn apply_completion(outcome: &mut RecurringScheduleOutcome, completion: &JobCompletion) {
    outcome.completed += 1;
    match completion.kind {
        JobCompletionKind::Succeeded { .. } => {
            outcome.succeeded += 1;
        }
        JobCompletionKind::Failed { .. } => {
            outcome.failed += 1;
        }
        JobCompletionKind::TimedOut { .. } => {
            outcome.timed_out += 1;
        }
    }
}

fn completion_label(kind: &JobCompletionKind) -> &'static str {
    match kind {
        JobCompletionKind::Succeeded { .. } => "succeeded",
        JobCompletionKind::Failed { .. } => "failed",
        JobCompletionKind::TimedOut { .. } => "timed_out",
    }
}
