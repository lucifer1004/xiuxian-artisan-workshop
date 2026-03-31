use anyhow::Result;
use base64::Engine;
use fs2::FileExt;
use sha2::{Digest, Sha256};
use std::fs::{File, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::{Semaphore, oneshot};
use tokio::time::MissedTickBehavior;
use xiuxian_config_core::{absolutize_path, resolve_project_root_or_cwd};
use xiuxian_llm::llm::multimodal::Base64ImageSource;
use xiuxian_llm::llm::vision::deepseek::preprocess_image_for_ocr;
use xiuxian_llm::llm::vision::{
    DeepseekRuntime, PreparedVisionImage, get_deepseek_runtime, infer_deepseek_ocr_truth,
};

const OCR_TRUTH_HEADER: &str =
    "[PHYSICAL_OCR_TRUTH]: The following is a high-fidelity Markdown reconstruction of the image.";
const OCR_TRUTH_FOOTER: &str =
    "[INSTRUCTION]: Use this truth to answer the user query and keep the answer grounded.";
const DEFAULT_OCR_MAX_DIMENSION: u32 = 1_024;
const OCR_WORKER_PANIC_PREFIX: &str = "deepseek_ocr_worker_panicked:";
const OCR_RSS_GIB_SCALE: u128 = 1_000_000;
const OCR_BYTES_PER_GIB_U128: u128 = 1024_u128 * 1024_u128 * 1024_u128;
const OCR_BYTES_PER_GIB_U64: u64 = 1024_u64 * 1024_u64 * 1024_u64;

enum OcrBlockingExecution<T> {
    Busy,
    BusyBackpressure,
    MemoryGuard { rss_bytes: u64, limit_bytes: u64 },
    Completed(T),
    RefineFailed(anyhow::Error),
    Panicked(String),
    ChannelClosed,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::llm) enum OcrProbeFirstOutcome {
    TimedOut,
    Panicked,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::llm) struct OcrGateTimeoutRecoveryProbe {
    pub(in crate::llm) first_outcome: OcrProbeFirstOutcome,
    pub(in crate::llm) second_was_busy: bool,
    pub(in crate::llm) second_completed: bool,
    pub(in crate::llm) recovered_after_wait: bool,
}

#[derive(Debug, Clone, Copy)]
struct OcrTimeoutConfig {
    duration: Duration,
    stage: &'static str,
}

type OcrWorkerSpawn<T> = (
    oneshot::Receiver<Result<T, anyhow::Error>>,
    Arc<AtomicBool>,
    Instant,
);

static OCR_WORKER_SEQ: AtomicU64 = AtomicU64::new(0);
static OCR_WORKER_IN_FLIGHT: AtomicUsize = AtomicUsize::new(0);
static OCR_WORKER_TOTAL_STARTED: AtomicU64 = AtomicU64::new(0);
static OCR_WORKER_TOTAL_COMPLETED: AtomicU64 = AtomicU64::new(0);
static OCR_WORKER_TOTAL_TIMED_OUT: AtomicU64 = AtomicU64::new(0);
static OCR_WORKER_TOTAL_BACKPRESSURE: AtomicU64 = AtomicU64::new(0);
static OCR_WORKER_TOTAL_PANICKED: AtomicU64 = AtomicU64::new(0);
static OCR_WORKER_TOTAL_CHANNEL_CLOSED: AtomicU64 = AtomicU64::new(0);
static OCR_WORKER_TOTAL_CIRCUIT_SKIPPED: AtomicU64 = AtomicU64::new(0);
static OCR_WORKER_TOTAL_MEMORY_GUARD: AtomicU64 = AtomicU64::new(0);
static OCR_WORKER_TOTAL_CROSS_PROCESS_BUSY: AtomicU64 = AtomicU64::new(0);
static OCR_WORKER_TOTAL_CROSS_PROCESS_LOCK_ERRORS: AtomicU64 = AtomicU64::new(0);
static OCR_WORKER_TOTAL_CROSS_PROCESS_WAIT_ACQUIRED: AtomicU64 = AtomicU64::new(0);
static OCR_WORKER_TOTAL_CROSS_PROCESS_WAIT_TIMED_OUT: AtomicU64 = AtomicU64::new(0);
static OCR_FAILURE_CIRCUIT_OPEN_UNTIL_EPOCH_MS: AtomicU64 = AtomicU64::new(0);
static OCR_RUNTIME_PREWARMED: AtomicBool = AtomicBool::new(false);

pub(in crate::llm) fn mark_deepseek_ocr_runtime_prewarmed() {
    OCR_RUNTIME_PREWARMED.store(true, Ordering::Release);
}

pub(in crate::llm) fn build_ocr_truth_overlay_text(ocr_truth_markdown: &str) -> String {
    format!("{OCR_TRUTH_HEADER}\n\n{ocr_truth_markdown}\n\n{OCR_TRUTH_FOOTER}")
}

fn ocr_truth_preview(value: &str, max_chars: usize) -> String {
    value
        .chars()
        .take(max_chars)
        .collect::<String>()
        .replace('\n', "\\n")
}

fn ocr_truth_sha256(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    hex::encode(hasher.finalize())
}

#[derive(Debug, Clone, Copy)]
struct OcrWorkerTelemetrySnapshot {
    worker_id: u64,
    in_flight: usize,
    total_started: u64,
    total_completed: u64,
    total_timed_out: u64,
    total_backpressure: u64,
    total_panicked: u64,
    total_channel_closed: u64,
    total_circuit_skipped: u64,
    total_cross_process_busy: u64,
    total_cross_process_lock_errors: u64,
    total_cross_process_wait_acquired: u64,
    total_cross_process_wait_timed_out: u64,
    circuit_open_until_epoch_ms: u64,
}

fn snapshot_ocr_worker_telemetry(worker_id: u64) -> OcrWorkerTelemetrySnapshot {
    OcrWorkerTelemetrySnapshot {
        worker_id,
        in_flight: OCR_WORKER_IN_FLIGHT.load(Ordering::Relaxed),
        total_started: OCR_WORKER_TOTAL_STARTED.load(Ordering::Relaxed),
        total_completed: OCR_WORKER_TOTAL_COMPLETED.load(Ordering::Relaxed),
        total_timed_out: OCR_WORKER_TOTAL_TIMED_OUT.load(Ordering::Relaxed),
        total_backpressure: OCR_WORKER_TOTAL_BACKPRESSURE.load(Ordering::Relaxed),
        total_panicked: OCR_WORKER_TOTAL_PANICKED.load(Ordering::Relaxed),
        total_channel_closed: OCR_WORKER_TOTAL_CHANNEL_CLOSED.load(Ordering::Relaxed),
        total_circuit_skipped: OCR_WORKER_TOTAL_CIRCUIT_SKIPPED.load(Ordering::Relaxed),
        total_cross_process_busy: OCR_WORKER_TOTAL_CROSS_PROCESS_BUSY.load(Ordering::Relaxed),
        total_cross_process_lock_errors: OCR_WORKER_TOTAL_CROSS_PROCESS_LOCK_ERRORS
            .load(Ordering::Relaxed),
        total_cross_process_wait_acquired: OCR_WORKER_TOTAL_CROSS_PROCESS_WAIT_ACQUIRED
            .load(Ordering::Relaxed),
        total_cross_process_wait_timed_out: OCR_WORKER_TOTAL_CROSS_PROCESS_WAIT_TIMED_OUT
            .load(Ordering::Relaxed),
        circuit_open_until_epoch_ms: OCR_FAILURE_CIRCUIT_OPEN_UNTIL_EPOCH_MS
            .load(Ordering::Relaxed),
    }
}

enum OcrProcessLockAcquire {
    Acquired(OcrProcessGuard),
    Busy {
        path: PathBuf,
        waited_ms: u64,
        attempts: u32,
    },
    Error {
        path: PathBuf,
        message: String,
    },
}

enum OcrProcessGuard {
    Active { _lock: OcrProcessLock },
    Disabled,
}

struct OcrProcessLock {
    file: File,
    path: PathBuf,
}

impl Drop for OcrProcessLock {
    fn drop(&mut self) {
        if let Err(error) = self.file.unlock() {
            tracing::warn!(
                event = "agent.llm.vision.deepseek.ocr.global_lock.release_failed",
                path = %self.path.display(),
                error = %error,
                "DeepSeek OCR global process lock release failed"
            );
        }
    }
}

async fn execute_ocr_blocking_task_with_gate<T, F>(
    gate: Arc<Semaphore>,
    timeout: Duration,
    task: F,
) -> OcrBlockingExecution<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, anyhow::Error> + Send + 'static,
{
    let gate_permit = match acquire_ocr_gate_permit_or_skip(gate) {
        Ok(permit) => permit,
        Err(outcome) => return outcome,
    };
    let mut gate_permit = Some(gate_permit);
    let worker_id = OCR_WORKER_SEQ.fetch_add(1, Ordering::Relaxed) + 1;
    OCR_WORKER_TOTAL_STARTED.fetch_add(1, Ordering::Relaxed);
    OCR_WORKER_IN_FLIGHT.fetch_add(1, Ordering::Relaxed);

    let (rx, worker_done, started) = match spawn_ocr_worker_thread(worker_id, task) {
        Ok(values) => values,
        Err(outcome) => {
            OCR_WORKER_IN_FLIGHT.fetch_sub(1, Ordering::Relaxed);
            return outcome;
        }
    };

    log_ocr_worker_started(worker_id, timeout);
    let execution_outcome =
        wait_for_ocr_worker_outcome(timeout, rx, Arc::clone(&worker_done), worker_id, started)
            .await;
    if matches!(execution_outcome, OcrBlockingExecution::TimedOut)
        && let Some(permit) = gate_permit.take()
    {
        log_ocr_gate_release_deferred(worker_id);
        defer_gate_release_until_worker_finishes(worker_id, worker_done, permit);
    }
    execution_outcome
}

fn acquire_ocr_gate_permit_or_skip<T>(
    gate: Arc<Semaphore>,
) -> Result<tokio::sync::OwnedSemaphorePermit, OcrBlockingExecution<T>> {
    if let Some((rss_bytes, limit_bytes)) = deepseek_ocr_memory_guard_violation() {
        OCR_WORKER_TOTAL_MEMORY_GUARD.fetch_add(1, Ordering::Relaxed);
        tracing::warn!(
            event = "agent.llm.vision.deepseek.ocr.memory_guard",
            rss_gb = bytes_to_gib(rss_bytes),
            limit_gb = bytes_to_gib(limit_bytes),
            rss_bytes,
            limit_bytes,
            "DeepSeek OCR blocked by process RSS guard before starting worker"
        );
        return Err(OcrBlockingExecution::MemoryGuard {
            rss_bytes,
            limit_bytes,
        });
    }

    let max_in_flight = deepseek_ocr_max_in_flight();
    let before_in_flight = OCR_WORKER_IN_FLIGHT.load(Ordering::Relaxed);
    if before_in_flight >= max_in_flight {
        OCR_WORKER_TOTAL_BACKPRESSURE.fetch_add(1, Ordering::Relaxed);
        tracing::warn!(
            event = "agent.llm.vision.deepseek.ocr.backpressure",
            in_flight = before_in_flight,
            max_in_flight,
            "DeepSeek OCR skipped due to worker backpressure guard"
        );
        return Err(OcrBlockingExecution::BusyBackpressure);
    }

    gate.try_acquire_owned()
        .map_err(|_| OcrBlockingExecution::Busy)
}

fn spawn_ocr_worker_thread<T, F>(
    worker_id: u64,
    task: F,
) -> Result<OcrWorkerSpawn<T>, OcrBlockingExecution<T>>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, anyhow::Error> + Send + 'static,
{
    let (tx, rx) = oneshot::channel::<Result<T, anyhow::Error>>();
    let worker_done = Arc::new(AtomicBool::new(false));
    let worker_done_for_thread = Arc::clone(&worker_done);
    let started = Instant::now();
    let spawn_result = std::thread::Builder::new()
        .name(format!("deepseek-ocr-refine-{worker_id}"))
        .spawn(move || {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(task));
            worker_done_for_thread.store(true, Ordering::Relaxed);
            OCR_WORKER_IN_FLIGHT.fetch_sub(1, Ordering::Relaxed);
            match outcome {
                Ok(result) => {
                    OCR_WORKER_TOTAL_COMPLETED.fetch_add(1, Ordering::Relaxed);
                    let _ = tx.send(result);
                }
                Err(payload) => {
                    OCR_WORKER_TOTAL_PANICKED.fetch_add(1, Ordering::Relaxed);
                    let panic_message = panic_payload_to_string(&payload);
                    tracing::error!(
                        event = "agent.llm.vision.deepseek.ocr.worker_panicked",
                        worker_id,
                        panic = %panic_message,
                        "DeepSeek OCR worker panicked during blocking refinement"
                    );
                    let _ = tx.send(Err(anyhow::anyhow!(
                        "{OCR_WORKER_PANIC_PREFIX}{panic_message}"
                    )));
                }
            }
        });
    if let Err(error) = spawn_result {
        worker_done.store(true, Ordering::Relaxed);
        return Err(OcrBlockingExecution::RefineFailed(anyhow::anyhow!(
            "failed to spawn deepseek OCR worker thread: {error}"
        )));
    }
    Ok((rx, worker_done, started))
}

