//! Abort-on-drop task handle utilities for MCP pool operations.

use std::time::Duration;

use anyhow::Result;
use tokio::task::{JoinError, JoinHandle};

/// Abort-on-drop wrapper for join handles used by MCP request workers.
pub struct AbortOnDropJoinHandle<T> {
    handle: JoinHandle<T>,
    abort_on_drop: bool,
}

impl<T> AbortOnDropJoinHandle<T> {
    /// Create a new wrapper that aborts on drop by default.
    #[must_use]
    pub fn new(handle: JoinHandle<T>) -> Self {
        Self {
            handle,
            abort_on_drop: true,
        }
    }

    /// Await the task with a timeout.
    ///
    /// # Errors
    /// Returns timeout elapsed when wait exceeded.
    pub async fn timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Result<T, JoinError>, tokio::time::error::Elapsed> {
        tokio::time::timeout(timeout, &mut self.handle).await
    }

    /// Abort the underlying task immediately.
    pub fn abort(&mut self) {
        self.handle.abort();
    }

    /// Disable abort-on-drop behavior after a successful completion path.
    pub fn disarm(&mut self) {
        self.abort_on_drop = false;
    }
}

impl<T> Drop for AbortOnDropJoinHandle<T> {
    fn drop(&mut self) {
        if self.abort_on_drop {
            self.handle.abort();
        }
    }
}
