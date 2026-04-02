use std::time::Duration;

/// Default base URL for a local Flight-backed Julia analyzer.
pub const DEFAULT_FLIGHT_BASE_URL: &str = "http://127.0.0.1:8815";
/// Default Wendao Flight schema contract version.
pub const DEFAULT_FLIGHT_SCHEMA_VERSION: &str = "v1";
/// Canonical Arrow schema metadata key for the Wendao schema version.
pub const FLIGHT_SCHEMA_VERSION_METADATA_KEY: &str = "wendao.schema_version";
/// Canonical Arrow schema metadata key for request/response trace identifiers.
pub const FLIGHT_TRACE_ID_METADATA_KEY: &str = "trace_id";
/// Default timeout for runtime-owned Flight roundtrips.
pub const DEFAULT_FLIGHT_TIMEOUT_SECS: u64 = 10;

/// Validate a non-empty Flight schema version string.
///
/// # Errors
///
/// Returns an error when the provided version is blank after trimming.
pub fn validate_flight_schema_version(schema_version: &str) -> Result<String, String> {
    let normalized = schema_version.trim();
    if normalized.is_empty() {
        return Err("Flight schema version must not be blank".to_string());
    }
    Ok(normalized.to_string())
}

/// Validate a non-zero timeout value for Flight roundtrips.
///
/// # Errors
///
/// Returns an error when the provided timeout is zero.
pub fn validate_flight_timeout_secs(timeout_secs: u64) -> Result<u64, String> {
    if timeout_secs == 0 {
        return Err("Flight timeout_secs must be greater than zero".to_string());
    }
    Ok(timeout_secs)
}

/// Resolve a runtime timeout from an optional `timeout_secs` override.
///
/// # Errors
///
/// Returns an error when the provided timeout override is zero.
pub fn resolve_flight_timeout(timeout_secs: Option<u64>) -> Result<Duration, String> {
    let timeout_secs = match timeout_secs {
        Some(timeout_secs) => validate_flight_timeout_secs(timeout_secs)?,
        None => DEFAULT_FLIGHT_TIMEOUT_SECS,
    };
    Ok(Duration::from_secs(timeout_secs))
}
