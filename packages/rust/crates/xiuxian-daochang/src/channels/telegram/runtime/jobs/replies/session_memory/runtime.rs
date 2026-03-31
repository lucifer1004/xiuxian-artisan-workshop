use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::jobs::{JobRecord, JobState, QueuedJob, epoch_millis};

const JOB_MANAGER_STATE_VERSION: u32 = 1;

#[derive(Debug, Default)]
pub(super) struct JobRecoverySnapshot {
    pub(super) records: HashMap<String, JobRecord>,
    pub(super) queued_jobs: Vec<QueuedJob>,
    pub(super) recovered_queued: usize,
    pub(super) recovered_running: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedJobManagerState {
    version: u32,
    jobs: BTreeMap<String, PersistedJobRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PersistedJobRecord {
    session_id: String,
    recipient: String,
    parent_session_id: String,
    prompt: String,
    state: JobState,
    submitted_at_epoch_ms: u128,
    started_at_epoch_ms: Option<u128>,
    finished_at_epoch_ms: Option<u128>,
    output_preview: Option<String>,
    error: Option<String>,
}

pub(super) fn load_recovery_snapshot(path: &Path) -> Result<JobRecoverySnapshot> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(JobRecoverySnapshot::default());
        }
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "failed to read background job state file {}",
                    path.display()
                )
            });
        }
    };

    if String::from_utf8_lossy(&bytes).trim().is_empty() {
        return Ok(JobRecoverySnapshot::default());
    }

    let persisted: PersistedJobManagerState =
        serde_json::from_slice(&bytes).with_context(|| {
            format!(
                "failed to parse background job state file {}",
                path.display()
            )
        })?;
    if persisted.version != JOB_MANAGER_STATE_VERSION {
        return Err(anyhow!(
            "unsupported background job state version {} in {}",
            persisted.version,
            path.display()
        ));
    }

    let now_instant = Instant::now();
    let now_epoch_ms = epoch_millis();
    let mut snapshot = JobRecoverySnapshot::default();

    for (job_id, record) in persisted.jobs {
        let mut runtime_record = JobRecord {
            session_id: record.session_id.clone(),
            recipient: record.recipient.clone(),
            parent_session_id: record.parent_session_id.clone(),
            prompt: record.prompt.clone(),
            state: record.state,
            submitted_at: restore_instant(record.submitted_at_epoch_ms, now_instant, now_epoch_ms),
            started_at: record
                .started_at_epoch_ms
                .map(|epoch_ms| restore_instant(epoch_ms, now_instant, now_epoch_ms)),
            finished_at: record
                .finished_at_epoch_ms
                .map(|epoch_ms| restore_instant(epoch_ms, now_instant, now_epoch_ms)),
            output_preview: record.output_preview,
            error: record.error,
        };

        if matches!(runtime_record.state, JobState::Queued | JobState::Running) {
            if runtime_record.state == JobState::Queued {
                snapshot.recovered_queued = snapshot.recovered_queued.saturating_add(1);
            } else {
                snapshot.recovered_running = snapshot.recovered_running.saturating_add(1);
            }
            snapshot.queued_jobs.push(QueuedJob {
                job_id: job_id.clone(),
                recipient: runtime_record.recipient.clone(),
                parent_session_id: runtime_record.parent_session_id.clone(),
                session_id: runtime_record.session_id.clone(),
                prompt: runtime_record.prompt.clone(),
            });
            runtime_record.state = JobState::Queued;
            runtime_record.started_at = None;
            runtime_record.finished_at = None;
            runtime_record.output_preview = None;
            runtime_record.error = None;
        }

        snapshot.records.insert(job_id, runtime_record);
    }

    Ok(snapshot)
}

pub(super) fn persist_records(path: &Path, records: &HashMap<String, JobRecord>) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create background job state directory {}",
                parent.display()
            )
        })?;
    }

    let now_instant = Instant::now();
    let now_epoch_ms = epoch_millis();
    let jobs = records
        .iter()
        .map(|(job_id, record)| {
            (
                job_id.clone(),
                PersistedJobRecord {
                    session_id: record.session_id.clone(),
                    recipient: record.recipient.clone(),
                    parent_session_id: record.parent_session_id.clone(),
                    prompt: record.prompt.clone(),
                    state: record.state,
                    submitted_at_epoch_ms: instant_to_epoch_ms(
                        record.submitted_at,
                        now_instant,
                        now_epoch_ms,
                    ),
                    started_at_epoch_ms: record.started_at.map(|started_at| {
                        instant_to_epoch_ms(started_at, now_instant, now_epoch_ms)
                    }),
                    finished_at_epoch_ms: record.finished_at.map(|finished_at| {
                        instant_to_epoch_ms(finished_at, now_instant, now_epoch_ms)
                    }),
                    output_preview: record.output_preview.clone(),
                    error: record.error.clone(),
                },
            )
        })
        .collect();

    let payload = serde_json::to_vec_pretty(&PersistedJobManagerState {
        version: JOB_MANAGER_STATE_VERSION,
        jobs,
    })
    .context("failed to serialize background job state")?;

    let temp_path = temporary_persistence_path(path);
    std::fs::write(&temp_path, payload).with_context(|| {
        format!(
            "failed to write temporary background job state file {}",
            temp_path.display()
        )
    })?;
    std::fs::rename(&temp_path, path).with_context(|| {
        format!(
            "failed to move temporary background job state into place {}",
            path.display()
        )
    })?;
    Ok(())
}

fn restore_instant(epoch_ms: u128, now_instant: Instant, now_epoch_ms: u128) -> Instant {
    let age_ms = now_epoch_ms.saturating_sub(epoch_ms);
    let age_ms = u64::try_from(age_ms).unwrap_or(u64::MAX);
    now_instant
        .checked_sub(Duration::from_millis(age_ms))
        .unwrap_or(now_instant)
}

fn instant_to_epoch_ms(instant: Instant, now_instant: Instant, now_epoch_ms: u128) -> u128 {
    let age_ms = now_instant
        .checked_duration_since(instant)
        .map_or(0, |duration| duration.as_millis());
    now_epoch_ms.saturating_sub(age_ms)
}

fn temporary_persistence_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("background-jobs.json");
    path.with_file_name(format!("{file_name}.tmp"))
}
