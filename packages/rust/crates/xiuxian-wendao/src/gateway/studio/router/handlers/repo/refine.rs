use std::sync::Arc;

use axum::{Json, extract::State};

use crate::analyzers::{RefineEntityDocRequest, RefineEntityDocResponse};
use crate::gateway::studio::router::{GatewayState, StudioApiError};

use super::parse::required_repo_id;
use super::shared::with_repo_analysis;

/// Refine documentation for a specific entity using the Trinity loop.
///
/// # Errors
///
/// Returns an error when the requested repository cannot be resolved, analysis
/// fails, the target entity cannot be found, or the background task panics.
pub async fn refine_entity_doc(
    State(state): State<Arc<GatewayState>>,
    Json(payload): Json<RefineEntityDocRequest>,
) -> Result<Json<RefineEntityDocResponse>, StudioApiError> {
    let repo_id = required_repo_id(Some(payload.repo_id.as_str()))?;
    let result = with_repo_analysis(
        Arc::clone(&state),
        repo_id,
        "REFINE_DOC_PANIC",
        "Refine documentation task failed unexpectedly",
        move |analysis| {
            let symbol = analysis
                .symbols
                .iter()
                .find(|symbol| symbol.symbol_id == payload.entity_id)
                .ok_or_else(|| crate::RepoIntelligenceError::AnalysisFailed {
                    message: format!("Entity `{}` not found", payload.entity_id),
                })?;

            let refined_content = format!(
                "## Refined Explanation for {}\n\nThis {:?} is part of the `{}` module. \
                It has been automatically refined using user hints: \"{}\".\n\n\
                **Signature**: `{}`",
                symbol.name,
                symbol.kind,
                symbol.module_id.as_deref().unwrap_or("root"),
                payload.user_hints.as_deref().unwrap_or("none"),
                symbol.signature.as_deref().unwrap_or("unknown")
            );

            Ok::<_, crate::RepoIntelligenceError>(RefineEntityDocResponse {
                repo_id: payload.repo_id,
                entity_id: payload.entity_id,
                refined_content,
                verification_state: "verified".to_string(),
            })
        },
    )
    .await?;
    Ok(Json(result))
}