fn log_ocr_worker_started(worker_id: u64, timeout: Duration) {
    tracing::debug!(
        event = "agent.llm.vision.deepseek.ocr.worker_started",
        worker_id,
        timeout_ms = u64::try_from(timeout.as_millis()).unwrap_or(u64::MAX),
        heartbeat_ms =
            u64::try_from(deepseek_ocr_worker_heartbeat_interval().as_millis()).unwrap_or(u64::MAX),
        in_flight = OCR_WORKER_IN_FLIGHT.load(Ordering::Relaxed),
        "DeepSeek OCR worker started"
    );
}

async fn wait_for_ocr_worker_outcome<T>(
    timeout: Duration,
    rx: oneshot::Receiver<Result<T, anyhow::Error>>,
    worker_done: Arc<AtomicBool>,
    worker_id: u64,
    started: Instant,
) -> OcrBlockingExecution<T>
where
    T: Send + 'static,
{
    let heartbeat_interval = deepseek_ocr_worker_heartbeat_interval();
    let mut heartbeat = tokio::time::interval(heartbeat_interval);
    heartbeat.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let _ = heartbeat.tick().await;
    let mut timeout_future = Box::pin(tokio::time::timeout(timeout, rx));

    loop {
        tokio::select! {
            execution = &mut timeout_future => {
                break match execution {
                    Ok(Ok(Ok(value))) => OcrBlockingExecution::Completed(value),
                    Ok(Ok(Err(error))) => {
                        let message = error.to_string();
                        if let Some(panic_message) = message.strip_prefix(OCR_WORKER_PANIC_PREFIX) {
                            OcrBlockingExecution::Panicked(panic_message.to_string())
                        } else {
                            OcrBlockingExecution::RefineFailed(error)
                        }
                    }
                    Ok(Err(_)) => channel_closed_outcome(worker_id),
                    Err(_) => timeout_outcome(worker_id, started, Arc::clone(&worker_done)),
                };
            }
            _ = heartbeat.tick() => {
                log_ocr_worker_heartbeat(worker_id, started, heartbeat_interval, &worker_done);
            }
        }
    }
}

