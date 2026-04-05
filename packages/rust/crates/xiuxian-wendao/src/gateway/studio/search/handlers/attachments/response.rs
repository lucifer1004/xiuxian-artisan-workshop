use crate::gateway::studio::router::{StudioApiError, StudioState};
use crate::gateway::studio::search::handlers::queries::AttachmentSearchQuery;
use crate::gateway::studio::types::AttachmentSearchResponse;
use crate::link_graph::LinkGraphAttachmentKind;

pub(crate) async fn load_attachment_search_response_from_studio(
    studio: &StudioState,
    query: AttachmentSearchQuery,
) -> Result<AttachmentSearchResponse, StudioApiError> {
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
    studio.ensure_attachment_index_ready().await?;
    let hits = studio
        .search_attachment_hits(
            query_text,
            limit,
            extensions.as_slice(),
            kinds.as_slice(),
            query.case_sensitive,
        )
        .await?;

    Ok(AttachmentSearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        selected_scope: "attachments".to_string(),
    })
}
