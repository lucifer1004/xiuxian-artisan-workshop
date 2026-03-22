use std::path::Path;
use std::sync::Arc;

use axum::Json;
use axum::extract::{Query, State};

use crate::gateway::studio::router::{GatewayState, StudioApiError};
use crate::gateway::studio::types::DefinitionResolveResponse;

use super::super::definition::{
    DefinitionMatchMode, DefinitionResolveOptions, resolve_best_definition,
    resolve_definition_candidates,
};
use super::super::observation_hints::definition_observation_hints;
use super::queries::DefinitionResolveQuery;

pub async fn search_definition(
    State(state): State<Arc<GatewayState>>,
    Query(query): Query<DefinitionResolveQuery>,
) -> Result<Json<DefinitionResolveResponse>, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Definition search requires a non-empty query",
        ));
    }

    let source_path = query
        .path
        .as_deref()
        .map(|path| normalize_source_path(state.studio.project_root.as_path(), path));
    let source_paths = source_path
        .as_ref()
        .map(std::slice::from_ref)
        .filter(|paths| !paths.is_empty());
    let observation_hints =
        definition_observation_hints(state.as_ref(), source_paths, query.line, query_text).await;
    let ast_index = state.studio.ast_index().await?;
    let projects = state.studio.configured_projects();
    let options = DefinitionResolveOptions {
        scope_patterns: observation_hints.as_ref().and_then(|hints| {
            (!hints.scope_patterns.is_empty()).then_some(hints.scope_patterns.clone())
        }),
        languages: observation_hints
            .as_ref()
            .and_then(|hints| (!hints.languages.is_empty()).then_some(hints.languages.clone())),
        preferred_source_path: source_path.clone(),
        match_mode: DefinitionMatchMode::ExactOnly,
        include_markdown: false,
        ..DefinitionResolveOptions::default()
    };
    let candidates = resolve_definition_candidates(
        query_text,
        ast_index.as_slice(),
        state.studio.project_root.as_path(),
        state.studio.config_root.as_path(),
        projects.as_slice(),
        &options,
    );
    let Some(definition) = resolve_best_definition(
        query_text,
        ast_index.as_slice(),
        state.studio.project_root.as_path(),
        state.studio.config_root.as_path(),
        projects.as_slice(),
        &options,
    ) else {
        return Err(StudioApiError::not_found("Definition not found"));
    };
    let navigation_target = definition.navigation_target.clone();

    Ok(Json(DefinitionResolveResponse {
        query: query_text.to_string(),
        source_path,
        source_line: query.line,
        candidate_count: candidates.len(),
        selected_scope: "definition".to_string(),
        navigation_target: navigation_target.clone(),
        definition: definition.clone(),
        resolved_target: Some(navigation_target),
        resolved_hit: Some(definition),
    }))
}

fn normalize_source_path(project_root: &Path, path: &str) -> String {
    let path = Path::new(path);
    if path.is_absolute() {
        return path.strip_prefix(project_root).map_or_else(
            |_| path.to_string_lossy().replace('\\', "/"),
            |relative| relative.to_string_lossy().replace('\\', "/"),
        );
    }

    path.to_string_lossy().replace('\\', "/")
}