fn channel_closed_outcome<T>(worker_id: u64) -> OcrBlockingExecution<T> {
    OCR_WORKER_TOTAL_CHANNEL_CLOSED.fetch_add(1, Ordering::Relaxed);
    let telemetry = snapshot_ocr_worker_telemetry(worker_id);
    tracing::warn!(
        event = "agent.llm.vision.deepseek.ocr.channel_closed",
        worker_id = telemetry.worker_id,
        in_flight = telemetry.in_flight,
        total_started = telemetry.total_started,
        total_completed = telemetry.total_completed,
        total_timed_out = telemetry.total_timed_out,
        total_backpressure = telemetry.total_backpressure,
        total_panicked = telemetry.total_panicked,
        total_channel_closed = telemetry.total_channel_closed,
        total_circuit_skipped = telemetry.total_circuit_skipped,
        circuit_open_until_epoch_ms = telemetry.circuit_open_until_epoch_ms,
        "DeepSeek OCR worker channel closed before delivering result"
    );
    OcrBlockingExecution::ChannelClosed
}

fn timeout_outcome<T>(
    worker_id: u64,
    started: Instant,
    worker_done: Arc<AtomicBool>,
) -> OcrBlockingExecution<T> {
    OCR_WORKER_TOTAL_TIMED_OUT.fetch_add(1, Ordering::Relaxed);
    let telemetry = snapshot_ocr_worker_telemetry(worker_id);
    tracing::warn!(
        event = "agent.llm.vision.deepseek.ocr.worker_timeout",
        worker_id = telemetry.worker_id,
        worker_elapsed_ms = started.elapsed().as_millis(),
        in_flight = telemetry.in_flight,
        total_started = telemetry.total_started,
        total_completed = telemetry.total_completed,
        total_timed_out = telemetry.total_timed_out,
        total_backpressure = telemetry.total_backpressure,
        total_panicked = telemetry.total_panicked,
        total_channel_closed = telemetry.total_channel_closed,
        total_circuit_skipped = telemetry.total_circuit_skipped,
        circuit_open_until_epoch_ms = telemetry.circuit_open_until_epoch_ms,
        "DeepSeek OCR worker timed out while background execution may still be running"
    );
    spawn_ocr_stuck_watchdog(worker_id, worker_done);
    OcrBlockingExecution::TimedOut
}

fn spawn_ocr_stuck_watchdog(worker_id: u64, worker_done: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        std::thread::sleep(deepseek_ocr_stuck_watchdog_delay());
        if !worker_done.load(Ordering::Relaxed) {
            let telemetry = snapshot_ocr_worker_telemetry(worker_id);
            tracing::error!(
                event = "agent.llm.vision.deepseek.ocr.worker_stuck_suspect",
                worker_id = telemetry.worker_id,
                in_flight = telemetry.in_flight,
                total_started = telemetry.total_started,
                total_completed = telemetry.total_completed,
                total_timed_out = telemetry.total_timed_out,
                total_backpressure = telemetry.total_backpressure,
                total_panicked = telemetry.total_panicked,
                total_channel_closed = telemetry.total_channel_closed,
                total_circuit_skipped = telemetry.total_circuit_skipped,
                circuit_open_until_epoch_ms = telemetry.circuit_open_until_epoch_ms,
                "DeepSeek OCR worker still not finished after watchdog delay"
            );
        }
    });
}

fn log_ocr_worker_heartbeat(
    worker_id: u64,
    started: Instant,
    heartbeat_interval: Duration,
    worker_done: &Arc<AtomicBool>,
) {
    if worker_done.load(Ordering::Relaxed) {
        return;
    }
    let telemetry = snapshot_ocr_worker_telemetry(worker_id);
    tracing::info!(
        event = "agent.llm.vision.deepseek.ocr.worker_heartbeat",
        worker_id = telemetry.worker_id,
        worker_elapsed_ms = started.elapsed().as_millis(),
        heartbeat_ms = u64::try_from(heartbeat_interval.as_millis()).unwrap_or(u64::MAX),
        in_flight = telemetry.in_flight,
        total_started = telemetry.total_started,
        total_completed = telemetry.total_completed,
        total_timed_out = telemetry.total_timed_out,
        total_backpressure = telemetry.total_backpressure,
        total_panicked = telemetry.total_panicked,
        total_channel_closed = telemetry.total_channel_closed,
        total_circuit_skipped = telemetry.total_circuit_skipped,
        "DeepSeek OCR worker still running"
    );
}

