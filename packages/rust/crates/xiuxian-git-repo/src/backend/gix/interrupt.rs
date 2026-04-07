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
mod tests {
    use std::sync::atomic::Ordering;
    use std::thread;
    use std::time::{Duration, Instant};

    use super::{
        remote_operation_timeout_with_lookup, run_interruptible_remote_operation_with_timeout,
    };
    use crate::backend::BackendError;
    use crate::backend::gix::tuning::default_remote_operation_timeout_for_parallelism;

    #[test]
    fn remote_operation_timeout_defaults_when_env_is_missing() {
        assert_eq!(
            remote_operation_timeout_with_lookup(&|_| None),
            default_remote_operation_timeout_for_parallelism(
                std::thread::available_parallelism()
                    .map(std::num::NonZeroUsize::get)
                    .unwrap_or(1),
            )
        );
    }

    #[test]
    fn remote_operation_timeout_uses_positive_override() {
        let timeout = remote_operation_timeout_with_lookup(&|key| {
            (key == "XIUXIAN_GIT_REPO_REMOTE_OPERATION_TIMEOUT_SECS").then(|| "12".to_string())
        });
        assert_eq!(timeout.as_secs(), 12);
    }

    #[test]
    fn remote_operation_timeout_ignores_invalid_override() {
        let timeout = remote_operation_timeout_with_lookup(&|key| {
            (key == "XIUXIAN_GIT_REPO_REMOTE_OPERATION_TIMEOUT_SECS").then(|| "invalid".to_string())
        });
        assert_eq!(
            timeout,
            default_remote_operation_timeout_for_parallelism(
                std::thread::available_parallelism()
                    .map(std::num::NonZeroUsize::get)
                    .unwrap_or(1),
            )
        );
    }

    #[test]
    fn interruptible_remote_operation_returns_immediately_on_fast_success_path() {
        let started_at = Instant::now();
        let result = run_interruptible_remote_operation_with_timeout(
            "fetch origin",
            Duration::from_secs(1),
            |_should_interrupt| Ok::<usize, BackendError>(7),
        );
        let elapsed = started_at.elapsed();

        assert_eq!(result.ok(), Some(7));
        assert!(
            elapsed < Duration::from_millis(250),
            "fast success path should not wait on the full timeout budget, elapsed={elapsed:?}"
        );
    }

    #[test]
    fn interruptible_remote_operation_reports_timeout_when_watchdog_fires() {
        let result = run_interruptible_remote_operation_with_timeout(
            "fetch origin",
            Duration::from_millis(20),
            |should_interrupt| {
                while !should_interrupt.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(5));
                }
                Err::<(), BackendError>(BackendError::new("operation interrupted"))
            },
        );
        let result = match result {
            Ok(()) => panic!("expected timeout failure"),
            Err(error) => error,
        };

        assert!(result.message().contains("fetch origin"));
        assert!(result.message().contains("timed out"));
    }

    #[test]
    fn interruptible_remote_operation_preserves_non_timeout_errors() {
        let result = run_interruptible_remote_operation_with_timeout(
            "probe remote",
            Duration::from_secs(1),
            |_should_interrupt| Err::<(), BackendError>(BackendError::new("authentication failed")),
        );
        let result = match result {
            Ok(()) => panic!("expected authentication failure"),
            Err(error) => error,
        };

        assert_eq!(result.message(), "authentication failed");
    }
}
