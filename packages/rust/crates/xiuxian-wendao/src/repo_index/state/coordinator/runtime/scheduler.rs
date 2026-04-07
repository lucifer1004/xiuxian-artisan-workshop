use std::panic::AssertUnwindSafe;
use std::sync::Arc;

use futures::FutureExt;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use crate::repo_index::state::coordinator::RepoIndexCoordinator;
use crate::repo_index::state::task::{
    RepoTaskFeedback, RepoTaskOutcome, should_penalize_adaptive_concurrency,
};
use crate::repo_index::types::RepoIndexPhase;

enum RepoTaskJoinResult {
    Completed(Box<RepoTaskFeedback>),
    Panicked { repo_id: String },
}

impl RepoIndexCoordinator {
    pub(crate) async fn run(self: Arc<Self>, shutdown: CancellationToken) {
        let mut running = JoinSet::new();
        loop {
            if shutdown.is_cancelled() {
                break;
            }

            self.dispatch_pending_tasks(&mut running);

            if shutdown.is_cancelled() {
                break;
            }

            if running.is_empty() {
                tokio::select! {
                    biased;
                    () = shutdown.cancelled() => break,
                    () = self.notify.notified() => {}
                }
                continue;
            }

            tokio::select! {
                biased;
                () = shutdown.cancelled() => break,
                Some(result) = running.join_next() => {
                    self.handle_task_result(result);
                }
                () = self.notify.notified() => {}
            }
        }

        running.abort_all();
        while let Some(_result) = running.join_next().await {}
    }

    fn dispatch_pending_tasks(self: &Arc<Self>, running: &mut JoinSet<RepoTaskJoinResult>) {
        loop {
            let target = self.target_parallelism(running.len());
            if running.len() >= target {
                break;
            }

            let Some(task) = self
                .pending
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .pop_front()
            else {
                break;
            };

            self.mark_active(task.repository.id.as_str());
            let coordinator = Arc::clone(self);
            let repo_id = task.repository.id.clone();
            running.spawn(async move {
                match AssertUnwindSafe(coordinator.process_task(task))
                    .catch_unwind()
                    .await
                {
                    Ok(feedback) => RepoTaskJoinResult::Completed(Box::new(feedback)),
                    Err(_) => RepoTaskJoinResult::Panicked { repo_id },
                }
            });
        }
    }

    fn target_parallelism(&self, active_count: usize) -> usize {
        let queued = self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len();
        let mut controller = self
            .concurrency
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        controller.target_limit(queued, active_count)
    }

