use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
};

use crate::analyzers::{DocsProjectedGapReportQuery, build_docs_projected_gap_report};
use crate::gateway::studio::router::handlers::repo::required_repo_id;
use crate::gateway::studio::router::handlers::repo::shared::with_repo_analysis;
use crate::gateway::studio::router::{GatewayState, StudioApiError};

use super::types::DocsProjectedGapReportApiQuery;

/// Docs projected gap report endpoint.
///
/// # Errors
///
/// Returns an error when `repo` is missing, repository lookup or analysis
/// fails, or the background task panics.
pub async fn projected_gap_report(
    Query(query): Query<DocsProjectedGapReportApiQuery>,
    State(state): State<Arc<GatewayState>>,
) -> Result<Json<crate::analyzers::DocsProjectedGapReportResult>, StudioApiError> {
    let repo_id = required_repo_id(query.repo.as_deref())?;
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id.clone(),
        "DOCS_PROJECTED_GAP_REPORT_PANIC",
        "Docs projected gap report task failed unexpectedly",
        move |analysis| {
            Ok::<_, crate::analyzers::RepoIntelligenceError>(build_docs_projected_gap_report(
                &DocsProjectedGapReportQuery { repo_id },
                &analysis,
            ))
        },
    )
    .await?;
    Ok(Json(result))
}