fn log_ocr_gate_release_deferred(worker_id: u64) {
    let telemetry = snapshot_ocr_worker_telemetry(worker_id);
    tracing::warn!(
        event = "agent.llm.vision.deepseek.ocr.gate_release_deferred",
        worker_id = telemetry.worker_id,
        in_flight = telemetry.in_flight,
        total_started = telemetry.total_started,
        total_completed = telemetry.total_completed,
        total_timed_out = telemetry.total_timed_out,
        total_backpressure = telemetry.total_backpressure,
        total_panicked = telemetry.total_panicked,
        total_channel_closed = telemetry.total_channel_closed,
        total_circuit_skipped = telemetry.total_circuit_skipped,
        circuit_open_until_epoch_ms = telemetry.circuit_open_until_epoch_ms,
        "DeepSeek OCR gate release deferred until timed-out worker actually finishes"
    );
}

async fn execute_ocr_blocking_task<T, F>(timeout: Duration, task: F) -> OcrBlockingExecution<T>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, anyhow::Error> + Send + 'static,
{
    execute_ocr_blocking_task_with_gate(deepseek_ocr_gate(), timeout, task).await
}

pub(in crate::llm) async fn simulate_ocr_gate_timeout_recovery_for_tests(
    blocking_sleep_ms: u64,
    timeout_ms: u64,
) -> OcrGateTimeoutRecoveryProbe {
    let gate = Arc::new(Semaphore::new(1));
    let timeout = Duration::from_millis(timeout_ms.max(1));

    let first_result = execute_ocr_blocking_task_with_gate(Arc::clone(&gate), timeout, move || {
        std::thread::sleep(Duration::from_millis(blocking_sleep_ms));
        Ok(())
    })
    .await;

    let second_result = execute_ocr_blocking_task_with_gate(Arc::clone(&gate), timeout, || {
        Ok::<(), anyhow::Error>(())
    })
    .await;

    let recovery_wait_ms = blocking_sleep_ms
        .max(timeout_ms)
        .saturating_add(25)
        .min(2_000);
    tokio::time::sleep(Duration::from_millis(recovery_wait_ms)).await;
    let third_result =
        execute_ocr_blocking_task_with_gate(gate, timeout, || Ok::<(), anyhow::Error>(())).await;

    let first_outcome = match first_result {
        OcrBlockingExecution::TimedOut => OcrProbeFirstOutcome::TimedOut,
        OcrBlockingExecution::Panicked(_) => OcrProbeFirstOutcome::Panicked,
        _ => OcrProbeFirstOutcome::Other,
    };

    OcrGateTimeoutRecoveryProbe {
        first_outcome,
        second_was_busy: matches!(
            second_result,
            OcrBlockingExecution::Busy | OcrBlockingExecution::BusyBackpressure
        ),
        second_completed: matches!(second_result, OcrBlockingExecution::Completed(())),
        recovered_after_wait: matches!(third_result, OcrBlockingExecution::Completed(())),
    }
}

pub(in crate::llm) async fn simulate_ocr_gate_panic_recovery_for_tests()
-> OcrGateTimeoutRecoveryProbe {
    let gate = Arc::new(Semaphore::new(1));
    let timeout = Duration::from_secs(1);

    let first_result = execute_ocr_blocking_task_with_gate(
        Arc::clone(&gate),
        timeout,
        panic_for_ocr_recovery_probe,
    )
    .await;

    let second_result =
        execute_ocr_blocking_task_with_gate(gate, timeout, || Ok::<(), anyhow::Error>(())).await;

    let first_outcome = match first_result {
        OcrBlockingExecution::TimedOut => OcrProbeFirstOutcome::TimedOut,
        OcrBlockingExecution::Panicked(_) => OcrProbeFirstOutcome::Panicked,
        _ => OcrProbeFirstOutcome::Other,
    };

    OcrGateTimeoutRecoveryProbe {
        first_outcome,
        second_was_busy: matches!(
            second_result,
            OcrBlockingExecution::Busy | OcrBlockingExecution::BusyBackpressure
        ),
        second_completed: matches!(second_result, OcrBlockingExecution::Completed(())),
        recovered_after_wait: matches!(second_result, OcrBlockingExecution::Completed(())),
    }
}

pub(in crate::llm) async fn infer_deepseek_ocr_truth_markdown(
    source: &Base64ImageSource,
) -> Option<String> {
    let request_started = Instant::now();
    let runtime = get_deepseek_runtime();
    if !log_ocr_runtime_status(runtime.as_ref()) {
        return None;
    }
    let _process_guard = acquire_global_ocr_process_guard_or_log().await?;
    if !allow_ocr_request_under_failure_circuit() {
        return None;
    }
    let max_dimension = deepseek_ocr_max_dimension();
    let decode_started = Instant::now();
    let image_bytes = decode_source_image_bytes(source)?;
    let image_decode_ms = decode_started.elapsed().as_millis();
    let preprocess_started = Instant::now();
    let prepared = preprocess_source_image_payload(image_bytes, max_dimension)?;
    let image_preprocess_ms = preprocess_started.elapsed().as_millis();
    if !OCR_RUNTIME_PREWARMED.load(Ordering::Acquire) {
        tracing::info!(
            event = "agent.llm.vision.deepseek.ocr.prewarm_deferred",
            "DeepSeek OCR request path is running without blocking prewarm"
        );
    }

    let timeout_config = deepseek_ocr_timeout();
    let refine_started = Instant::now();
    let markdown = run_refinement_task(
        Arc::clone(&runtime),
        prepared,
        timeout_config,
        request_started,
        max_dimension,
    )
    .await?;
    let refine_ms = refine_started.elapsed().as_millis();
    OCR_RUNTIME_PREWARMED.store(true, Ordering::Release);

    let markdown = normalize_ocr_truth_markdown(markdown);
    log_ocr_truth_outcome(
        markdown.as_deref(),
        image_decode_ms,
        image_preprocess_ms,
        refine_ms,
        request_started.elapsed().as_millis(),
    );
    markdown
}