    fn handle_task_result(&self, result: Result<RepoTaskJoinResult, tokio::task::JoinError>) {
        let feedback = match result {
            Ok(RepoTaskJoinResult::Completed(feedback)) => *feedback,
            Ok(RepoTaskJoinResult::Panicked { repo_id }) => {
                self.concurrency
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .record_failure();
                self.record_failure_status(
                    repo_id.as_str(),
                    &crate::analyzers::RepoIntelligenceError::AnalysisFailed {
                        message: format!(
                            "repo index worker for `{repo_id}` panicked while processing the task"
                        ),
                    },
                    None,
                );
                self.release_repo(repo_id.as_str());
                self.notify.notify_one();
                return;
            }
            Err(error) => {
                let mut controller = self
                    .concurrency
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                controller.record_failure();
                self.refresh_status_snapshot();
                if error.is_panic() {
                    self.notify.notify_one();
                }
                return;
            }
        };

        match feedback.outcome {
            RepoTaskOutcome::Success { revision } => {
                self.record_repo_status(
                    feedback.repo_id.as_str(),
                    RepoIndexPhase::Ready,
                    revision,
                    None,
                );
                self.concurrency
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .record_success(
                        feedback.control_elapsed,
                        self.pending
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .len(),
                    );
            }
            RepoTaskOutcome::Failure { revision, error } => {
                self.record_failure_status(feedback.repo_id.as_str(), &error, revision);
                if should_penalize_adaptive_concurrency(&error) {
                    self.concurrency
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .record_failure();
                }
            }
            RepoTaskOutcome::Requeued { task, error } => {
                self.concurrency
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .record_failure();
                self.release_repo(feedback.repo_id.as_str());
                if !self.enqueue_task(task, true) {
                    self.record_failure_status(feedback.repo_id.as_str(), &error, None);
                }
                self.notify.notify_one();
                return;
            }
            RepoTaskOutcome::Skipped => {}
        }
        self.release_repo(feedback.repo_id.as_str());
        self.notify.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::Duration;

    use super::RepoTaskJoinResult;
    use crate::analyzers::registry::PluginRegistry;
    use crate::repo_index::state::coordinator::RepoIndexCoordinator;
    use crate::repo_index::state::task::{
        RepoIndexTask, RepoIndexTaskPriority, RepoTaskFeedback, RepoTaskOutcome,
    };
    use crate::repo_index::state::tests::remote_repo;
    use crate::repo_index::types::{RepoIndexEntryStatus, RepoIndexPhase};
    use crate::search::SearchPlaneService;

    #[test]
    fn handle_task_result_releases_repo_after_panicked_worker() {
        let coordinator = RepoIndexCoordinator::new(
            PathBuf::from("."),
            std::sync::Arc::new(PluginRegistry::new()),
            SearchPlaneService::new(PathBuf::from(".")),
        );
        coordinator.set_status_for_test(RepoIndexEntryStatus {
            repo_id: "alpha/repo".to_string(),
            phase: RepoIndexPhase::Syncing,
            queue_position: None,
            last_error: None,
            last_revision: None,
            updated_at: None,
            attempt_count: 1,
        });
        coordinator.mark_active_for_test("alpha/repo");

        coordinator.handle_task_result(Ok(RepoTaskJoinResult::Panicked {
            repo_id: "alpha/repo".to_string(),
        }));

        let status = coordinator.status_response(None);
        assert_eq!(status.active_repo_ids, Vec::<String>::new());
        assert_eq!(status.failed, 1);
        assert_eq!(status.repos[0].phase, RepoIndexPhase::Failed);
        assert!(
            status.repos[0]
                .last_error
                .as_deref()
                .is_some_and(|message| message.contains("panicked"))
        );
    }

    #[test]
    fn handle_task_result_uses_control_elapsed_for_adaptive_concurrency() {
        let coordinator = RepoIndexCoordinator::new(
            PathBuf::from("."),
            std::sync::Arc::new(PluginRegistry::new()),
            SearchPlaneService::new(PathBuf::from(".")),
        );
        coordinator.mark_active_for_test("alpha/repo");
        coordinator
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push_back(RepoIndexTask {
                repository: remote_repo("beta/repo", "https://example.com/beta/repo.git"),
                refresh: false,
                fingerprint: "fingerprint-beta".to_string(),
                priority: RepoIndexTaskPriority::Background,
                retry_count: 0,
            });
        {
            let mut controller = coordinator
                .concurrency
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            controller.current_limit = 3;
            controller.max_limit = 6;
            controller.reference_limit = 3;
            controller.ema_elapsed_ms = Some(100.0);
            controller.baseline_elapsed_ms = Some(100.0);
            controller.previous_efficiency = Some(3.0 / 100.0);
        }

        coordinator.handle_task_result(Ok(RepoTaskJoinResult::Completed(Box::new(
            RepoTaskFeedback {
                repo_id: "alpha/repo".to_string(),
                control_elapsed: Duration::from_millis(110),
                outcome: RepoTaskOutcome::Success {
                    revision: Some("rev-1".to_string()),
                },
            },
        ))));

        let controller = coordinator
            .concurrency
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(controller.current_limit, 3);
        assert_eq!(controller.success_streak, 1);
    }

    #[test]
    fn handle_task_result_does_not_penalize_unsupported_repo_failures() {
        let coordinator = RepoIndexCoordinator::new(
            PathBuf::from("."),
            std::sync::Arc::new(PluginRegistry::new()),
            SearchPlaneService::new(PathBuf::from(".")),
        );
        coordinator.mark_active_for_test("alpha/repo");
        coordinator
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push_back(RepoIndexTask {
                repository: remote_repo("beta/repo", "https://example.com/beta/repo.git"),
                refresh: false,
                fingerprint: "fingerprint-beta".to_string(),
                priority: RepoIndexTaskPriority::Background,
                retry_count: 0,
            });
        {
            let mut controller = coordinator
                .concurrency
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            controller.current_limit = 3;
            controller.max_limit = 6;
            controller.reference_limit = 3;
            controller.previous_efficiency = Some(3.0 / 100.0);
        }

        coordinator.handle_task_result(Ok(RepoTaskJoinResult::Completed(Box::new(
            RepoTaskFeedback {
                repo_id: "alpha/repo".to_string(),
                control_elapsed: Duration::from_millis(110),
                outcome: RepoTaskOutcome::Failure {
                    revision: None,
                    error: crate::analyzers::RepoIntelligenceError::UnsupportedRepositoryLayout {
                        repo_id: "alpha/repo".to_string(),
                        message: "missing Project.toml".to_string(),
                    },
                },
            },
        ))));

        let controller = coordinator
            .concurrency
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert_eq!(controller.current_limit, 3);

        let status = coordinator.status_response(Some("alpha/repo"));
        assert_eq!(status.repos[0].phase, RepoIndexPhase::Unsupported);
    }
}
