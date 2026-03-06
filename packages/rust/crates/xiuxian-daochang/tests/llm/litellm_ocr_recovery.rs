use std::path::Path;
use xiuxian_daochang::test_support::{
    deepseek_ocr_memory_guard_triggered, resolve_deepseek_ocr_global_lock_path,
    resolve_deepseek_ocr_memory_limit_bytes, simulate_ocr_gate_panic_recovery,
    simulate_ocr_gate_timeout_recovery,
};

#[test]
fn deepseek_ocr_global_lock_path_targets_lock_file() {
    let path = resolve_deepseek_ocr_global_lock_path();
    let parsed = Path::new(path.as_str());
    let file_name = parsed.file_name().and_then(|value| value.to_str());
    let parent_name = parsed
        .parent()
        .and_then(|value| value.file_name())
        .and_then(|value| value.to_str());
    assert_eq!(file_name, Some("deepseek-ocr.lock"));
    assert_eq!(parent_name, Some("locks"));
}

#[tokio::test]
async fn deepseek_ocr_gate_blocks_new_work_after_timeout_until_worker_finishes() {
    let probe = simulate_ocr_gate_timeout_recovery(200, 20).await;

    assert!(
        probe.first_timed_out(),
        "probe setup failed: first OCR task should timeout"
    );
    assert!(
        !probe.first_panicked(),
        "timeout probe unexpectedly panicked"
    );
    assert!(
        probe.second_was_busy,
        "OCR gate should remain busy after timeout while timed-out worker is still running"
    );
    assert!(
        !probe.second_completed,
        "second OCR probe should not complete immediately when timed-out worker still owns gate"
    );
    assert!(
        probe.recovered_after_wait,
        "OCR gate should recover once timed-out worker actually finishes"
    );
}

#[tokio::test]
async fn deepseek_ocr_gate_recovery_stress_probe_is_safe_and_recovers() {
    for _ in 0..16 {
        let probe = simulate_ocr_gate_timeout_recovery(120, 20).await;
        assert!(probe.first_timed_out());
        assert!(!probe.first_panicked());
        assert!(probe.second_was_busy);
        assert!(!probe.second_completed);
        assert!(probe.recovered_after_wait);
    }
}

#[tokio::test]
async fn deepseek_ocr_gate_recovers_after_worker_panic_without_stuck_busy() {
    let probe = simulate_ocr_gate_panic_recovery().await;

    assert!(
        probe.first_panicked(),
        "probe setup failed: first OCR task should panic"
    );
    assert!(
        !probe.second_was_busy,
        "OCR gate remained busy after worker panic; this causes stale busy loops in runtime"
    );
    assert!(
        probe.second_completed,
        "second OCR probe should complete once panic path releases the gate"
    );
    assert!(
        probe.recovered_after_wait,
        "panic path should already be recovered on immediate retry"
    );
}

#[test]
fn deepseek_ocr_memory_limit_parser_accepts_positive_gb_values() {
    let Some(limit) = resolve_deepseek_ocr_memory_limit_bytes(Some("8")) else {
        panic!("8 GB limit should parse into bytes");
    };
    assert_eq!(limit, 8 * 1024 * 1024 * 1024);
}

#[test]
fn deepseek_ocr_memory_limit_parser_rejects_zero_or_invalid_values() {
    assert!(resolve_deepseek_ocr_memory_limit_bytes(Some("0")).is_none());
    assert!(resolve_deepseek_ocr_memory_limit_bytes(Some("-1")).is_none());
    assert!(resolve_deepseek_ocr_memory_limit_bytes(Some("not-a-number")).is_none());
}

#[test]
fn deepseek_ocr_memory_guard_trips_when_rss_exceeds_limit() {
    let over_limit = deepseek_ocr_memory_guard_triggered(Some("0.000001"), 16 * 1024);
    assert!(
        over_limit,
        "memory guard should trip when RSS is above tiny configured limit"
    );

    let within_limit = deepseek_ocr_memory_guard_triggered(Some("8"), 64 * 1024 * 1024);
    assert!(
        !within_limit,
        "memory guard should not trip when RSS is below configured limit"
    );
}