async fn acquire_global_ocr_process_guard_or_log() -> Option<OcrProcessGuard> {
    match acquire_global_ocr_process_guard_with_wait().await {
        OcrProcessLockAcquire::Acquired(guard) => Some(guard),
        OcrProcessLockAcquire::Busy {
            path,
            waited_ms,
            attempts,
        } => {
            OCR_WORKER_TOTAL_CROSS_PROCESS_BUSY.fetch_add(1, Ordering::Relaxed);
            let telemetry = snapshot_ocr_worker_telemetry(0);
            tracing::warn!(
                event = "agent.llm.vision.deepseek.ocr.cross_process_busy",
                path = %path.display(),
                waited_ms,
                attempts,
                in_flight = telemetry.in_flight,
                total_started = telemetry.total_started,
                total_completed = telemetry.total_completed,
                total_timed_out = telemetry.total_timed_out,
                total_backpressure = telemetry.total_backpressure,
                total_panicked = telemetry.total_panicked,
                total_channel_closed = telemetry.total_channel_closed,
                total_circuit_skipped = telemetry.total_circuit_skipped,
                total_cross_process_busy = telemetry.total_cross_process_busy,
                total_cross_process_lock_errors = telemetry.total_cross_process_lock_errors,
                total_cross_process_wait_acquired = telemetry.total_cross_process_wait_acquired,
                total_cross_process_wait_timed_out = telemetry.total_cross_process_wait_timed_out,
                "DeepSeek OCR skipped after waiting for global OCR lock"
            );
            None
        }
        OcrProcessLockAcquire::Error { path, message } => {
            OCR_WORKER_TOTAL_CROSS_PROCESS_LOCK_ERRORS.fetch_add(1, Ordering::Relaxed);
            let telemetry = snapshot_ocr_worker_telemetry(0);
            tracing::warn!(
                event = "agent.llm.vision.deepseek.ocr.cross_process_lock_error",
                path = %path.display(),
                error = %message,
                in_flight = telemetry.in_flight,
                total_started = telemetry.total_started,
                total_completed = telemetry.total_completed,
                total_timed_out = telemetry.total_timed_out,
                total_backpressure = telemetry.total_backpressure,
                total_panicked = telemetry.total_panicked,
                total_channel_closed = telemetry.total_channel_closed,
                total_circuit_skipped = telemetry.total_circuit_skipped,
                total_cross_process_busy = telemetry.total_cross_process_busy,
                total_cross_process_lock_errors = telemetry.total_cross_process_lock_errors,
                total_cross_process_wait_acquired = telemetry.total_cross_process_wait_acquired,
                total_cross_process_wait_timed_out = telemetry.total_cross_process_wait_timed_out,
                "DeepSeek OCR skipped because global lock setup failed"
            );
            None
        }
    }
}

fn allow_ocr_request_under_failure_circuit() -> bool {
    if let Some(remaining_ms) = deepseek_ocr_failure_circuit_remaining_ms() {
        OCR_WORKER_TOTAL_CIRCUIT_SKIPPED.fetch_add(1, Ordering::Relaxed);
        let telemetry = snapshot_ocr_worker_telemetry(0);
        tracing::warn!(
            event = "agent.llm.vision.deepseek.ocr.circuit_open",
            remaining_ms,
            in_flight = telemetry.in_flight,
            total_started = telemetry.total_started,
            total_completed = telemetry.total_completed,
            total_timed_out = telemetry.total_timed_out,
            total_backpressure = telemetry.total_backpressure,
            total_panicked = telemetry.total_panicked,
            total_channel_closed = telemetry.total_channel_closed,
            total_circuit_skipped = telemetry.total_circuit_skipped,
            circuit_open_until_epoch_ms = telemetry.circuit_open_until_epoch_ms,
            "DeepSeek OCR skipped because failure circuit is open"
        );
        return false;
    }
    true
}

