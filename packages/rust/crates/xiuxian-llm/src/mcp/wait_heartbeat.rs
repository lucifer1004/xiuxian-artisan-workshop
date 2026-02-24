//! Heartbeat classification helpers for MCP in-flight waits.

use crate::mcp::health::HealthProbeStatus;

const DEFAULT_INFLIGHT_LOG_INTERVAL_SECS: u64 = 5;
const DEFAULT_WAIT_HEARTBEAT_DEGRADED_WARN_AFTER_SECS: u64 = 30;

/// Wait heartbeat classification used for in-flight call logging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaitHeartbeatState {
    /// Health payload indicates server is ready.
    Healthy,
    /// Server appears reachable but is still initializing or timing out.
    Degraded,
    /// Structured payload indicates unhealthy/non-ready state.
    Unhealthy,
}

/// Classify health probe state into wait-heartbeat buckets.
#[must_use]
pub fn classify_wait_heartbeat(health_probe: &HealthProbeStatus) -> WaitHeartbeatState {
    if health_probe.has_structured_ready_state {
        return if health_probe.ready == Some(true) && health_probe.initializing == Some(false) {
            WaitHeartbeatState::Healthy
        } else if health_probe.ready == Some(false) && health_probe.initializing == Some(true) {
            WaitHeartbeatState::Degraded
        } else {
            WaitHeartbeatState::Unhealthy
        };
    }
    if matches!(health_probe.status_code, Some(code) if (200..300).contains(&code)) {
        return WaitHeartbeatState::Healthy;
    }
    if health_probe.timed_out || health_probe.transport_error {
        return WaitHeartbeatState::Degraded;
    }
    WaitHeartbeatState::Degraded
}

/// Clamp degraded-heartbeat warning threshold to a safe interval.
#[must_use]
pub fn degraded_wait_warn_after_secs(timeout_secs: u64) -> u64 {
    timeout_secs.clamp(
        DEFAULT_INFLIGHT_LOG_INTERVAL_SECS,
        DEFAULT_WAIT_HEARTBEAT_DEGRADED_WARN_AFTER_SECS,
    )
}
