use std::path::Path;
use std::sync::Arc;

use crate::gateway::studio::router::StudioApiError;
use crate::gateway::studio::search::handlers::code_search::query::{
    RepoSearchResultLimits, parse_repo_code_search_query,
};
use crate::gateway::studio::search::handlers::code_search::types::RepoSearchTarget;
use crate::gateway::studio::types::{SearchHit, StudioNavigationTarget};
use crate::query_core::{
    InMemoryWendaoExplainSink, RetrievalCorpus, query_repo_code_relation,
    query_repo_content_relation, query_repo_entity_relation,
};
use crate::search_plane::{
    SearchCorpusKind, SearchPlaneService, SearchQueryTelemetry, SearchQueryTelemetrySource,
};
use chrono::Utc;

/// Search repo entity rows for a repo-scoped code query.
///
/// # Errors
///
/// Returns [`StudioApiError`] when the repo entity search plane fails.
pub(crate) async fn search_repo_entity_hits(
    search_plane: &SearchPlaneService,
    repo_id: &str,
    raw_query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, StudioApiError> {
    let parsed = parse_repo_code_search_query(raw_query);
    let Some(search_term) = parsed.search_term() else {
        return Ok(Vec::new());
    };
    let explain_sink = Arc::new(InMemoryWendaoExplainSink::new());
    let relation = query_repo_entity_relation(
        search_plane,
        repo_id,
        search_term,
        &parsed.language_filters,
        &parsed.kind_filters,
        limit,
        Some(explain_sink.clone()),
    )
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_ENTITY_SEARCH_FAILED",
            "Failed to query repo entity search plane",
            Some(error.to_string()),
        )
    })?;
    record_query_core_telemetry(
        search_plane,
        SearchCorpusKind::RepoEntity,
        repo_id,
        limit,
        explain_sink.events().as_slice(),
    );

    query_relation_to_search_hits(repo_id, &relation).map_err(|error| {
        StudioApiError::internal(
            "REPO_ENTITY_SEARCH_DECODE_FAILED",
            "Failed to decode repo entity query-core relation",
            Some(error.to_string()),
        )
    })
}

/// Search repo content rows for a repo-scoped code query.
///
/// # Errors
///
/// Returns [`StudioApiError`] when the repo content search plane fails.
pub(crate) async fn search_repo_content_hits(
    search_plane: &SearchPlaneService,
    repo_id: &str,
    raw_query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, StudioApiError> {
    let parsed = parse_repo_code_search_query(raw_query);
    let Some(search_term) = parsed.search_term() else {
        return Ok(Vec::new());
    };
    if !parsed.kind_filters.is_empty() && !parsed.kind_filters.contains("file") {
        return Ok(Vec::new());
    }
    let explain_sink = Arc::new(InMemoryWendaoExplainSink::new());
    let relation = query_repo_content_relation(
        search_plane,
        repo_id,
        search_term,
        &parsed.language_filters,
        limit,
        Some(explain_sink.clone()),
    )
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_CONTENT_SEARCH_FAILED",
            "Failed to query repo content search plane",
            Some(error.to_string()),
        )
    })?;
    record_query_core_telemetry(
        search_plane,
        SearchCorpusKind::RepoContentChunk,
        repo_id,
        limit,
        explain_sink.events().as_slice(),
    );

    query_relation_to_search_hits(repo_id, &relation).map_err(|error| {
        StudioApiError::internal(
            "REPO_CONTENT_SEARCH_DECODE_FAILED",
            "Failed to decode repo content query-core relation",
            Some(error.to_string()),
        )
    })
}

/// Search one repo through the entity-first, content-fallback code-search policy.
///
/// # Errors
///
/// Returns [`StudioApiError`] when one of the repo-scoped search lanes fails.
pub(crate) async fn search_repo_code_hits(
    search_plane: &SearchPlaneService,
    target: &RepoSearchTarget,
    raw_query: &str,
    per_repo_limits: RepoSearchResultLimits,
) -> Result<Vec<SearchHit>, StudioApiError> {
    let parsed = parse_repo_code_search_query(raw_query);
    let Some(search_term) = parsed.search_term() else {
        return Ok(Vec::new());
    };
    let explain_sink = Arc::new(InMemoryWendaoExplainSink::new());
    let query_limit = if target.publication_state.entity_published {
        per_repo_limits.entity_limit
    } else {
        per_repo_limits.content_limit
    };
    let result = query_repo_code_relation(
        search_plane,
        target.repo_id.as_str(),
        search_term,
        &parsed.language_filters,
        &parsed.kind_filters,
        target.publication_state.entity_published,
        target.publication_state.content_published,
        query_limit,
        Some(explain_sink.clone()),
    )
    .await
    .map_err(|error| {
        StudioApiError::internal(
            "REPO_CODE_SEARCH_FAILED",
            "Failed to query repo code search through query core",
            Some(error.to_string()),
        )
    })?;

    let corpus = match result.corpus {
        RetrievalCorpus::RepoEntity => SearchCorpusKind::RepoEntity,
        RetrievalCorpus::RepoContent => SearchCorpusKind::RepoContentChunk,
    };
    let telemetry_limit = match result.corpus {
        RetrievalCorpus::RepoEntity => per_repo_limits.entity_limit,
        RetrievalCorpus::RepoContent => per_repo_limits.content_limit,
    };
    record_query_core_telemetry(
        search_plane,
        corpus,
        target.repo_id.as_str(),
        telemetry_limit,
        explain_sink.events().as_slice(),
    );

    let mut repository_hits =
        query_relation_to_search_hits(target.repo_id.as_str(), &result.relation).map_err(
            |error| {
                StudioApiError::internal(
                    "REPO_CODE_SEARCH_DECODE_FAILED",
                    "Failed to decode repo code query-core relation",
                    Some(error.to_string()),
                )
            },
        )?;

    if result.corpus == RetrievalCorpus::RepoContent
        && repository_hits.len() > per_repo_limits.content_limit
    {
        repository_hits.truncate(per_repo_limits.content_limit);
    }

    Ok(repository_hits)
}

