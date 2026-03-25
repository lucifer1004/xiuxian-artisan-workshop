use std::{path::PathBuf, sync::Arc};

use tokio::sync::OwnedSemaphorePermit;

use crate::analyzers::service::{
    CachedRepositoryAnalysis, analyze_registered_repository_bundle_with_registry,
};
use crate::analyzers::{
    RegisteredRepository, RepoIntelligenceError, RepositoryAnalysisOutput,
    analyze_registered_repository_with_registry,
};
use crate::gateway::studio::router::{
    GatewayState, StudioApiError, configured_repositories, configured_repository,
    map_repo_intelligence_error,
};

pub(super) fn resolve_repository(
    state: &Arc<GatewayState>,
    repo_id: &str,
) -> Result<RegisteredRepository, StudioApiError> {
    configured_repository(&state.studio, repo_id).map_err(map_repo_intelligence_error)
}

fn repository_uses_managed_remote_source(repository: &RegisteredRepository) -> bool {
    repository.url.is_some()
}

async fn acquire_managed_remote_sync_permit(
    state: &Arc<GatewayState>,
    repository: &RegisteredRepository,
) -> Result<Option<OwnedSemaphorePermit>, StudioApiError> {
    if !repository_uses_managed_remote_source(repository) {
        return Ok(None);
    }
    state
        .studio
        .repo_index
        .acquire_sync_permit(repository.id.as_str())
        .await
        .map(Some)
        .map_err(map_repo_intelligence_error)
}

pub(crate) async fn with_repo_analysis<T, F>(
    state: Arc<GatewayState>,
    repo_id: String,
    panic_code: &'static str,
    panic_message: &'static str,
    build: F,
) -> Result<T, StudioApiError>
where
    T: Send + 'static,
    F: FnOnce(RepositoryAnalysisOutput) -> Result<T, RepoIntelligenceError> + Send + 'static,
{
    let cwd = state.studio.project_root.clone();
    let repository = resolve_repository(&state, repo_id.as_str())?;
    let permit = acquire_managed_remote_sync_permit(&state, &repository).await?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let analysis = analyze_registered_repository_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
        build(analysis)
    })
    .await
    .map_err(|error| StudioApiError::internal(panic_code, panic_message, Some(error.to_string())))?
    .map_err(map_repo_intelligence_error)
}

pub(crate) async fn with_repo_cached_analysis_bundle<T, F>(
    state: Arc<GatewayState>,
    repo_id: String,
    panic_code: &'static str,
    panic_message: &'static str,
    build: F,
) -> Result<T, StudioApiError>
where
    T: Send + 'static,
    F: FnOnce(CachedRepositoryAnalysis) -> Result<T, RepoIntelligenceError> + Send + 'static,
{
    let cwd = state.studio.project_root.clone();
    let repository = resolve_repository(&state, repo_id.as_str())?;
    let permit = acquire_managed_remote_sync_permit(&state, &repository).await?;
    let plugin_registry = Arc::clone(&state.studio.plugin_registry);
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let cached = analyze_registered_repository_bundle_with_registry(
            &repository,
            cwd.as_path(),
            &plugin_registry,
        )?;
        build(cached)
    })
    .await
    .map_err(|error| StudioApiError::internal(panic_code, panic_message, Some(error.to_string())))?
    .map_err(map_repo_intelligence_error)
}

pub(super) async fn with_repository<T, F>(
    state: Arc<GatewayState>,
    repo_id: String,
    panic_code: &'static str,
    panic_message: &'static str,
    requires_managed_remote_sync_permit: bool,
    build: F,
) -> Result<T, StudioApiError>
where
    T: Send + 'static,
    F: FnOnce(RegisteredRepository, PathBuf) -> Result<T, RepoIntelligenceError> + Send + 'static,
{
    let cwd = state.studio.project_root.clone();
    let repository = resolve_repository(&state, repo_id.as_str())?;
    let permit = if requires_managed_remote_sync_permit {
        acquire_managed_remote_sync_permit(&state, &repository).await?
    } else {
        None
    };
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        build(repository, cwd)
    })
    .await
    .map_err(|error| StudioApiError::internal(panic_code, panic_message, Some(error.to_string())))?
    .map_err(map_repo_intelligence_error)
}

pub(super) fn repo_index_repositories(
    state: &Arc<GatewayState>,
    repo: Option<&str>,
) -> Result<Vec<RegisteredRepository>, StudioApiError> {
    if let Some(repo_id) = repo {
        return Ok(vec![resolve_repository(state, repo_id)?]);
    }
    Ok(configured_repositories(&state.studio))
}
