use std::sync::Arc;

use super::types::{OcrGateTimeoutRecoveryProbe, OcrProbeFirstOutcome};

/// Simulate OCR timeout/busy recovery without starting channel runtimes.
pub async fn simulate_ocr_gate_timeout_recovery(
    blocking_sleep_ms: u64,
    timeout_ms: u64,
) -> OcrGateTimeoutRecoveryProbe {
    let probe = crate::model_host::ocr::simulate_ocr_gate_timeout_recovery_for_tests(
        blocking_sleep_ms,
        timeout_ms,
    )
    .await;
    OcrGateTimeoutRecoveryProbe {
        first_outcome: map_probe_first_outcome(probe.first_outcome),
        second_was_busy: probe.second_was_busy,
        second_completed: probe.second_completed,
        recovered_after_wait: probe.recovered_after_wait,
    }
}

/// Simulate OCR panic/busy recovery without starting channel runtimes.
pub async fn simulate_ocr_gate_panic_recovery() -> OcrGateTimeoutRecoveryProbe {
    let probe = crate::model_host::ocr::simulate_ocr_gate_panic_recovery_for_tests().await;
    OcrGateTimeoutRecoveryProbe {
        first_outcome: map_probe_first_outcome(probe.first_outcome),
        second_was_busy: probe.second_was_busy,
        second_completed: probe.second_completed,
        recovered_after_wait: probe.recovered_after_wait,
    }
}

/// Probe `DeepSeek` OCR truth extraction from raw image bytes.
pub async fn infer_deepseek_ocr_truth_from_image_bytes(
    image_bytes: Vec<u8>,
    media_type: &str,
) -> Option<String> {
    crate::model_host::ocr::infer_deepseek_ocr_truth_from_bytes_for_tests(
        Arc::from(image_bytes.into_boxed_slice()),
        media_type,
    )
    .await
}

/// Resolve effective global OCR process-lock file path.
#[must_use]
pub fn resolve_deepseek_ocr_global_lock_path() -> String {
    crate::model_host::ocr::deepseek_ocr_global_lock_path()
        .display()
        .to_string()
}

/// Resolve OCR RSS guard threshold bytes from a raw GB string.
#[must_use]
pub fn resolve_deepseek_ocr_memory_limit_bytes(raw_limit_gb: Option<&str>) -> Option<u64> {
    crate::model_host::ocr::resolve_deepseek_ocr_memory_limit_bytes(raw_limit_gb)
}

/// Evaluate whether OCR memory guard should trigger for a given RSS usage.
#[must_use]
pub fn deepseek_ocr_memory_guard_triggered(raw_limit_gb: Option<&str>, rss_bytes: u64) -> bool {
    crate::model_host::ocr::deepseek_ocr_memory_guard_triggered(raw_limit_gb, rss_bytes)
}

fn map_probe_first_outcome(
    outcome: crate::model_host::ocr::OcrProbeFirstOutcome,
) -> OcrProbeFirstOutcome {
    match outcome {
        crate::model_host::ocr::OcrProbeFirstOutcome::TimedOut => OcrProbeFirstOutcome::TimedOut,
        crate::model_host::ocr::OcrProbeFirstOutcome::Panicked => OcrProbeFirstOutcome::Panicked,
        crate::model_host::ocr::OcrProbeFirstOutcome::Other => OcrProbeFirstOutcome::Other,
    }
}
