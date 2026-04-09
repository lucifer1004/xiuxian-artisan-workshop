use std::path::PathBuf;
use std::time::Duration;

use crate::repo_index::state::task::AdaptiveConcurrencyController;
use crate::repo_index::state::tests::new_coordinator;
use crate::search::SearchPlaneService;

#[test]
fn adaptive_controller_expands_with_backlog_and_fast_feedback() {
    let mut controller = AdaptiveConcurrencyController::new_for_test(4);

    assert_eq!(controller.target_limit(8, 0), 1);

    controller.record_success(Duration::from_millis(20), 7);
    assert_eq!(controller.target_limit(7, 0), 2);

    controller.record_success(Duration::from_millis(18), 6);
    assert_eq!(controller.target_limit(6, 0), 2);

    controller.record_success(Duration::from_millis(18), 5);
    assert_eq!(controller.target_limit(5, 0), 3);

    controller.record_failure();
    assert_eq!(controller.target_limit(5, 0), 1);
}

#[test]
fn adaptive_controller_contracts_when_efficiency_collapses() {
    let mut controller = AdaptiveConcurrencyController::new_for_test(6);
    controller.current_limit = 4;
    controller.reference_limit = 4;
    controller.ema_elapsed_ms = Some(100.0);
    controller.baseline_elapsed_ms = Some(100.0);
    controller.previous_efficiency = Some(4.0 / 100.0);

    controller.record_success(Duration::from_millis(600), 8);

    assert_eq!(controller.target_limit(8, 0), 2);
}

#[test]
fn adaptive_controller_keeps_parallelism_when_feedback_stays_near_baseline() {
    let mut controller = AdaptiveConcurrencyController::new_for_test(6);
    controller.current_limit = 3;
    controller.reference_limit = 3;
    controller.ema_elapsed_ms = Some(100.0);
    controller.baseline_elapsed_ms = Some(100.0);
    controller.previous_efficiency = Some(3.0 / 100.0);

    controller.record_success(Duration::from_millis(110), 8);

    assert_eq!(controller.target_limit(8, 0), 3);
}

#[tokio::test]
async fn sync_permit_blocks_after_reaching_configured_remote_sync_limit() {
    let coordinator = new_coordinator(SearchPlaneService::new(PathBuf::from(".")));
    let mut held_permits = Vec::new();
    for permit_index in 0..coordinator.sync_concurrency_limit {
        held_permits.push(
            coordinator
                .acquire_sync_permit(format!("repo-{permit_index}").as_str())
                .await
                .unwrap_or_else(|error| panic!("permit {permit_index}: {error}")),
        );
    }
    let blocked = tokio::time::timeout(
        Duration::from_millis(25),
        coordinator.acquire_sync_permit("repo-blocked"),
    )
    .await;
    assert!(blocked.is_err());

    drop(held_permits);

    let next = tokio::time::timeout(
        Duration::from_secs(1),
        coordinator.acquire_sync_permit("repo-released"),
    )
    .await
    .unwrap_or_else(|error| panic!("next permit should become available: {error}"))
    .unwrap_or_else(|error| panic!("next permit acquisition failed: {error}"));
    drop(next);
}
