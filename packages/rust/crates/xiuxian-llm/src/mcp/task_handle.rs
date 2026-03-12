//! Abort-on-drop task handle utilities for MCP pool operations.

use std::time::Duration;

use anyhow::Result;
use tokio::task::{JoinError, JoinHandle};

pub(super) struct AbortOnDropJoinHandle<T> {
    handle: JoinHandle<T>,
    abort_on_drop: bool,
}

impl<T> AbortOnDropJoinHandle<T> {
    pub(super) fn new(handle: JoinHandle<T>) -> Self {
        Self {
            handle,
            abort_on_drop: true,
        }
    }

    pub(super) async fn timeout(
        &mut self,
        timeout: Duration,
    ) -> Result<Result<T, JoinError>, tokio::time::error::Elapsed> {
        tokio::time::timeout(timeout, &mut self.handle).await
    }

    pub(super) fn abort(&mut self) {
        self.handle.abort();
    }

    pub(super) fn disarm(&mut self) {
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
