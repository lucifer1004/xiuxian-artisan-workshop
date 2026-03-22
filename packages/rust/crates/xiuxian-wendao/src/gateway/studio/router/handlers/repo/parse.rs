use crate::analyzers::{ProjectionPageKind, RepoSyncMode};
use crate::gateway::studio::router::StudioApiError;

pub(super) fn required_repo_id(repo: Option<&str>) -> Result<String, StudioApiError> {
    repo.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| StudioApiError::bad_request("MISSING_REPO", "`repo` is required"))
}

pub(super) fn required_search_query(query: Option<&str>) -> Result<String, StudioApiError> {
    query
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| StudioApiError::bad_request("MISSING_QUERY", "`query` is required"))
}

pub(super) fn required_page_id(page_id: Option<&str>) -> Result<String, StudioApiError> {
    page_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| StudioApiError::bad_request("MISSING_PAGE_ID", "`page_id` is required"))
}

pub(super) fn required_node_id(node_id: Option<&str>) -> Result<String, StudioApiError> {
    node_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| StudioApiError::bad_request("MISSING_NODE_ID", "`node_id` is required"))
}

pub(super) fn parse_repo_sync_mode(mode: Option<&str>) -> Result<RepoSyncMode, StudioApiError> {
    match mode
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("ensure")
    {
        "ensure" => Ok(RepoSyncMode::Ensure),
        "refresh" => Ok(RepoSyncMode::Refresh),
        "status" => Ok(RepoSyncMode::Status),
        other => Err(StudioApiError::bad_request(
            "INVALID_MODE",
            format!("unsupported repo sync mode `{other}`"),
        )),
    }
}

pub(super) fn parse_projection_page_kind(
    kind: Option<&str>,
) -> Result<Option<ProjectionPageKind>, StudioApiError> {
    match kind.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(None),
        Some("reference") => Ok(Some(ProjectionPageKind::Reference)),
        Some("how_to") => Ok(Some(ProjectionPageKind::HowTo)),
        Some("tutorial") => Ok(Some(ProjectionPageKind::Tutorial)),
        Some("explanation") => Ok(Some(ProjectionPageKind::Explanation)),
        Some(other) => Err(StudioApiError::bad_request(
            "INVALID_KIND",
            format!("unsupported projected page kind `{other}`"),
        )),
    }
}

pub(super) fn required_projection_page_kind(
    kind: Option<&str>,
) -> Result<ProjectionPageKind, StudioApiError> {
    parse_projection_page_kind(kind)?
        .ok_or_else(|| StudioApiError::bad_request("MISSING_KIND", "`kind` is required"))
}
