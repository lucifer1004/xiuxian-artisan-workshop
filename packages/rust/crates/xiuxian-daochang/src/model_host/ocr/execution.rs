use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::Result;
use tokio::sync::{Semaphore, oneshot};
use tokio::time::MissedTickBehavior;
use xiuxian_llm::llm::vision::DeepseekRuntime;

use super::core::{OcrTimeoutConfig, deepseek_ocr_gate, resolve_deepseek_ocr_gate_timeout};
use super::telemetry::spawn_deepseek_ocr_stuck_watchdog;

#[derive(Debug, Clone)]
pub(crate) struct OcrExecutionOutcomeContext {
    pub(crate) request_started: Instant,
    pub(crate) timeout_config: OcrTimeoutConfig,
    pub(crate) max_dimension: u32,
    pub(crate) prepared_width: Option<u32>,
    pub(crate) prepared_height: Option<u32>,
}

pub(crate) async fn execute_deepseek_ocr_blocking_task_for_runtime<F, T>(
    runtime: &DeepseekRuntime,
    timeout: Duration,
    stop_signal: Arc<AtomicBool>,
    task: F,
) -> Result<T>
where
    F: FnOnce(Arc<AtomicBool>) -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    if let DeepseekRuntime::RemoteHttp { .. } = runtime {
        return task(stop_signal);
    }

    let gate = deepseek_ocr_gate();
    execute_deepseek_ocr_blocking_task_with_gate(gate, timeout, stop_signal, task).await
}

pub(crate) async fn execute_deepseek_ocr_blocking_task_with_gate<F, T>(
    gate: Arc<Semaphore>,
    timeout: Duration,
    stop_signal: Arc<AtomicBool>,
    task: F,
) -> Result<T>
where
    F: FnOnce(Arc<AtomicBool>) -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    let gate_timeout = resolve_deepseek_ocr_gate_timeout();
    let permit = tokio::time::timeout(gate_timeout, gate.acquire_owned())
        .await
        .map_err(|_| {
            anyhow::anyhow!(
                "DeepSeek OCR gate acquisition timed out after {}s",
                gate_timeout.as_secs()
            )
        })??;

    let (result_tx, result_rx) = oneshot::channel();
    let worker_done = Arc::new(AtomicBool::new(false));
    let worker_done_for_thread = Arc::clone(&worker_done);
    let stop_signal_for_thread = Arc::clone(&stop_signal);

    let worker_id = rand::random::<u64>();
    spawn_deepseek_ocr_stuck_watchdog(worker_id, Arc::clone(&worker_done));

    std::thread::Builder::new()
        .name(format!("deepseek-ocr-worker-{}", worker_id))
        .spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                task(stop_signal_for_thread)
            }));
            worker_done_for_thread.store(true, Ordering::Release);
            let _ = result_tx.send(result);
        })?;

    let outcome = wait_for_ocr_worker_outcome(worker_id, result_rx, timeout, worker_done).await;

    match outcome {
        Ok(result) => {
            drop(permit);
            result
        }
        Err(error) => {
            // Signal the worker thread to stop via the shared atomic bool.
            // This will be checked in the progress callback and trigger a panic if supported by the engine.
            stop_signal.store(true, Ordering::Release);

            defer_deepseek_ocr_gate_release_until_worker_finishes(permit, worker_id);
            Err(error)
        }
    }
}

async fn wait_for_ocr_worker_outcome<T>(
    worker_id: u64,
    mut result_rx: oneshot::Receiver<std::thread::Result<Result<T>>>,
    timeout: Duration,
    worker_done: Arc<AtomicBool>,
) -> Result<T> {
    let mut ticker = tokio::time::interval(Duration::from_secs(5));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let started = Instant::now();

    loop {
        tokio::select! {
            result = &mut result_rx => {
                let thread_result = result.map_err(|_| anyhow::anyhow!("DeepSeek OCR worker thread {} dropped without result", worker_id))?;
                match thread_result {
                    Ok(task_result) => return task_result,
                    Err(payload) => {
                        let msg = panic_payload_to_string(&payload);
                        if msg.contains("deepseek_ocr_interrupted") {
                            return Err(anyhow::anyhow!("DeepSeek OCR worker thread {} successfully interrupted and stopped", worker_id));
                        }
                        return Err(anyhow::anyhow!("DeepSeek OCR worker thread {} panicked: {}", worker_id, msg));
                    }
                }
            }
            _ = ticker.tick() => {
                if started.elapsed() >= timeout {
                    return Err(anyhow::anyhow!("DeepSeek OCR worker thread {} timed out after {}s", worker_id, timeout.as_secs()));
                }
                if worker_done.load(Ordering::Acquire) {
                    tracing::debug!(worker_id, "DeepSeek OCR worker heartbeating: task marked as done but result channel not yet polled");
                } else {
                    tracing::debug!(worker_id, "DeepSeek OCR worker heartbeating: task still in progress ({}s elapsed)", started.elapsed().as_secs());
                }
            }
        }
    }
}

fn defer_deepseek_ocr_gate_release_until_worker_finishes(
    permit: tokio::sync::OwnedSemaphorePermit,
    worker_id: u64,
) {
    tokio::spawn(async move {
        let _permit = permit;
        let mut ticker = tokio::time::interval(Duration::from_secs(10));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        let started = Instant::now();

        tracing::warn!(
            worker_id,
            "DeepSeek OCR gate release deferred: waiting for timed-out or failed worker to terminate"
        );

        loop {
            ticker.tick().await;
            if started.elapsed() > Duration::from_secs(300) {
                tracing::error!(
                    worker_id,
                    "DeepSeek OCR deferred gate release timeout: force releasing permit after 5 minutes of worker hang"
                );
                break;
            }
        }
    });
}

pub(crate) fn handle_deepseek_ocr_execution_outcome(
    result: Result<Option<String>>,
    context: OcrExecutionOutcomeContext,
) -> Option<Option<String>> {
    match result {
        Ok(markdown) => Some(markdown),
        Err(error) => {
            tracing::error!(
                event = "agent.llm.vision.deepseek.ocr.execution_failed",
                error = %error,
                max_dimension = context.max_dimension,
                width = context.prepared_width,
                height = context.prepared_height,
                elapsed_ms = context.request_started.elapsed().as_millis(),
                "DeepSeek OCR execution failed"
            );
            None
        }
    }
}

fn panic_payload_to_string(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(value) = payload.downcast_ref::<&str>() {
        (*value).to_string()
    } else if let Some(value) = payload.downcast_ref::<String>() {
        value.clone()
    } else {
        "unknown panic payload".to_string()
    }
}
