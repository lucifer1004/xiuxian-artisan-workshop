//! Background job queue for long-running turns (e.g. research-heavy requests).

mod heartbeat;
pub(crate) mod manager;
mod scheduler;

pub use heartbeat::{
    HeartbeatProbeState, JobHealthState, classify_heartbeat_probe_result, classify_job_health,
};
pub(crate) use manager::append_completion_to_parent_session;
pub use manager::{
    JobCompletion, JobCompletionKind, JobManager, JobManagerConfig, JobMetricsSnapshot, JobState,
    JobStatusSnapshot, TurnRunner,
};
pub(crate) use manager::{JobRecord, QueuedJob, epoch_millis};
pub use scheduler::{RecurringScheduleConfig, RecurringScheduleOutcome, run_recurring_schedule};
