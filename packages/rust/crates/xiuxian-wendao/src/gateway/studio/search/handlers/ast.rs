use std::path::Path;
use std::sync::Arc;

use arrow::array::{ArrayRef, Float64Array, StringArray, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema};
use axum::Json;
use axum::extract::{Query, State};
use axum::response::Response;

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::search::handlers::arrow_transport::{
    arrow_payload_response, build_arrow_search_ipc, encode_json,
};
use crate::gateway::studio::types::{AstSearchHit, AstSearchResponse, UiProjectConfig};

use super::super::project_scope::project_metadata_for_path;
use super::queries::AstSearchQuery;

pub async fn search_ast(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<AstSearchQuery>,
) -> Result<Json<AstSearchResponse>, StudioApiError> {
    let response = load_ast_search_response(state.as_ref(), query).await?;
    Ok(Json(response))
}

pub async fn search_ast_hits_arrow(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<AstSearchQuery>,
) -> Result<Response, StudioApiError> {
    let response = load_ast_search_response(state.as_ref(), query).await?;
    ast_hits_arrow_response(&response.hits)
        .map_err(|error| StudioApiError::internal("SEARCH_AST_HITS_ARROW_FAILED", error, None))
}

pub(crate) async fn load_ast_search_response(
    state: &GatewayState,
    query: AstSearchQuery,
) -> Result<AstSearchResponse, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "AST search requires a non-empty query",
        ));
    }

    let limit = query.limit.unwrap_or(20).max(1);
    state.studio.ensure_local_symbol_index_ready().await?;
    let ast_hits = state
        .studio
        .search_local_symbol_hits(query_text, limit)
        .await?;
    let projects = state.studio.configured_projects();
    let mut hits = ast_hits
        .iter()
        .map(|hit| {
            enrich_ast_hit(
                hit,
                state.studio.project_root.as_path(),
                state.studio.config_root.as_path(),
                projects.as_slice(),
            )
        })
        .collect::<Vec<_>>();
    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line_start.cmp(&right.line_start))
    });
    hits.truncate(limit);

    Ok(AstSearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        selected_scope: "definitions".to_string(),
    })
}

pub(crate) fn ast_hits_arrow_response(hits: &[AstSearchHit]) -> Result<Response, String> {
    encode_ast_hits_ipc(hits).map(arrow_payload_response)
}

fn encode_ast_hits_ipc(hits: &[AstSearchHit]) -> Result<Vec<u8>, String> {
    let names: Vec<&str> = hits.iter().map(|hit| hit.name.as_str()).collect();
    let signatures: Vec<&str> = hits.iter().map(|hit| hit.signature.as_str()).collect();
    let paths: Vec<&str> = hits.iter().map(|hit| hit.path.as_str()).collect();
    let languages: Vec<&str> = hits.iter().map(|hit| hit.language.as_str()).collect();
    let crate_names: Vec<&str> = hits.iter().map(|hit| hit.crate_name.as_str()).collect();
    let project_names: Vec<Option<&str>> =
        hits.iter().map(|hit| hit.project_name.as_deref()).collect();
    let root_labels: Vec<Option<&str>> = hits.iter().map(|hit| hit.root_label.as_deref()).collect();
    let node_kinds: Vec<Option<&str>> = hits.iter().map(|hit| hit.node_kind.as_deref()).collect();
    let owner_titles: Vec<Option<&str>> =
        hits.iter().map(|hit| hit.owner_title.as_deref()).collect();
    let navigation_targets_json: Vec<String> = hits
        .iter()
        .map(|hit| encode_json(&hit.navigation_target))
        .collect::<Result<_, _>>()?;

    let schema = Schema::new(vec![
        Field::new("name", DataType::Utf8, false),
        Field::new("signature", DataType::Utf8, false),
        Field::new("path", DataType::Utf8, false),
        Field::new("language", DataType::Utf8, false),
        Field::new("crateName", DataType::Utf8, false),
        Field::new("projectName", DataType::Utf8, true),
        Field::new("rootLabel", DataType::Utf8, true),
        Field::new("nodeKind", DataType::Utf8, true),
        Field::new("ownerTitle", DataType::Utf8, true),
        Field::new("navigationTargetJson", DataType::Utf8, false),
        Field::new("lineStart", DataType::UInt64, false),
        Field::new("lineEnd", DataType::UInt64, false),
        Field::new("score", DataType::Float64, false),
    ]);

    let columns: Vec<ArrayRef> = vec![
        Arc::new(StringArray::from(names)),
        Arc::new(StringArray::from(signatures)),
        Arc::new(StringArray::from(paths)),
        Arc::new(StringArray::from(languages)),
        Arc::new(StringArray::from(crate_names)),
        Arc::new(StringArray::from(project_names)),
        Arc::new(StringArray::from(root_labels)),
        Arc::new(StringArray::from(node_kinds)),
        Arc::new(StringArray::from(owner_titles)),
        Arc::new(StringArray::from(
            navigation_targets_json
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
        )),
        Arc::new(UInt64Array::from(
            hits.iter()
                .map(|hit| hit.line_start as u64)
                .collect::<Vec<_>>(),
        )),
        Arc::new(UInt64Array::from(
            hits.iter()
                .map(|hit| hit.line_end as u64)
                .collect::<Vec<_>>(),
        )),
        Arc::new(Float64Array::from(
            hits.iter().map(|hit| hit.score).collect::<Vec<_>>(),
        )),
    ];
    build_arrow_search_ipc(schema, columns)
}

