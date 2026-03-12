use omni_agent::test_support::{
    resolve_deepseek_ocr_global_lock_path, simulate_ocr_gate_panic_recovery,
    simulate_ocr_gate_timeout_recovery,
};
use std::path::Path;

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