fn normalize_ocr_truth_markdown(markdown: Option<String>) -> Option<String> {
    markdown
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn log_ocr_truth_outcome(
    markdown: Option<&str>,
    image_decode_ms: u128,
    image_preprocess_ms: u128,
    refine_ms: u128,
    elapsed_ms: u128,
) {
    if let Some(ocr_truth) = markdown {
        let preview = ocr_truth_preview(ocr_truth, 120);
        let sha256 = ocr_truth_sha256(ocr_truth);
        tracing::info!(
            event = "agent.llm.vision.deepseek.ocr.truth_extracted",
            chars = ocr_truth.chars().count(),
            sha256 = %sha256,
            preview = %preview,
            image_decode_ms,
            image_preprocess_ms,
            refine_ms,
            elapsed_ms,
            "DeepSeek OCR truth extracted and injected into anthropic multimodal payload"
        );
        return;
    }
    tracing::info!(
        event = "agent.llm.vision.deepseek.ocr.empty",
        image_decode_ms,
        image_preprocess_ms,
        refine_ms,
        elapsed_ms,
        "DeepSeek OCR completed but produced empty truth markdown"
    );
}

fn log_ocr_runtime_status(runtime: &DeepseekRuntime) -> bool {
    match runtime {
        DeepseekRuntime::Disabled { reason } => {
            tracing::warn!(
                event = "agent.llm.vision.deepseek.ocr.skipped",
                reason = %reason,
                "DeepSeek OCR skipped because runtime is disabled"
            );
            false
        }
        DeepseekRuntime::Configured { model_root } => {
            tracing::info!(
                event = "agent.llm.vision.deepseek.ocr.runtime_ready",
                model_root = %model_root,
                "DeepSeek OCR runtime is configured for multimodal image"
            );
            true
        }
    }
}

fn decode_source_image_bytes(source: &Base64ImageSource) -> Option<Arc<[u8]>> {
    let image_bytes = match base64::engine::general_purpose::STANDARD.decode(source.data.as_bytes())
    {
        Ok(bytes) => bytes,
        Err(error) => {
            tracing::warn!(
                event = "agent.llm.vision.deepseek.ocr.decode_failed",
                error = %error,
                media_type = %source.media_type,
                "DeepSeek OCR skipped because image base64 decode failed"
            );
            return None;
        }
    };
    tracing::debug!(
        event = "agent.llm.vision.deepseek.ocr.image_decoded",
        media_type = %source.media_type,
        bytes = image_bytes.len(),
        "DeepSeek OCR source image decoded from base64 payload"
    );
    Some(Arc::from(image_bytes.into_boxed_slice()))
}

fn preprocess_source_image_payload(
    image_bytes: Arc<[u8]>,
    max_dimension: u32,
) -> Option<PreparedVisionImage> {
    match preprocess_image_for_ocr(image_bytes, max_dimension) {
        Ok(prepared) => {
            tracing::debug!(
                event = "agent.llm.vision.deepseek.ocr.image_preprocessed",
                width = prepared.width,
                height = prepared.height,
                scale = prepared.scale,
                "DeepSeek OCR source image preprocessed before inference"
            );
            Some(prepared)
        }
        Err(error) => {
            tracing::warn!(
                event = "agent.llm.vision.deepseek.ocr.image_preprocess_failed",
                error = %error,
                "DeepSeek OCR skipped because source image preprocessing failed"
            );
            None
        }
    }
}

async fn run_refinement_task(
    runtime: Arc<DeepseekRuntime>,
    prepared: PreparedVisionImage,
    timeout_config: OcrTimeoutConfig,
    request_started: Instant,
    max_dimension: u32,
) -> Option<Option<String>> {
    let prepared_width = prepared.width;
    let prepared_height = prepared.height;
    match execute_ocr_blocking_task(timeout_config.duration, move || {
        Ok(infer_deepseek_ocr_truth(runtime.as_ref(), &prepared)?)
    })
    .await
    {
        OcrBlockingExecution::Busy => {
            tracing::warn!(
                event = "agent.llm.vision.deepseek.ocr.busy",
                "DeepSeek OCR skipped because another OCR task is still running"
            );
            None
        }
        OcrBlockingExecution::BusyBackpressure => {
            tracing::warn!(
                event = "agent.llm.vision.deepseek.ocr.busy_backpressure",
                in_flight = OCR_WORKER_IN_FLIGHT.load(Ordering::Relaxed),
                max_in_flight = deepseek_ocr_max_in_flight(),
                "DeepSeek OCR skipped due to worker backpressure guard"
            );
            None
        }
        OcrBlockingExecution::MemoryGuard {
            rss_bytes,
            limit_bytes,
        } => {
            open_deepseek_ocr_failure_circuit("memory_guard");
            tracing::warn!(
                event = "agent.llm.vision.deepseek.ocr.memory_guard",
                rss_gb = bytes_to_gib(rss_bytes),
                limit_gb = bytes_to_gib(limit_bytes),
                rss_bytes,
                limit_bytes,
                elapsed_ms = request_started.elapsed().as_millis(),
                "DeepSeek OCR skipped because process RSS is above guard threshold"
            );
            None
        }
        OcrBlockingExecution::Completed(markdown) => Some(markdown),
        OcrBlockingExecution::RefineFailed(error) => {
            tracing::warn!(
                event = "agent.llm.vision.deepseek.ocr.refine_failed",
                error = %error,
                "DeepSeek OCR inference failed; continue without OCR truth overlay"
            );
            None
        }
        OcrBlockingExecution::Panicked(panic_message) => {
            open_deepseek_ocr_failure_circuit("panic");
            tracing::error!(
                event = "agent.llm.vision.deepseek.ocr.panic",
                panic = %panic_message,
                elapsed_ms = request_started.elapsed().as_millis(),
                "DeepSeek OCR worker panicked; continue without OCR truth overlay"
            );
            None
        }
        OcrBlockingExecution::ChannelClosed => {
            open_deepseek_ocr_failure_circuit("channel_closed");
            tracing::warn!(
                event = "agent.llm.vision.deepseek.ocr.channel_closed",
                "DeepSeek OCR worker channel closed before delivering result; continue without OCR truth overlay"
            );
            None
        }
        OcrBlockingExecution::TimedOut => {
            open_deepseek_ocr_failure_circuit("timeout");
            let timeout_ms = u64::try_from(timeout_config.duration.as_millis()).unwrap_or(u64::MAX);
            tracing::warn!(
                event = "agent.llm.vision.deepseek.ocr.timeout",
                timeout_ms,
                timeout_stage = timeout_config.stage,
                elapsed_ms = request_started.elapsed().as_millis(),
                max_dimension,
                width = prepared_width,
                height = prepared_height,
                "DeepSeek OCR timed out; continue without OCR truth overlay"
            );
            None
        }
    }
}

fn deepseek_ocr_timeout() -> OcrTimeoutConfig {
    const DEFAULT_WARM_TIMEOUT_MS: u64 = 120_000;
    const DEFAULT_COLD_TIMEOUT_MS: u64 = 120_000;
    static COLD_START_PENDING: OnceLock<std::sync::atomic::AtomicBool> = OnceLock::new();
    let cold_pending = COLD_START_PENDING
        .get_or_init(|| std::sync::atomic::AtomicBool::new(true))
        .swap(false, std::sync::atomic::Ordering::SeqCst);

    let warm_timeout_ms = std::env::var("XIUXIAN_VISION_OCR_TIMEOUT_MS")
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_WARM_TIMEOUT_MS);
    let cold_timeout_ms = std::env::var("XIUXIAN_VISION_OCR_COLD_TIMEOUT_MS")
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_COLD_TIMEOUT_MS)
        .max(warm_timeout_ms);

    if cold_pending {
        OcrTimeoutConfig {
            duration: Duration::from_millis(cold_timeout_ms),
            stage: "cold_start",
        }
    } else {
        OcrTimeoutConfig {
            duration: Duration::from_millis(warm_timeout_ms),
            stage: "steady_state",
        }
    }
}

fn deepseek_ocr_memory_guard_violation() -> Option<(u64, u64)> {
    let limit_bytes = deepseek_ocr_max_process_rss_bytes()?;
    let current_rss_bytes = current_process_rss_bytes()?;
    (current_rss_bytes > limit_bytes).then_some((current_rss_bytes, limit_bytes))
}

fn deepseek_ocr_max_process_rss_bytes() -> Option<u64> {
    parse_process_rss_limit_bytes(std::env::var("XIUXIAN_VISION_OCR_MAX_PROCESS_RSS_GB").ok())
}

fn parse_process_rss_limit_bytes(raw_value: Option<String>) -> Option<u64> {
    let raw = raw_value?;
    let parsed = raw.trim();
    if parsed.is_empty() || parsed.starts_with('-') {
        return None;
    }

    let (int_part_raw, frac_part_raw) = parsed
        .split_once('.')
        .map_or((parsed, ""), |(integer, fraction)| (integer, fraction));

    if !int_part_raw.chars().all(|ch| ch.is_ascii_digit())
        || !frac_part_raw.chars().all(|ch| ch.is_ascii_digit())
    {
        return None;
    }

    let int_part = if int_part_raw.is_empty() {
        0_u128
    } else {
        int_part_raw.parse::<u128>().ok()?
    };

    let mut frac_digits = frac_part_raw.chars().take(6).collect::<String>();
    while frac_digits.len() < 6 {
        frac_digits.push('0');
    }
    let frac_part = if frac_digits.is_empty() {
        0_u128
    } else {
        frac_digits.parse::<u128>().ok()?
    };

    let scaled_gib = int_part
        .checked_mul(OCR_RSS_GIB_SCALE)?
        .checked_add(frac_part)?;
    if scaled_gib == 0 {
        return None;
    }

    let bytes_u128 = scaled_gib
        .checked_mul(OCR_BYTES_PER_GIB_U128)?
        .checked_div(OCR_RSS_GIB_SCALE)?;
    let clamped = bytes_u128.min(u128::from(u64::MAX));
    u64::try_from(clamped).ok()
}

