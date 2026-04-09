use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::error::BackendError;
use super::tuning::default_remote_operation_timeout;

const REMOTE_OPERATION_TIMEOUT_ENV: &str = "XIUXIAN_GIT_REPO_REMOTE_OPERATION_TIMEOUT_SECS";

pub(crate) fn run_interruptible_remote_operation<T>(
    operation_name: &str,
    operation: impl FnOnce(&AtomicBool) -> Result<T, BackendError>,
) -> Result<T, BackendError> {
    run_interruptible_remote_operation_with_timeout(
        operation_name,
        remote_operation_timeout(),
        operation,
    )
}

fn run_interruptible_remote_operation_with_timeout<T>(
    operation_name: &str,
    timeout: Duration,
    operation: impl FnOnce(&AtomicBool) -> Result<T, BackendError>,
) -> Result<T, BackendError> {
    let should_interrupt = Arc::new(AtomicBool::new(false));
    let deadline = RemoteOperationDeadline::start(timeout, Arc::clone(&should_interrupt));
    let result = operation(should_interrupt.as_ref());
    let timed_out = deadline.finish();

    match result {
        Ok(value) => Ok(value),
        Err(_) if timed_out => Err(BackendError::new(format!(
            "remote operation `{operation_name}` timed out after {}",
            format_timeout(timeout)
        ))),
        Err(error) => Err(error),
    }
}

fn remote_operation_timeout() -> Duration {
    remote_operation_timeout_with_lookup(&|key| std::env::var(key).ok())
}

fn remote_operation_timeout_with_lookup(lookup: &dyn Fn(&str) -> Option<String>) -> Duration {
    lookup(REMOTE_OPERATION_TIMEOUT_ENV)
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0)
        .map_or_else(default_remote_operation_timeout, Duration::from_secs)
}

fn format_timeout(timeout: Duration) -> String {
    if timeout.as_secs() > 0 {
        format!("{}s", timeout.as_secs())
    } else {
        format!("{}ms", timeout.as_millis())
    }
}

struct RemoteOperationDeadline {
    cancel_tx: Option<mpsc::Sender<()>>,
    worker: Option<JoinHandle<()>>,
    timed_out: Arc<AtomicBool>,
}

impl RemoteOperationDeadline {
    fn start(timeout: Duration, should_interrupt: Arc<AtomicBool>) -> Self {
        let (cancel_tx, cancel_rx) = mpsc::channel();
        let timed_out = Arc::new(AtomicBool::new(false));
        let timed_out_for_worker = Arc::clone(&timed_out);
        let worker = thread::spawn(move || {
            if cancel_rx.recv_timeout(timeout).is_err() {
                timed_out_for_worker.store(true, Ordering::SeqCst);
                should_interrupt.store(true, Ordering::SeqCst);
            }
        });
        Self {
            cancel_tx: Some(cancel_tx),
            worker: Some(worker),
            timed_out,
        }
    }

    fn finish(mut self) -> bool {
        self.cancel();
        self.timed_out.load(Ordering::SeqCst)
    }

    fn cancel(&mut self) {
        if let Some(cancel_tx) = self.cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for RemoteOperationDeadline {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
#[path = "../../../tests/unit/backend/gix/interrupt.rs"]
mod tests;
