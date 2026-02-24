//! Shared helper functions for MCP pool runtime.

use std::time::Duration;

const DEFAULT_SLOW_CALL_WARN_MS: u128 = 2_000;
const DEFAULT_LONG_TOOL_SLOW_WARN_MS: u128 = 20_000;
const MAX_LONG_TOOL_SLOW_WARN_MS: u128 = 60_000;
const MAX_LIST_TOOLS_CACHE_TTL_MS: u64 = 60_000;
const DEFAULT_MEMORY_SAVE_TOOL_TIMEOUT_SECS: u64 = 5;

/// Clamp list-tools cache TTL to a safe range and return duration.
#[must_use]
pub fn list_tools_cache_ttl_from_config(raw_ms: u64) -> Duration {
    let sanitized = raw_ms.clamp(1, MAX_LIST_TOOLS_CACHE_TTL_MS);
    Duration::from_millis(sanitized)
}

/// Compute slow-call warning threshold based on operation and timeout budget.
#[must_use]
pub fn call_slow_warn_threshold_ms(operation: &str, timeout: Duration) -> u128 {
    if is_expected_long_running_tool(operation) {
        return timeout
            .as_millis()
            .saturating_div(3)
            .clamp(DEFAULT_LONG_TOOL_SLOW_WARN_MS, MAX_LONG_TOOL_SLOW_WARN_MS);
    }
    timeout
        .as_millis()
        .saturating_div(6)
        .max(DEFAULT_SLOW_CALL_WARN_MS)
}

/// Derive effective timeout budget for a tool call.
#[must_use]
pub fn call_timeout_for_tool(tool_name: &str, default_timeout: Duration) -> Duration {
    if tool_name == "memory.save_memory" {
        return default_timeout.min(Duration::from_secs(DEFAULT_MEMORY_SAVE_TOOL_TIMEOUT_SECS));
    }
    default_timeout
}

/// Identify long-running MCP operations with elevated slow-call threshold.
#[must_use]
pub fn is_expected_long_running_tool(operation: &str) -> bool {
    operation.starts_with("tools/call:crawl4ai.")
}

/// Compute percentage with two decimals using integer basis-point arithmetic.
#[must_use]
pub fn hit_rate_pct_two_decimals(hits: u64, requests: u64) -> f64 {
    if requests == 0 {
        return 0.0;
    }
    let basis_points = hits
        .saturating_mul(10_000)
        .checked_div(requests)
        .unwrap_or_default();
    let basis_points = u16::try_from(basis_points).unwrap_or(u16::MAX);
    f64::from(basis_points) / 100.0
}
