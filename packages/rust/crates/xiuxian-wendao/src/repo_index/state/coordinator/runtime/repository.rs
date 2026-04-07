use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::analyzers::errors::RepoIntelligenceError;
use crate::analyzers::query::RepoSyncResult;
use crate::analyzers::{
    RegisteredRepository, RepoSyncMode, RepoSyncQuery, analyze_registered_repository_with_registry,
    repo_sync_for_registered_repository,
};
use crate::repo_index::state::collect::{await_analysis_completion, collect_code_documents};
use crate::repo_index::state::coordinator::RepoIndexCoordinator;
use crate::repo_index::state::task::{repo_index_analysis_timeout, repo_index_sync_timeout};
use crate::repo_index::types::RepoCodeDocument;

impl RepoIndexCoordinator {
    pub(crate) async fn run_repository_analysis(
        &self,
        repository: RegisteredRepository,
    ) -> Result<crate::analyzers::RepositoryAnalysisOutput, RepoIntelligenceError> {
        let repo_id = repository.id.clone();
        let project_root = self.project_root.clone();
        let plugin_registry = Arc::clone(&self.plugin_registry);
        let task = tokio::task::spawn_blocking(move || {
            analyze_registered_repository_with_registry(
                &repository,
                project_root.as_path(),
                plugin_registry.as_ref(),
            )
        });
        await_analysis_completion(repo_id.as_str(), task, repo_index_analysis_timeout()).await
    }

    pub(crate) async fn run_repository_sync(
        &self,
        repo_id: &str,
        repository: RegisteredRepository,
        refresh: bool,
    ) -> Result<RepoSyncResult, RepoIntelligenceError> {
        let repo_id = repo_id.to_string();
        let repo_id_for_worker = repo_id.clone();
        let project_root = self.project_root.clone();
        let mode = if refresh {
            RepoSyncMode::Refresh
        } else {
            RepoSyncMode::Ensure
        };
        let permit = self.acquire_sync_permit(repo_id.as_str()).await?;
        self.bump_status(
            repo_id.as_str(),
            crate::repo_index::types::RepoIndexPhase::Syncing,
            None,
            None,
        );
        let task = tokio::task::spawn_blocking(move || {
            repo_sync_for_registered_repository(
                &RepoSyncQuery {
                    repo_id: repo_id_for_worker,
                    mode,
                },
                &repository,
                project_root.as_path(),
            )
        });
        let result =
            await_repository_sync_completion(repo_id.as_str(), task, repo_index_sync_timeout())
                .await;
        drop(permit);
        result
    }

    pub(crate) async fn collect_code_documents_for_task(
        &self,
        repo_id: &str,
        fingerprint: &str,
        checkout_path: &str,
    ) -> Result<Option<Vec<RepoCodeDocument>>, RepoIntelligenceError> {
        let repo_id = repo_id.to_string();
        let fingerprint = fingerprint.to_string();
        let checkout_path = checkout_path.to_string();
        let fingerprints = Arc::clone(&self.fingerprints);
        let repo_id_for_error = repo_id.clone();
        let repo_id_for_worker = repo_id.clone();
        let task = tokio::task::spawn_blocking(move || {
            Ok::<Option<Vec<RepoCodeDocument>>, RepoIntelligenceError>(collect_code_documents(
                Path::new(checkout_path.as_str()),
                || {
                    let current = fingerprints
                        .read()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .get(&repo_id_for_worker)
                        .cloned();
                    current.as_deref() != Some(fingerprint.as_str())
                },
            ))
        });

        let analysis_timeout = repo_index_analysis_timeout();
        match tokio::time::timeout(analysis_timeout, task).await {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => Err(RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "repo `{repo_id_for_error}` code document worker terminated unexpectedly: {error}"
                ),
            }),
            Err(_) => Err(RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "repo `{repo_id_for_error}` code document collection timed out after {}s while indexing was running",
                    analysis_timeout.as_secs()
                ),
            }),
        }
    }
}

async fn await_repository_sync_completion(
    repo_id: &str,
    mut task: tokio::task::JoinHandle<Result<RepoSyncResult, RepoIntelligenceError>>,
    timeout: Duration,
) -> Result<RepoSyncResult, RepoIntelligenceError> {
    tokio::select! {
        result = &mut task => {
            match result {
                Ok(result) => result,
                Err(error) => Err(RepoIntelligenceError::AnalysisFailed {
                    message: format!(
                        "repo sync worker for `{repo_id}` terminated unexpectedly: {error}"
                    ),
                }),
            }
        }
        () = tokio::time::sleep(timeout) => {
            task.abort();
            Err(RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "repo sync for `{repo_id}` timed out after {}s while waiting for managed source materialization",
                    timeout.as_secs()
                ),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::await_repository_sync_completion;
    use crate::analyzers::RepoIntelligenceError;
    use crate::analyzers::query::{RepoSourceKind, RepoSyncResult};

    #[tokio::test]
    async fn await_repository_sync_completion_returns_without_waiting_for_full_timeout() {
        let task = tokio::task::spawn_blocking(|| {
            Ok::<RepoSyncResult, RepoIntelligenceError>(RepoSyncResult {
                repo_id: "alpha/repo".to_string(),
                source_kind: RepoSourceKind::ManagedRemote,
                ..RepoSyncResult::default()
            })
        });
        let started_at = Instant::now();
        let result =
            await_repository_sync_completion("alpha/repo", task, Duration::from_secs(1)).await;
        let elapsed = started_at.elapsed();

        assert!(result.is_ok(), "fast sync worker should succeed");
        assert!(
            elapsed < Duration::from_millis(250),
            "successful sync completion should not wait on the timeout budget, elapsed={elapsed:?}"
        );
    }

    #[tokio::test]
    async fn await_repository_sync_completion_reports_timeout_for_stuck_worker() {
        let task = tokio::task::spawn_blocking(|| {
            std::thread::sleep(Duration::from_millis(50));
            Ok::<RepoSyncResult, RepoIntelligenceError>(RepoSyncResult {
                repo_id: "alpha/repo".to_string(),
                source_kind: RepoSourceKind::ManagedRemote,
                ..RepoSyncResult::default()
            })
        });

        let error =
            match await_repository_sync_completion("alpha/repo", task, Duration::from_millis(10))
                .await
            {
                Ok(result) => panic!("expected timeout failure, got {result:?}"),
                Err(error) => error,
            };

        match error {
            RepoIntelligenceError::AnalysisFailed { message } => {
                assert!(message.contains("alpha/repo"));
                assert!(message.contains("timed out"));
            }
            other => panic!("expected sync timeout failure, got {other:?}"),
        }
    }
}
