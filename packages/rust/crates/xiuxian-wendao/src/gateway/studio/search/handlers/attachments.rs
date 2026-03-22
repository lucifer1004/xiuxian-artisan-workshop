use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::{
    AttachmentSearchHit, AttachmentSearchResponse, StudioNavigationTarget, UiProjectConfig,
};
use crate::link_graph::LinkGraphAttachmentKind;

use super::super::project_scope::project_metadata_for_path;
use super::queries::AttachmentSearchQuery;

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
    let projects = state.studio.configured_projects();
    let graph_index = state.link_graph_index().await?;
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
    let hits = graph_index
        .search_attachments(
            query_text,
            limit,
            extensions.as_slice(),
            kinds.as_slice(),
            query.case_sensitive,
        )
        .into_iter()
        .map(|hit| {
            attachment_search_hit(
                state.studio.project_root.as_path(),
                state.studio.config_root.as_path(),
                projects.as_slice(),
                hit,
            )
        })
        .collect::<Vec<_>>();

    Ok(Json(AttachmentSearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        selected_scope: "attachments".to_string(),
    }))
}

fn attachment_search_hit(
    project_root: &std::path::Path,
    config_root: &std::path::Path,
    projects: &[UiProjectConfig],
    hit: crate::link_graph::LinkGraphAttachmentHit,
) -> AttachmentSearchHit {
    let metadata = project_metadata_for_path(
        project_root,
        config_root,
        projects,
        hit.source_path.as_str(),
    );

    AttachmentSearchHit {
        name: hit.attachment_name.clone(),
        path: hit.source_path.clone(),
        navigation_target: StudioNavigationTarget {
            path: hit.source_path.clone(),
            category: "doc".to_string(),
            project_name: metadata.project_name,
            root_label: metadata.root_label,
            line: None,
            line_end: None,
            column: None,
        },
        score: hit.score,
        source_id: hit.source_id.clone(),
        source_stem: hit.source_stem,
        source_title: hit.source_title,
        source_path: hit.source_path,
        attachment_id: format!("att://{}/{}", hit.source_id, hit.attachment_path),
        attachment_path: hit.attachment_path,
        attachment_name: hit.attachment_name,
        attachment_ext: hit.attachment_ext,
        kind: attachment_kind_label(hit.kind).to_string(),
        vision_snippet: hit.vision_snippet,
    }
}

fn attachment_kind_label(kind: LinkGraphAttachmentKind) -> &'static str {
    match kind {
        LinkGraphAttachmentKind::Image => "image",
        LinkGraphAttachmentKind::Pdf => "pdf",
        LinkGraphAttachmentKind::Gpg => "gpg",
        LinkGraphAttachmentKind::Document => "document",
        LinkGraphAttachmentKind::Archive => "archive",
        LinkGraphAttachmentKind::Audio => "audio",
        LinkGraphAttachmentKind::Video => "video",
        LinkGraphAttachmentKind::Other => "other",
    }
}