#[cfg(test)]
use crate::gateway::studio::router::StudioState;

#[cfg(test)]
/// Build repo entity search hits through the Studio state wrapper.
pub(crate) async fn build_repo_entity_search_hits(
    studio: &StudioState,
    repo_id: &str,
    raw_query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, StudioApiError> {
    search_repo_entity_hits(&studio.search_plane, repo_id, raw_query, limit).await
}

#[cfg(test)]
/// Build repo content search hits through the Studio state wrapper.
pub(crate) async fn build_repo_content_search_hits(
    studio: &StudioState,
    repo_id: &str,
    raw_query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, StudioApiError> {
    search_repo_content_hits(&studio.search_plane, repo_id, raw_query, limit).await
}

fn query_relation_to_search_hits(
    repo_id: &str,
    relation: &crate::query_core::WendaoRelation,
) -> Result<Vec<SearchHit>, xiuxian_vector::VectorStoreError> {
    let mut hits = Vec::new();
    for batch in relation.batches() {
        let rows = xiuxian_vector::retrieval_rows_from_record_batch(batch)?;
        hits.extend(
            rows.into_iter()
                .map(|row| retrieval_row_to_search_hit(repo_id, row)),
        );
    }
    Ok(hits)
}

fn retrieval_row_to_search_hit(repo_id: &str, row: xiuxian_vector::RetrievalRow) -> SearchHit {
    let doc_type = row.doc_type.clone().or_else(|| Some("file".to_string()));
    let kind_tag = doc_type.clone().unwrap_or_else(|| "unknown".to_string());
    let mut tags = vec![
        repo_id.to_string(),
        "code".to_string(),
        kind_tag.clone(),
        format!("kind:{kind_tag}"),
    ];
    if let Some(language) = row
        .language
        .clone()
        .or_else(|| infer_code_language(row.path.as_str()))
    {
        tags.push(language.clone());
        tags.push(format!("lang:{language}"));
    }
    let stem = if row.id.is_empty() {
        Path::new(row.path.as_str())
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(row.path.as_str())
            .to_string()
    } else {
        row.id.clone()
    };

    SearchHit {
        stem,
        title: row.title.clone().or_else(|| Some(row.path.clone())),
        path: row.path.clone(),
        doc_type,
        tags,
        score: row.score.unwrap_or_default(),
        best_section: row.best_section.clone().or(row.snippet.clone()),
        match_reason: row
            .match_reason
            .clone()
            .or_else(|| Some(row.source.clone())),
        hierarchical_uri: None,
        hierarchy: Some(row.path.split('/').map(str::to_string).collect::<Vec<_>>()),
        saliency_score: None,
        audit_status: None,
        verification_state: None,
        implicit_backlinks: None,
        implicit_backlink_items: None,
        navigation_target: row.line.map(|line| StudioNavigationTarget {
            path: format!("{repo_id}/{}", row.path),
            category: "repo_code".to_string(),
            project_name: Some(repo_id.to_string()),
            root_label: Some(repo_id.to_string()),
            line: usize::try_from(line).ok(),
            line_end: usize::try_from(line).ok(),
            column: None,
        }),
    }
}

fn infer_code_language(path: &str) -> Option<String> {
    match Path::new(path).extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("jl") || ext.eq_ignore_ascii_case("julia") => {
            Some("julia".to_string())
        }
        Some(ext) if ext.eq_ignore_ascii_case("mo") || ext.eq_ignore_ascii_case("modelica") => {
            Some("modelica".to_string())
        }
        Some(ext) if ext.eq_ignore_ascii_case("rs") => Some("rust".to_string()),
        Some(ext) if ext.eq_ignore_ascii_case("py") => Some("python".to_string()),
        Some(ext) if ext.eq_ignore_ascii_case("ts") || ext.eq_ignore_ascii_case("tsx") => {
            Some("typescript".to_string())
        }
        _ => None,
    }
}

fn record_query_core_telemetry(
    search_plane: &SearchPlaneService,
    corpus: SearchCorpusKind,
    repo_id: &str,
    limit: usize,
    events: &[crate::query_core::WendaoExplainEvent],
) {
    let Some(event) = events
        .iter()
        .rev()
        .find(|event| event.operator_kind == crate::query_core::WendaoOperatorKind::VectorSearch)
    else {
        return;
    };

    let result_count =
        u64::try_from(event.output_row_count.unwrap_or_default()).unwrap_or(u64::MAX);
    let rows_scanned = u64::try_from(
        event
            .input_row_count
            .unwrap_or(event.output_row_count.unwrap_or_default()),
    )
    .unwrap_or(u64::MAX);
    let matched_rows = result_count;
    let working_set_budget_rows = u64::try_from(limit.max(1)).unwrap_or(u64::MAX);

    search_plane.record_query_telemetry(
        corpus,
        SearchQueryTelemetry {
            captured_at: Utc::now().to_rfc3339(),
            scope: Some(repo_id.to_string()),
            source: SearchQueryTelemetrySource::Scan,
            batch_count: 1,
            rows_scanned,
            matched_rows,
            result_count,
            batch_row_limit: None,
            recall_limit_rows: Some(u64::try_from(limit).unwrap_or(u64::MAX)),
            working_set_budget_rows,
            trim_threshold_rows: working_set_budget_rows,
            peak_working_set_rows: matched_rows,
            trim_count: 0,
            dropped_candidate_count: 0,
        },
    );
}
