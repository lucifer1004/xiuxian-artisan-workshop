use std::sync::Arc;

use arrow::array::{ArrayRef, Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use axum::Json;
use axum::extract::{Query, State};
use axum::response::Response;

use super::queries::AttachmentSearchQuery;
use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::search::handlers::arrow_transport::{
    arrow_payload_response, build_arrow_search_ipc, encode_optional_json,
};
use crate::gateway::studio::types::{AttachmentSearchHit, AttachmentSearchResponse};
use crate::link_graph::LinkGraphAttachmentKind;

pub async fn search_attachments(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<AttachmentSearchQuery>,
) -> Result<Json<AttachmentSearchResponse>, StudioApiError> {
    let response = load_attachment_search_response(state.as_ref(), query).await?;
    Ok(Json(response))
}

pub async fn search_attachments_hits_arrow(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<AttachmentSearchQuery>,
) -> Result<Response, StudioApiError> {
    let response = load_attachment_search_response(state.as_ref(), query).await?;
    attachment_hits_arrow_response(&response.hits).map_err(|error| {
        StudioApiError::internal("SEARCH_ATTACHMENT_HITS_ARROW_FAILED", error, None)
    })
}

pub(crate) async fn load_attachment_search_response(
    state: &GatewayState,
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

    Ok(AttachmentSearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        selected_scope: "attachments".to_string(),
    })
}

pub(crate) fn attachment_hits_arrow_response(
    hits: &[AttachmentSearchHit],
) -> Result<Response, String> {
    encode_attachment_hits_ipc(hits).map(arrow_payload_response)
}

fn encode_attachment_hits_ipc(hits: &[AttachmentSearchHit]) -> Result<Vec<u8>, String> {
    let names: Vec<&str> = hits.iter().map(|hit| hit.name.as_str()).collect();
    let paths: Vec<&str> = hits.iter().map(|hit| hit.path.as_str()).collect();
    let source_ids: Vec<&str> = hits.iter().map(|hit| hit.source_id.as_str()).collect();
    let source_stems: Vec<&str> = hits.iter().map(|hit| hit.source_stem.as_str()).collect();
    let source_titles: Vec<&str> = hits.iter().map(|hit| hit.source_title.as_str()).collect();
    let navigation_targets_json: Vec<Option<String>> = hits
        .iter()
        .map(|hit| encode_optional_json(Some(&hit.navigation_target)))
        .collect::<Result<_, _>>()?;
    let source_paths: Vec<&str> = hits.iter().map(|hit| hit.source_path.as_str()).collect();
    let attachment_ids: Vec<&str> = hits.iter().map(|hit| hit.attachment_id.as_str()).collect();
    let attachment_paths: Vec<&str> = hits
        .iter()
        .map(|hit| hit.attachment_path.as_str())
        .collect();
    let attachment_names: Vec<&str> = hits
        .iter()
        .map(|hit| hit.attachment_name.as_str())
        .collect();
    let attachment_exts: Vec<&str> = hits.iter().map(|hit| hit.attachment_ext.as_str()).collect();
    let kinds: Vec<&str> = hits.iter().map(|hit| hit.kind.as_ref()).collect();
    let vision_snippets: Vec<Option<&str>> = hits
        .iter()
        .map(|hit| hit.vision_snippet.as_deref())
        .collect();

    let schema = Schema::new(vec![
        Field::new("name", DataType::Utf8, false),
        Field::new("path", DataType::Utf8, false),
        Field::new("sourceId", DataType::Utf8, false),
        Field::new("sourceStem", DataType::Utf8, false),
        Field::new("sourceTitle", DataType::Utf8, false),
        Field::new("navigationTargetJson", DataType::Utf8, true),
        Field::new("sourcePath", DataType::Utf8, false),
        Field::new("attachmentId", DataType::Utf8, false),
        Field::new("attachmentPath", DataType::Utf8, false),
        Field::new("attachmentName", DataType::Utf8, false),
        Field::new("attachmentExt", DataType::Utf8, false),
        Field::new("kind", DataType::Utf8, false),
        Field::new("score", DataType::Float64, false),
        Field::new("visionSnippet", DataType::Utf8, true),
    ]);

    let columns: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(names)),
        Arc::new(StringArray::from(paths)),
        Arc::new(StringArray::from(source_ids)),
        Arc::new(StringArray::from(source_stems)),
        Arc::new(StringArray::from(source_titles)),
        Arc::new(StringArray::from(navigation_targets_json)),
        Arc::new(StringArray::from(source_paths)),
        Arc::new(StringArray::from(attachment_ids)),
        Arc::new(StringArray::from(attachment_paths)),
        Arc::new(StringArray::from(attachment_names)),
        Arc::new(StringArray::from(attachment_exts)),
        Arc::new(StringArray::from(kinds)),
        Arc::new(Float64Array::from(
            hits.iter().map(|hit| hit.score).collect::<Vec<_>>(),
        )),
        Arc::new(StringArray::from(vision_snippets)),
    ];
    build_arrow_search_ipc(schema, columns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    use crate::gateway::studio::types::StudioNavigationTarget;
    use arrow::ipc::reader::StreamReader;

    #[test]
    fn search_attachment_arrow_roundtrip_preserves_navigation_and_snippet() {
        let hits = vec![AttachmentSearchHit {
            name: "topology.png".to_string(),
            path: "kernel/docs/attachments/topology-owner.md".to_string(),
            source_id: "note:topology-owner".to_string(),
            source_stem: "topology-owner".to_string(),
            source_title: "Topology Owner".to_string(),
            navigation_target: StudioNavigationTarget {
                path: "kernel/docs/attachments/topology-owner.md".to_string(),
                category: "knowledge".to_string(),
                project_name: Some("kernel".to_string()),
                root_label: Some("kernel".to_string()),
                line: Some(8),
                line_end: Some(12),
                column: Some(1),
            },
            source_path: "kernel/docs/attachments/topology-owner.md".to_string(),
            attachment_id: "attachment:topology-owner:diagram".to_string(),
            attachment_path: "kernel/docs/assets/topology.png".to_string(),
            attachment_name: "topology.png".to_string(),
            attachment_ext: "png".to_string(),
            kind: "image".to_string(),
            score: 0.91,
            vision_snippet: Some("A topology diagram".to_string()),
        }];

        let encoded = encode_attachment_hits_ipc(&hits)
            .expect("attachment hit arrow encoding should succeed");
        let reader = StreamReader::try_new(Cursor::new(encoded), None)
            .expect("attachment hit stream reader should open");
        let batches = reader
            .collect::<Result<Vec<_>, _>>()
            .expect("attachment hit stream should decode");
        assert_eq!(batches.len(), 1);
        let batch = &batches[0];
        assert_eq!(batch.num_rows(), 1);
        assert!(batch.column_by_name("navigationTargetJson").is_some());
        assert!(batch.column_by_name("visionSnippet").is_some());
    }
}
