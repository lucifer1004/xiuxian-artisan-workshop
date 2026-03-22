use std::path::Path;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};

use crate::gateway::studio::router::{GatewayState, StudioApiError, StudioState};
use crate::gateway::studio::types::{
    SearchHit, SearchResponse, StudioNavigationTarget, UiProjectConfig,
};
use crate::link_graph::{LinkGraphHit, LinkGraphSearchOptions};

use super::super::project_scope::{
    SearchProjectMetadata, project_metadata_for_path, resolve_project_root_path,
};
use super::code_search::build_code_search_response;
use super::queries::SearchQuery;

pub async fn search_knowledge(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Knowledge search requires a non-empty query",
        ));
    }

    let limit = query.limit.unwrap_or(10).max(1);
    let response = build_knowledge_search_response(
        state.studio.as_ref(),
        query_text,
        limit,
        query
            .intent
            .clone()
            .or_else(|| Some("semantic_lookup".to_string())),
    )
    .await?;
    Ok(Json(response))
}

pub async fn search_intent(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<SearchQuery>,
) -> Result<Json<SearchResponse>, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    let intent = query.intent.clone().unwrap_or_default();
    let limit = query.limit.unwrap_or(10).max(1);

    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Intent search requires a non-empty query",
        ));
    }

    if intent == "code_search" {
        let response = build_code_search_response(
            state.studio.as_ref(),
            raw_query,
            query.repo.as_deref(),
            limit,
        )?;
        return Ok(Json(response));
    }

    let response = build_knowledge_search_response(
        state.studio.as_ref(),
        query_text,
        limit,
        (!intent.is_empty()).then_some(intent),
    )
    .await?;
    Ok(Json(response))
}

async fn build_knowledge_search_response(
    studio: &StudioState,
    query_text: &str,
    limit: usize,
    intent: Option<String>,
) -> Result<SearchResponse, StudioApiError> {
    let graph_index = studio.graph_index().await?;
    let projects = studio.configured_projects();
    let hits = graph_index
        .execute_search(query_text, limit, &LinkGraphSearchOptions::default())
        .into_iter()
        .map(|hit| {
            knowledge_graph_hit_to_search_hit(
                studio.project_root.as_path(),
                studio.config_root.as_path(),
                projects.as_slice(),
                hit,
            )
        })
        .collect::<Vec<_>>();

    let selected_mode = if hits.is_empty() {
        "vector_only".to_string()
    } else {
        "graph_fts".to_string()
    };
    let graph_confidence_score = if hits.is_empty() { 0.0 } else { 1.0 };

    Ok(SearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        graph_confidence_score: Some(graph_confidence_score),
        selected_mode: Some(selected_mode.clone()),
        intent,
        intent_confidence: Some(graph_confidence_score),
        search_mode: Some(selected_mode),
        partial: false,
        indexing_state: None,
        pending_repos: Vec::new(),
        skipped_repos: Vec::new(),
    })
}

fn knowledge_graph_hit_to_search_hit(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    hit: LinkGraphHit,
) -> SearchHit {
    let metadata =
        project_metadata_for_path(project_root, config_root, projects, hit.path.as_str());
    let display_path = studio_display_path(
        project_root,
        config_root,
        projects,
        &metadata,
        hit.path.as_str(),
    );
    let hierarchy = Some(
        display_path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>(),
    );

    SearchHit {
        stem: hit.stem.clone(),
        title: (!hit.title.trim().is_empty()).then_some(hit.title.clone()),
        path: display_path,
        doc_type: hit.doc_type.clone(),
        tags: hit.tags.clone(),
        score: hit.score,
        best_section: hit.best_section.clone(),
        match_reason: hit
            .match_reason
            .clone()
            .or_else(|| Some("link_graph_search".to_string())),
        hierarchical_uri: None,
        hierarchy,
        saliency_score: None,
        audit_status: None,
        verification_state: None,
        implicit_backlinks: None,
        implicit_backlink_items: None,
        navigation_target: Some(StudioNavigationTarget {
            path: hit.path,
            category: "doc".to_string(),
            project_name: metadata.project_name,
            root_label: metadata.root_label,
            line: None,
            line_end: None,
            column: None,
        }),
    }
}

fn studio_display_path(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    metadata: &SearchProjectMetadata,
    path: &str,
) -> String {
    let normalized = path.replace('\\', "/");
    if projects.len() > 1
        && let Some(project_name) = metadata.project_name.as_deref()
    {
        let relative_to_project = projects
            .iter()
            .find(|project| project.name == project_name)
            .and_then(|project| resolve_project_root_path(config_root, project.root.as_str()))
            .and_then(|project_root_path| {
                let absolute_path = if Path::new(path).is_absolute() {
                    Path::new(path).to_path_buf()
                } else {
                    project_root.join(path)
                };
                absolute_path
                    .strip_prefix(project_root_path)
                    .ok()
                    .map(|relative| relative.to_string_lossy().replace('\\', "/"))
            })
            .filter(|relative| !relative.is_empty())
            .unwrap_or_else(|| normalized.clone());

        if !relative_to_project.starts_with(&format!("{project_name}/")) {
            return format!("{project_name}/{relative_to_project}");
        }
    }

    normalized
}
