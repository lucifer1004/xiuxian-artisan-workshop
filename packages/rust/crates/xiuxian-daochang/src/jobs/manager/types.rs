use std::time::{Instant, SystemTime, UNIX_EPOCH};

use anyhow::Result;
use async_trait::async_trait;

use crate::agent::Agent;

use crate::jobs::heartbeat::JobHealthState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    Succeeded,
    Failed,
    TimedOut,
}

#[derive(Debug, Clone)]
pub enum JobCompletionKind {
    Succeeded { output: String },
    Failed { error: String },
    TimedOut { timeout_secs: u64 },
}

#[derive(Debug, Clone)]
pub struct JobCompletion {
    pub job_id: String,
    pub recipient: String,
    pub parent_session_id: String,
    pub kind: JobCompletionKind,
}

impl JobCompletion {
    #[must_use]
    pub fn parent_session_key(&self) -> &str {
        &self.parent_session_id
    }
}

#[derive(Debug, Clone)]
pub struct JobStatusSnapshot {
    pub job_id: String,
    pub session_id: String,
    pub state: JobState,
    pub prompt_preview: String,
    pub submitted_age_secs: u64,
    pub running_age_secs: Option<u64>,
    pub finished_age_secs: Option<u64>,
    pub output_preview: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JobMetricsSnapshot {
    pub total_jobs: usize,
    pub queued: usize,
    pub running: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub timed_out: usize,
    pub oldest_queued_age_secs: Option<u64>,
    pub longest_running_age_secs: Option<u64>,
    pub health_state: JobHealthState,
}

#[derive(Debug, Clone)]
pub struct JobManagerConfig {
    pub queue_capacity: usize,
    pub max_in_flight: usize,
    pub job_timeout_secs: u64,
    pub heartbeat_interval_secs: u64,
    pub heartbeat_probe_timeout_secs: u64,
    pub max_queued_age_secs: u64,
    pub max_running_age_secs: u64,
}

impl Default for JobManagerConfig {
    fn default() -> Self {
        Self {
            queue_capacity: 64,
            max_in_flight: 4,
            job_timeout_secs: 300,
            heartbeat_interval_secs: 30,
            heartbeat_probe_timeout_secs: 5,
            max_queued_age_secs: 300,
            max_running_age_secs: 900,
        }
    }
}

#[async_trait]
pub trait TurnRunner: Send + Sync {
    async fn run_turn(&self, session_id: &str, prompt: &str) -> Result<String>;
}

#[async_trait]
impl TurnRunner for Agent {
    async fn run_turn(&self, session_id: &str, prompt: &str) -> Result<String> {
        Agent::run_turn(self, session_id, prompt).await
    }
}

#[derive(Debug, Clone)]
pub struct JobRecord {
    pub session_id: String,
    pub recipient: String,
    pub parent_session_id: String,
    pub prompt: String,
    pub state: JobState,
    pub submitted_at: Instant,
    pub started_at: Option<Instant>,
    pub finished_at: Option<Instant>,
    pub output_preview: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QueuedJob {
    pub job_id: String,
    pub recipient: String,
    pub parent_session_id: String,
    pub session_id: String,
    pub prompt: String,
}

#[must_use]
pub fn epoch_millis() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis().try_into().unwrap_or(u64::MAX),
        Err(_) => 0,
    }
}

#[must_use]
pub fn truncate_for_status(input: &str, max_chars: usize) -> String {
    if input.chars().count() <= max_chars {
        return input.to_string();
    }

    let head_len = max_chars.saturating_sub(3);
    let head = input.chars().take(head_len).collect::<String>();
    format!("{head}...")
}

#[must_use]
pub fn elapsed_secs_from(now: Instant, then: Instant) -> u64 {
    now.saturating_duration_since(then).as_secs()
}

#[cfg(test)]
mod tests {
    use super::{JobCompletion, JobCompletionKind, truncate_for_status};

    #[test]
    fn parent_session_key_returns_parent_session_id() {
        let completion = JobCompletion {
            job_id: "job-1".to_string(),
            recipient: "telegram:-1".to_string(),
            parent_session_id: "telegram:-1:42".to_string(),
            kind: JobCompletionKind::TimedOut { timeout_secs: 30 },
        };

        assert_eq!(completion.parent_session_key(), "telegram:-1:42");
    }

    #[test]
    fn truncate_for_status_adds_ellipsis_when_needed() {
        assert_eq!(truncate_for_status("abcdefghij", 6), "abc...");
        assert_eq!(truncate_for_status("short", 6), "short");
    }
}
