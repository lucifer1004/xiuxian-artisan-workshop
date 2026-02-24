//! Wait-heartbeat logging utilities for MCP pool calls.

use std::time::Duration;

use tokio::sync::oneshot;

use crate::mcp::{
    WaitHeartbeatState, classify_wait_heartbeat, degraded_wait_warn_after_secs, probe_health_status,
};

const DEFAULT_INFLIGHT_LOG_INTERVAL_SECS: u64 = 5;
const DEFAULT_WAIT_HEARTBEAT_DEGRADED_WARN_STREAK: u32 = 3;

/// Spawn periodic wait logger for in-flight MCP call operations.
#[must_use]
pub fn spawn_inflight_wait_logger(
    operation: String,
    server_url: String,
    client_index: usize,
    timeout: Duration,
) -> (tokio::task::JoinHandle<()>, oneshot::Sender<()>) {
    let (stop_tx, mut stop_rx) = oneshot::channel::<()>();
    let timeout_secs = timeout.as_secs().max(1);
    let overdue_limit_secs = timeout_secs.saturating_add(DEFAULT_INFLIGHT_LOG_INTERVAL_SECS);
    let degraded_warn_after_secs = degraded_wait_warn_after_secs(timeout_secs);
    let handle = tokio::spawn(async move {
        let mut waited_secs = DEFAULT_INFLIGHT_LOG_INTERVAL_SECS;
        let mut degraded_streak: u32 = 0;
        loop {
            tokio::select! {
                () = tokio::time::sleep(Duration::from_secs(DEFAULT_INFLIGHT_LOG_INTERVAL_SECS)) => {}
                _ = &mut stop_rx => break,
            }
            let health_probe = probe_health_status(&server_url).await;
            match classify_wait_heartbeat(&health_probe) {
                WaitHeartbeatState::Healthy => {
                    degraded_streak = 0;
                    tracing::debug!(
                        event = "mcp.pool.call.waiting.heartbeat_ok",
                        operation = %operation,
                        client_index,
                        waited_secs,
                        timeout_secs,
                        health_probe = %health_probe.summary,
                        heartbeat_state = "healthy",
                        "mcp call still waiting but health heartbeat is healthy"
                    );
                }
                WaitHeartbeatState::Degraded => {
                    degraded_streak = degraded_streak.saturating_add(1);
                    if waited_secs >= degraded_warn_after_secs
                        && degraded_streak >= DEFAULT_WAIT_HEARTBEAT_DEGRADED_WARN_STREAK
                    {
                        tracing::warn!(
                            event = "mcp.pool.call.waiting",
                            operation = %operation,
                            client_index,
                            waited_secs,
                            timeout_secs,
                            health_probe = %health_probe.summary,
                            heartbeat_state = "degraded",
                            degraded_streak,
                            degraded_warn_after_secs,
                            "mcp call still waiting and health heartbeat remains degraded"
                        );
                    } else {
                        tracing::debug!(
                            event = "mcp.pool.call.waiting.heartbeat_degraded",
                            operation = %operation,
                            client_index,
                            waited_secs,
                            timeout_secs,
                            health_probe = %health_probe.summary,
                            heartbeat_state = "degraded",
                            degraded_streak,
                            degraded_warn_after_secs,
                            "mcp call still waiting with temporary heartbeat degradation"
                        );
                    }
                }
                WaitHeartbeatState::Unhealthy => {
                    degraded_streak = degraded_streak.saturating_add(1);
                    tracing::warn!(
                        event = "mcp.pool.call.waiting",
                        operation = %operation,
                        client_index,
                        waited_secs,
                        timeout_secs,
                        health_probe = %health_probe.summary,
                        heartbeat_state = "unhealthy",
                        degraded_streak,
                        "mcp call still waiting and health heartbeat is unhealthy"
                    );
                }
            }
            if waited_secs >= overdue_limit_secs {
                tracing::warn!(
                    event = "mcp.pool.call.waiting.guard_stop",
                    operation = %operation,
                    client_index,
                    waited_secs,
                    timeout_secs,
                    "mcp call wait logger stopped after exceeding timeout guard"
                );
                break;
            }
            waited_secs += DEFAULT_INFLIGHT_LOG_INTERVAL_SECS;
        }
    });
    (handle, stop_tx)
}

/// Stop wait logger task by sending stop signal and awaiting completion.
pub async fn stop_wait_logger(handle: tokio::task::JoinHandle<()>, stop_tx: oneshot::Sender<()>) {
    let _ = stop_tx.send(());
    let _ = handle.await;
}
