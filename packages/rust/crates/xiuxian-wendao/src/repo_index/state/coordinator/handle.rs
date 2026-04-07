use std::sync::Arc;

use tokio::runtime::Handle;
use tokio::sync::Notify;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::repo_index::state::coordinator::RepoIndexCoordinator;

pub(super) struct RepoIndexRuntimeHandle {
    cancellation: CancellationToken,
    tracker: TaskTracker,
}

impl RepoIndexRuntimeHandle {
    pub(super) fn spawn(handle: &Handle, coordinator: Arc<RepoIndexCoordinator>) -> Self {
        let cancellation = CancellationToken::new();
        let tracker = TaskTracker::new();
        let runner_cancellation = cancellation.clone();
        tracker.spawn_on(
            async move {
                coordinator.run(runner_cancellation).await;
            },
            handle,
        );
        Self {
            cancellation,
            tracker,
        }
    }

    pub(super) fn stop(self, notify: &Notify) {
        self.tracker.close();
        self.cancellation.cancel();
        notify.notify_waiters();
    }
}
