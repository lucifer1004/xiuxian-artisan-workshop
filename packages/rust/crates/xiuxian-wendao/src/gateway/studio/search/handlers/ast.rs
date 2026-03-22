use std::path::Path;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::{AstSearchHit, AstSearchResponse, UiProjectConfig};

use super::super::project_scope::project_metadata_for_path;
use super::queries::AstSearchQuery;

pub async fn search_ast(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<AstSearchQuery>,
) -> Result<Json<AstSearchResponse>, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "AST search requires a non-empty query",
        ));
    }

    let limit = query.limit.unwrap_or(20).max(1);
    let ast_index = state.studio.ast_index().await?;
    let projects = state.studio.configured_projects();
    let mut hits = ast_index
        .iter()
        .filter(|hit| ast_hit_matches(hit, query_text))
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

    Ok(Json(AstSearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        selected_scope: "definitions".to_string(),
    }))
}

fn ast_hit_matches(hit: &AstSearchHit, query: &str) -> bool {
    let query = query.to_ascii_lowercase();
    hit.name.to_ascii_lowercase().contains(query.as_str())
        || hit.signature.to_ascii_lowercase().contains(query.as_str())
        || hit
            .owner_title
            .as_deref()
            .is_some_and(|owner| owner.to_ascii_lowercase().contains(query.as_str()))
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
    enriched.score = ast_hit_score(&enriched);
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
