//! General testing utilities.

use serde_json::Value;

/// Assert that two JSON values are equal, with a helpful diff message.
///
/// # Panics
///
/// Panics if the values are not equal.
#[track_caller]
pub fn assert_json_eq(expected: &Value, actual: &Value) {
    assert_eq!(
        expected,
        actual,
        "JSON mismatch:\n--- expected ---\n{}\n--- actual ---\n{}",
        serde_json::to_string_pretty(expected).unwrap_or_default(),
        serde_json::to_string_pretty(actual).unwrap_or_default()
    );
}

/// Create a temporary directory with a given prefix.
///
/// # Panics
///
/// Panics if the directory cannot be created.
#[must_use]
pub fn temp_dir_with_prefix(prefix: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(prefix)
        .tempdir()
        .unwrap_or_else(|error| panic!("failed to create temp directory: {error}"))
}

#[cfg(test)]
#[path = "../tests/unit/utils.rs"]
mod tests;