fn current_process_rss_bytes() -> Option<u64> {
    let pid = std::process::id();
    let pid_arg = pid.to_string();
    let output = Command::new("ps")
        .args(["-o", "rss=", "-p", pid_arg.as_str()])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let rss_kb = stdout.trim().parse::<u64>().ok()?;
    rss_kb.checked_mul(1024)
}

fn bytes_to_gib(bytes: u64) -> String {
    let whole = bytes / OCR_BYTES_PER_GIB_U64;
    let fraction = ((bytes % OCR_BYTES_PER_GIB_U64) * 100) / OCR_BYTES_PER_GIB_U64;
    format!("{whole}.{fraction:02}")
}

fn deepseek_ocr_max_dimension() -> u32 {
    std::env::var("XIUXIAN_VISION_OCR_MAX_DIMENSION")
        .ok()
        .and_then(|raw| raw.trim().parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_OCR_MAX_DIMENSION)
}

fn deepseek_ocr_max_in_flight() -> usize {
    std::env::var("XIUXIAN_VISION_OCR_MAX_IN_FLIGHT")
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(1)
}

fn deepseek_ocr_stuck_watchdog_delay() -> Duration {
    let millis = std::env::var("XIUXIAN_VISION_OCR_STUCK_WATCHDOG_MS")
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(30_000);
    Duration::from_millis(millis)
}

fn deepseek_ocr_worker_heartbeat_interval() -> Duration {
    let millis = std::env::var("XIUXIAN_VISION_OCR_HEARTBEAT_MS")
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(5_000);
    Duration::from_millis(millis)
}

fn deepseek_ocr_failure_cooldown() -> Duration {
    let millis = std::env::var("XIUXIAN_VISION_OCR_FAILURE_COOLDOWN_MS")
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(15_000);
    Duration::from_millis(millis)
}

fn deepseek_ocr_global_lock_enabled() -> bool {
    std::env::var("XIUXIAN_VISION_OCR_GLOBAL_LOCK")
        .ok()
        .map(|raw| raw.trim().to_ascii_lowercase())
        .is_none_or(|raw| !matches!(raw.as_str(), "0" | "false" | "no" | "off"))
}

fn deepseek_ocr_global_lock_path() -> PathBuf {
    let project_root = resolve_project_root_or_cwd();
    if let Some(value) = std::env::var("XIUXIAN_VISION_OCR_LOCK_PATH")
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
    {
        let custom = PathBuf::from(value);
        return absolutize_path(project_root.as_path(), custom.as_path());
    }

    let runtime_dir = std::env::var("PRJ_RUNTIME_DIR")
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty())
        .map(PathBuf::from)
        .map_or_else(
            || project_root.join(".run"),
            |candidate| absolutize_path(project_root.as_path(), candidate.as_path()),
        );
    runtime_dir.join("locks").join("deepseek-ocr.lock")
}

async fn acquire_global_ocr_process_guard_with_wait() -> OcrProcessLockAcquire {
    acquire_global_ocr_process_guard_with_policy(
        deepseek_ocr_global_lock_wait_timeout(),
        deepseek_ocr_global_lock_poll_interval(),
    )
    .await
}

async fn acquire_global_ocr_process_guard_with_policy(
    wait_timeout: Duration,
    poll_interval: Duration,
) -> OcrProcessLockAcquire {
    if wait_timeout.is_zero() {
        return try_acquire_global_ocr_process_guard();
    }

    let wait_timeout_ms = u64::try_from(wait_timeout.as_millis()).unwrap_or(u64::MAX);
    let mut attempts: u32 = 0;
    let wait_started = Instant::now();

    loop {
        attempts = attempts.saturating_add(1);
        match try_acquire_global_ocr_process_guard() {
            OcrProcessLockAcquire::Acquired(guard) => {
                let waited_ms =
                    u64::try_from(wait_started.elapsed().as_millis()).unwrap_or(u64::MAX);
                if attempts > 1 {
                    OCR_WORKER_TOTAL_CROSS_PROCESS_WAIT_ACQUIRED.fetch_add(1, Ordering::Relaxed);
                    tracing::info!(
                        event = "agent.llm.vision.deepseek.ocr.cross_process_wait_acquired",
                        waited_ms,
                        attempts,
                        wait_timeout_ms,
                        "DeepSeek OCR acquired global lock after waiting"
                    );
                }
                return OcrProcessLockAcquire::Acquired(guard);
            }
            OcrProcessLockAcquire::Busy { path, .. } => {
                let waited_ms =
                    u64::try_from(wait_started.elapsed().as_millis()).unwrap_or(u64::MAX);
                if waited_ms >= wait_timeout_ms {
                    OCR_WORKER_TOTAL_CROSS_PROCESS_WAIT_TIMED_OUT.fetch_add(1, Ordering::Relaxed);
                    return OcrProcessLockAcquire::Busy {
                        path,
                        waited_ms,
                        attempts,
                    };
                }

                if attempts == 1 || attempts.is_multiple_of(20) {
                    tracing::info!(
                        event = "agent.llm.vision.deepseek.ocr.cross_process_waiting",
                        path = %path.display(),
                        waited_ms,
                        attempts,
                        wait_timeout_ms,
                        poll_ms = u64::try_from(poll_interval.as_millis()).unwrap_or(u64::MAX),
                        "DeepSeek OCR waiting for global process lock"
                    );
                }
                tokio::time::sleep(poll_interval).await;
            }
            OcrProcessLockAcquire::Error { path, message } => {
                return OcrProcessLockAcquire::Error { path, message };
            }
        }
    }
}

