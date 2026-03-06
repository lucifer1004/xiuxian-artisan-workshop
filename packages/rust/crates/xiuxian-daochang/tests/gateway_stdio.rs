//! Test coverage for xiuxian-daochang behavior.

//! Unit tests for stdio gateway: constants and wiring (no stdin loop).

use xiuxian_daochang::DEFAULT_STDIO_SESSION_ID;

#[test]
fn default_stdio_session_id() {
    assert_eq!(DEFAULT_STDIO_SESSION_ID, "default");
}

#[test]
fn gateway_exports_run_stdio() {
    // Compile-time check that run_stdio is in the public API.
    let _ = xiuxian_daochang::run_stdio;
}