fn enrich_ast_hit(
    hit: &AstSearchHit,
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) -> AstSearchHit {
    let metadata =
        project_metadata_for_path(project_root, config_root, projects, hit.path.as_str());
    let mut navigation_target = hit.navigation_target.clone();
    navigation_target
        .project_name
        .clone_from(&metadata.project_name);
    navigation_target
        .root_label
        .clone_from(&metadata.root_label);

    let mut enriched = hit.clone();
    enriched.project_name = metadata.project_name;
    enriched.root_label = metadata.root_label;
    enriched.navigation_target = navigation_target;
    if enriched.score <= 0.0 {
        enriched.score = ast_hit_score(&enriched);
    }
    enriched
}

fn ast_hit_score(hit: &AstSearchHit) -> f64 {
    if hit.language != "markdown" {
        return 0.95;
    }

    match hit.node_kind.as_deref() {
        Some("task") => 0.88,
        Some("property" | "observation") => 0.8,
        _ => 0.95,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    use crate::gateway::studio::types::StudioNavigationTarget;
    use arrow::ipc::reader::StreamReader;

    #[test]
    fn search_ast_arrow_roundtrip_preserves_markdown_fields() {
        let hits = vec![AstSearchHit {
            name: "IndexTask".to_string(),
            signature: "- [ ] IndexTask".to_string(),
            path: "docs/index.md".to_string(),
            language: "markdown".to_string(),
            crate_name: "kernel".to_string(),
            project_name: Some("kernel".to_string()),
            root_label: Some("kernel".to_string()),
            node_kind: Some("task".to_string()),
            owner_title: Some("Index".to_string()),
            navigation_target: StudioNavigationTarget {
                path: "kernel/docs/index.md".to_string(),
                category: "knowledge".to_string(),
                project_name: Some("kernel".to_string()),
                root_label: Some("kernel".to_string()),
                line: Some(12),
                line_end: Some(14),
                column: Some(1),
            },
            line_start: 12,
            line_end: 14,
            score: 0.88,
        }];

        let encoded = encode_ast_hits_ipc(&hits).expect("ast hit arrow encoding should succeed");
        let reader = StreamReader::try_new(Cursor::new(encoded), None)
            .expect("ast hit stream reader should open");
        let batches = reader
            .collect::<Result<Vec<_>, _>>()
            .expect("ast hit stream should decode");
        assert_eq!(batches.len(), 1);
        let batch = &batches[0];
        assert_eq!(batch.num_rows(), 1);
        assert!(batch.column_by_name("nodeKind").is_some());
        assert!(batch.column_by_name("ownerTitle").is_some());
        assert!(batch.column_by_name("navigationTargetJson").is_some());
    }
}