fn try_acquire_global_ocr_process_guard() -> OcrProcessLockAcquire {
    if !deepseek_ocr_global_lock_enabled() {
        return OcrProcessLockAcquire::Acquired(OcrProcessGuard::Disabled);
    }

    let lock_path = deepseek_ocr_global_lock_path();
    if let Some(parent) = lock_path.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        return OcrProcessLockAcquire::Error {
            path: lock_path,
            message: format!("failed to create lock dir: {error}"),
        };
    }

    let mut file = match OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(lock_path.as_path())
    {
        Ok(file) => file,
        Err(error) => {
            return OcrProcessLockAcquire::Error {
                path: lock_path,
                message: format!("failed to open lock file: {error}"),
            };
        }
    };

    match file.try_lock_exclusive() {
        Ok(()) => {
            let _ = file.set_len(0);
            let _ = writeln!(
                file,
                "pid={} started_epoch_ms={}",
                std::process::id(),
                now_epoch_millis()
            );
            OcrProcessLockAcquire::Acquired(OcrProcessGuard::Active {
                _lock: OcrProcessLock {
                    file,
                    path: lock_path,
                },
            })
        }
        Err(error) if error.kind() == ErrorKind::WouldBlock => OcrProcessLockAcquire::Busy {
            path: lock_path,
            waited_ms: 0,
            attempts: 1,
        },
        Err(error) => OcrProcessLockAcquire::Error {
            path: lock_path,
            message: format!("failed to acquire exclusive lock: {error}"),
        },
    }
}

fn deepseek_ocr_global_lock_wait_timeout() -> Duration {
    let millis = std::env::var("XIUXIAN_VISION_OCR_GLOBAL_LOCK_WAIT_MS")
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(120_000);
    Duration::from_millis(millis)
}

fn deepseek_ocr_global_lock_poll_interval() -> Duration {
    let millis = std::env::var("XIUXIAN_VISION_OCR_GLOBAL_LOCK_POLL_MS")
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(250);
    Duration::from_millis(millis)
}

fn deepseek_ocr_failure_circuit_remaining_ms() -> Option<u64> {
    let open_until = OCR_FAILURE_CIRCUIT_OPEN_UNTIL_EPOCH_MS.load(Ordering::Relaxed);
    let now = now_epoch_millis();
    if open_until > now {
        Some(open_until.saturating_sub(now))
    } else {
        None
    }
}

fn open_deepseek_ocr_failure_circuit(reason: &str) {
    let cooldown_ms =
        u64::try_from(deepseek_ocr_failure_cooldown().as_millis()).unwrap_or(u64::MAX);
    if cooldown_ms == 0 {
        return;
    }
    let now = now_epoch_millis();
    let open_until = now.saturating_add(cooldown_ms);
    let previous = OCR_FAILURE_CIRCUIT_OPEN_UNTIL_EPOCH_MS.fetch_max(open_until, Ordering::Relaxed);
    let effective_open_until = previous.max(open_until);
    let telemetry = snapshot_ocr_worker_telemetry(0);
    tracing::warn!(
        event = "agent.llm.vision.deepseek.ocr.circuit_opened",
        reason,
        cooldown_ms,
        open_until_epoch_ms = effective_open_until,
        in_flight = telemetry.in_flight,
        total_started = telemetry.total_started,
        total_completed = telemetry.total_completed,
        total_timed_out = telemetry.total_timed_out,
        total_backpressure = telemetry.total_backpressure,
        total_panicked = telemetry.total_panicked,
        total_channel_closed = telemetry.total_channel_closed,
        total_circuit_skipped = telemetry.total_circuit_skipped,
        "DeepSeek OCR failure circuit opened"
    );
}

fn now_epoch_millis() -> u64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(value) => u64::try_from(value.as_millis()).unwrap_or(u64::MAX),
        Err(_) => 0,
    }
}

pub(in crate::llm) async fn infer_deepseek_ocr_truth_from_bytes_for_tests(
    image_bytes: Arc<[u8]>,
    media_type: &str,
) -> Option<String> {
    let source = Base64ImageSource {
        media_type: media_type.to_string(),
        data: base64::engine::general_purpose::STANDARD.encode(image_bytes.as_ref()),
    };
    infer_deepseek_ocr_truth_markdown(&source).await
}

pub(in crate::llm) fn resolve_deepseek_ocr_global_lock_path_for_tests() -> String {
    deepseek_ocr_global_lock_path().display().to_string()
}

pub(in crate::llm) fn resolve_deepseek_ocr_memory_limit_bytes_for_tests(
    raw_limit_gb: Option<&str>,
) -> Option<u64> {
    parse_process_rss_limit_bytes(raw_limit_gb.map(ToString::to_string))
}

pub(in crate::llm) fn deepseek_ocr_memory_guard_triggered_for_tests(
    raw_limit_gb: Option<&str>,
    rss_bytes: u64,
) -> bool {
    resolve_deepseek_ocr_memory_limit_bytes_for_tests(raw_limit_gb)
        .is_some_and(|limit_bytes| rss_bytes > limit_bytes)
}

fn deepseek_ocr_gate() -> Arc<Semaphore> {
    static OCR_GATE: OnceLock<Arc<Semaphore>> = OnceLock::new();
    Arc::clone(OCR_GATE.get_or_init(|| Arc::new(Semaphore::new(1))))
}

fn panic_payload_to_string(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(value) = payload.downcast_ref::<&str>() {
        (*value).to_string()
    } else if let Some(value) = payload.downcast_ref::<String>() {
        value.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

fn panic_for_ocr_recovery_probe() -> Result<(), anyhow::Error> {
    panic!("simulated deepseek ocr worker panic for recovery test");
}

fn defer_gate_release_until_worker_finishes(
    worker_id: u64,
    worker_done: Arc<AtomicBool>,
    gate_permit: tokio::sync::OwnedSemaphorePermit,
) {
    let spawn_result = std::thread::Builder::new()
        .name(format!("deepseek-ocr-gate-release-{worker_id}"))
        .spawn(move || {
            let deferred_at = Instant::now();
            while !worker_done.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(10));
            }
            drop(gate_permit);
            let telemetry = snapshot_ocr_worker_telemetry(worker_id);
            tracing::info!(
                event = "agent.llm.vision.deepseek.ocr.gate_released_after_timeout",
                worker_id = telemetry.worker_id,
                gate_hold_ms = deferred_at.elapsed().as_millis(),
                in_flight = telemetry.in_flight,
                total_started = telemetry.total_started,
                total_completed = telemetry.total_completed,
                total_timed_out = telemetry.total_timed_out,
                total_backpressure = telemetry.total_backpressure,
                total_panicked = telemetry.total_panicked,
                total_channel_closed = telemetry.total_channel_closed,
                total_circuit_skipped = telemetry.total_circuit_skipped,
                circuit_open_until_epoch_ms = telemetry.circuit_open_until_epoch_ms,
                "DeepSeek OCR gate permit released after timed-out worker finished"
            );
        });
    if let Err(error) = spawn_result {
        tracing::error!(
            event = "agent.llm.vision.deepseek.ocr.gate_release_spawn_failed",
            worker_id,
            error = %error,
            "Failed to spawn deferred OCR gate release watcher thread"
        );
    }
}
