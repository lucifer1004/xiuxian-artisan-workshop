use std::path::PathBuf;
use std::sync::Arc;

use crate::repo_index::state::tests::new_coordinator;
use crate::search::SearchPlaneService;

#[tokio::test]
async fn start_is_idempotent_and_stop_clears_runtime_handle() {
    let coordinator = Arc::new(new_coordinator(SearchPlaneService::new(PathBuf::from("."))));

    coordinator.start();
    tokio::task::yield_now().await;
    assert!(
        coordinator
            .runtime_handle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    );

    coordinator.start();
    tokio::task::yield_now().await;
    assert!(
        coordinator
            .runtime_handle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_some()
    );

    coordinator.stop();
    assert!(
        coordinator
            .runtime_handle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .is_none()
    );
}
