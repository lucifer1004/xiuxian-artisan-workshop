use std::time::Duration;

use super::{AdaptiveConcurrencyAdjustment, AdaptiveConcurrencyController};

#[test]
fn debug_snapshot_marks_io_pressure_contractions() {
    let mut controller = AdaptiveConcurrencyController::new_for_test(8);
    controller.current_limit = 6;
    controller.reference_limit = 6;
    controller.ema_elapsed_ms = Some(100.0);
    controller.baseline_elapsed_ms = Some(100.0);
    controller.previous_efficiency = Some(6.0 / 100.0);
    controller.io_pressure_streak = 1;

    controller.record_success(Duration::from_millis(1_300), 12);

    assert_eq!(
        controller.last_adjustment,
        AdaptiveConcurrencyAdjustment::ContractedIoPressure
    );
    assert_eq!(controller.current_limit, 3);
    assert_eq!(controller.last_elapsed_ms, Some(1_300));
    assert_eq!(controller.io_pressure_streak, 0);
}

#[test]
fn debug_snapshot_marks_failure_contractions() {
    let mut controller = AdaptiveConcurrencyController::new_for_test(8);
    controller.current_limit = 4;

    controller.record_failure();

    assert_eq!(
        controller.last_adjustment,
        AdaptiveConcurrencyAdjustment::ContractedFailure
    );
    assert_eq!(controller.current_limit, 2);
}

#[test]
fn debug_snapshot_resets_baseline_when_limit_changes() {
    let mut controller = AdaptiveConcurrencyController::new_for_test(8);
    controller.current_limit = 4;
    controller.reference_limit = 4;
    controller.ema_elapsed_ms = Some(120.0);
    controller.baseline_elapsed_ms = Some(120.0);
    controller.previous_efficiency = Some(4.0 / 120.0);

    controller.current_limit = 6;
    controller.record_success(Duration::from_millis(2_400), 12);

    assert_eq!(controller.current_limit, 6);
    assert_eq!(controller.reference_limit, 6);
    assert_eq!(controller.baseline_elapsed_ms, Some(2_400.0));
    assert_eq!(
        controller.last_adjustment,
        AdaptiveConcurrencyAdjustment::Stable
    );
}

#[test]
fn debug_snapshot_requires_sustained_io_pressure_before_contracting() {
    let mut controller = AdaptiveConcurrencyController::new_for_test(8);
    controller.current_limit = 6;
    controller.reference_limit = 6;
    controller.ema_elapsed_ms = Some(100.0);
    controller.baseline_elapsed_ms = Some(100.0);
    controller.previous_efficiency = Some(6.0 / 100.0);

    controller.record_success(Duration::from_millis(1_300), 12);

    assert_eq!(controller.current_limit, 6);
    assert_eq!(
        controller.last_adjustment,
        AdaptiveConcurrencyAdjustment::ObservedIoPressure
    );
    assert_eq!(controller.io_pressure_streak, 1);
}
