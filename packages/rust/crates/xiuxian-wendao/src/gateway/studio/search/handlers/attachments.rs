use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};

use super::queries::AttachmentSearchQuery;
use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::AttachmentSearchResponse;
use crate::link_graph::LinkGraphAttachmentKind;

pub async fn search_attachments(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<AttachmentSearchQuery>,
) -> Result<Json<AttachmentSearchResponse>, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Attachment search requires a non-empty query",
        ));
    }

    let limit = query.limit.unwrap_or(20).max(1);
    let extensions = query
        .ext
        .iter()
        .map(|value| value.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let kinds = query
        .kind
        .iter()
        .map(|value| LinkGraphAttachmentKind::from_alias(value))
        .collect::<Vec<_>>();
    state.studio.ensure_attachment_index_ready().await?;
    let hits = state
        .studio
        .search_attachment_hits(
            query_text,
            limit,
            extensions.as_slice(),
            kinds.as_slice(),
            query.case_sensitive,
        )
        .await?;

    Ok(Json(AttachmentSearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        selected_scope: "attachments".to_string(),
    }))
}
