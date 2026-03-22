use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;
use walkdir::WalkDir;

use crate::analyzers::RepoIntelligenceError;
use crate::gateway::studio::repo_index::types::RepoCodeDocument;

use super::language::{infer_code_language, is_excluded_code_path, is_supported_code_path};

pub(super) async fn await_analysis_completion(
    repo_id: &str,
    task: JoinHandle<Result<crate::analyzers::RepositoryAnalysisOutput, RepoIntelligenceError>>,
    timeout: Duration,
) -> Result<crate::analyzers::RepositoryAnalysisOutput, RepoIntelligenceError> {
    match tokio::time::timeout(timeout, task).await {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => Err(RepoIntelligenceError::AnalysisFailed {
            message: format!("repo `{repo_id}` indexing worker terminated unexpectedly: {error}"),
        }),
        Err(_) => Err(RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "repo `{repo_id}` indexing timed out after {}s while analysis was running",
                timeout.as_secs()
            ),
        }),
    }
}

pub(super) fn collect_code_documents(
    root: &Path,
    mut is_cancelled: impl FnMut() -> bool,
) -> Option<Vec<RepoCodeDocument>> {
    let mut documents = Vec::new();
    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        if is_cancelled() {
            return None;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let relative_path = entry.path().strip_prefix(root).ok().map_or_else(
            || entry.path().to_string_lossy().replace('\\', "/"),
            |path| path.to_string_lossy().replace('\\', "/"),
        );
        if is_excluded_code_path(relative_path.as_str())
            || !is_supported_code_path(relative_path.as_str())
        {
            continue;
        }
        let Ok(contents) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        documents.push(RepoCodeDocument {
            language: infer_code_language(relative_path.as_str()),
            path: relative_path,
            contents: Arc::<str>::from(contents),
        });
    }
    Some(documents)
}
