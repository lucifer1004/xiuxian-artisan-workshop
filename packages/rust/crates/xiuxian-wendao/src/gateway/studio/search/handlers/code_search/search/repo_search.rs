use std::path::Path;
use std::sync::Arc;

use xiuxian_vector::{LanceInt32Array, LanceListArray, LanceRecordBatch, LanceStringArray};

use crate::gateway::studio::router::StudioApiError;
#[cfg(test)]
use crate::gateway::studio::search::handlers::code_search::query::RepoSearchResultLimits;
use crate::gateway::studio::search::handlers::code_search::query::parse_repo_code_search_query;
#[cfg(test)]
use crate::gateway::studio::search::handlers::code_search::types::RepoSearchTarget;
use crate::gateway::studio::types::{SearchHit, StudioNavigationTarget};
use crate::link_graph::plugin_runtime::SearchPlaneRepoSearchFlightRouteProvider;
use crate::query_core::{
    InMemoryWendaoExplainSink, RepoRetrievalQuery, query_repo_entity_relation,
};
#[cfg(test)]
use crate::query_core::{RepoCodeQueryRequest, RetrievalCorpus, query_repo_code_relation};
use crate::search_plane::{
    SearchCorpusKind, SearchPlaneService, SearchQueryTelemetry, SearchQueryTelemetrySource,
};
use chrono::Utc;
use xiuxian_wendao_runtime::transport::{RepoSearchFlightRequest, RepoSearchFlightRouteProvider};

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
    let query = RepoRetrievalQuery::new(
        repo_id,
        search_term,
        &parsed.language_filters,
        &parsed.kind_filters,
        limit,
    );
    let relation = query_repo_entity_relation(search_plane, &query, Some(explain_sink.clone()))
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
    search_repo_content_hits_via_flight_contract(search_plane, repo_id, raw_query, limit).await
}

async fn search_repo_content_hits_via_flight_contract(
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
    let provider = SearchPlaneRepoSearchFlightRouteProvider::new(Arc::new(search_plane.clone()))
        .map_err(|error| {
            StudioApiError::internal(
                "REPO_CONTENT_SEARCH_BRIDGE_BUILD_FAILED",
                "Failed to build repo content Flight-backed provider",
                Some(error),
            )
        })?;
    let batch = provider
        .repo_search_batch(&RepoSearchFlightRequest {
            repo_id: repo_id.to_string(),
            query_text: search_term.to_string(),
            limit,
            language_filters: parsed.language_filters.clone(),
            path_prefixes: std::collections::HashSet::new(),
            title_filters: std::collections::HashSet::new(),
            tag_filters: std::collections::HashSet::new(),
            filename_filters: std::collections::HashSet::new(),
        })
        .await
        .map_err(|error| {
            StudioApiError::internal(
                "REPO_CONTENT_SEARCH_FAILED",
                "Failed to query repo content through the Flight-backed provider",
                Some(error),
            )
        })?;

    decode_repo_search_flight_batch_to_search_hits(repo_id, &batch).map_err(|error| {
        StudioApiError::internal(
            "REPO_CONTENT_SEARCH_DECODE_FAILED",
            "Failed to decode repo content Flight-backed search batch",
            Some(error),
        )
    })
}

/// Search one repo through the entity-first, content-fallback code-search policy.
///
/// # Errors
///
/// Returns [`StudioApiError`] when one of the repo-scoped search lanes fails.
#[cfg(test)]
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
    let query = RepoCodeQueryRequest::new(
        target.repo_id.as_str(),
        search_term,
        &parsed.language_filters,
        &parsed.kind_filters,
        target.publication_state.entity_published,
        target.publication_state.content_published,
        query_limit,
    );
    let result = query_repo_code_relation(search_plane, &query, Some(explain_sink.clone()))
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
            rows.iter()
                .map(|row| retrieval_row_to_search_hit(repo_id, row)),
        );
    }
    Ok(hits)
}

fn retrieval_row_to_search_hit(repo_id: &str, row: &xiuxian_vector::RetrievalRow) -> SearchHit {
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

struct RepoSearchFlightColumns<'a> {
    paths: &'a LanceStringArray,
    titles: &'a LanceStringArray,
    best_sections: &'a LanceStringArray,
    match_reasons: &'a LanceStringArray,
    navigation_paths: &'a LanceStringArray,
    navigation_categories: &'a LanceStringArray,
    navigation_lines: &'a LanceInt32Array,
    navigation_line_ends: &'a LanceInt32Array,
    hierarchies: &'a LanceListArray,
    tags: &'a LanceListArray,
    scores: &'a xiuxian_vector::LanceFloat64Array,
    languages: &'a LanceStringArray,
}

impl<'a> RepoSearchFlightColumns<'a> {
    fn from_batch(batch: &'a LanceRecordBatch) -> Result<Self, String> {
        use xiuxian_wendao_runtime::transport::{
            REPO_SEARCH_BEST_SECTION_COLUMN, REPO_SEARCH_HIERARCHY_COLUMN,
            REPO_SEARCH_LANGUAGE_COLUMN, REPO_SEARCH_MATCH_REASON_COLUMN,
            REPO_SEARCH_NAVIGATION_CATEGORY_COLUMN, REPO_SEARCH_NAVIGATION_LINE_COLUMN,
            REPO_SEARCH_NAVIGATION_LINE_END_COLUMN, REPO_SEARCH_NAVIGATION_PATH_COLUMN,
            REPO_SEARCH_PATH_COLUMN, REPO_SEARCH_SCORE_COLUMN, REPO_SEARCH_TAGS_COLUMN,
            REPO_SEARCH_TITLE_COLUMN,
        };

        Ok(Self {
            paths: repo_search_string_column(batch, REPO_SEARCH_PATH_COLUMN)?,
            titles: repo_search_string_column(batch, REPO_SEARCH_TITLE_COLUMN)?,
            best_sections: repo_search_string_column(batch, REPO_SEARCH_BEST_SECTION_COLUMN)?,
            match_reasons: repo_search_string_column(batch, REPO_SEARCH_MATCH_REASON_COLUMN)?,
            navigation_paths: repo_search_string_column(batch, REPO_SEARCH_NAVIGATION_PATH_COLUMN)?,
            navigation_categories: repo_search_string_column(
                batch,
                REPO_SEARCH_NAVIGATION_CATEGORY_COLUMN,
            )?,
            navigation_lines: repo_search_int32_column(batch, REPO_SEARCH_NAVIGATION_LINE_COLUMN)?,
            navigation_line_ends: repo_search_int32_column(
                batch,
                REPO_SEARCH_NAVIGATION_LINE_END_COLUMN,
            )?,
            hierarchies: repo_search_list_column(batch, REPO_SEARCH_HIERARCHY_COLUMN)?,
            tags: repo_search_list_column(batch, REPO_SEARCH_TAGS_COLUMN)?,
            scores: repo_search_float64_column(batch, REPO_SEARCH_SCORE_COLUMN)?,
            languages: repo_search_string_column(batch, REPO_SEARCH_LANGUAGE_COLUMN)?,
        })
    }

    fn hit_at(&self, repo_id: &str, index: usize) -> Result<SearchHit, String> {
        let path = self.paths.value(index).to_string();
        let title = non_empty(self.titles.value(index));
        let best_section = non_empty(self.best_sections.value(index));
        let match_reason = non_empty(self.match_reasons.value(index));
        let language = non_empty(self.languages.value(index));
        let path_hierarchy = utf8_list_value(self.hierarchies, index)?;
        let tag_values = utf8_list_value(self.tags, index)?;
        let line = positive_int32_to_usize(self.navigation_lines.value(index));
        let line_end = positive_int32_to_usize(self.navigation_line_ends.value(index));
        let navigation_path = non_empty(self.navigation_paths.value(index));
        let navigation_category = non_empty(self.navigation_categories.value(index));

        Ok(SearchHit {
            stem: repo_search_hit_stem(path.as_str()),
            title: title.or_else(|| Some(path.clone())),
            path,
            doc_type: Some("file".to_string()),
            tags: normalize_repo_search_tags(repo_id, tag_values, language.as_deref()),
            score: self.scores.value(index),
            best_section,
            match_reason,
            hierarchical_uri: None,
            hierarchy: Some(path_hierarchy),
            saliency_score: None,
            audit_status: None,
            verification_state: None,
            implicit_backlinks: None,
            implicit_backlink_items: None,
            navigation_target: navigation_path.map(|navigation_path| StudioNavigationTarget {
                path: navigation_path,
                category: navigation_category.unwrap_or_else(|| "repo_code".to_string()),
                project_name: Some(repo_id.to_string()),
                root_label: Some(repo_id.to_string()),
                line,
                line_end,
                column: None,
            }),
        })
    }
}

fn decode_repo_search_flight_batch_to_search_hits(
    repo_id: &str,
    batch: &LanceRecordBatch,
) -> Result<Vec<SearchHit>, String> {
    let columns = RepoSearchFlightColumns::from_batch(batch)?;
    (0..batch.num_rows())
        .map(|index| columns.hit_at(repo_id, index))
        .collect()
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn positive_int32_to_usize(value: i32) -> Option<usize> {
    if value <= 0 {
        return None;
    }
    usize::try_from(u32::try_from(value).ok()?).ok()
}

fn utf8_list_value(array: &LanceListArray, index: usize) -> Result<Vec<String>, String> {
    let offsets = array.value_offsets();
    let start = offset_i32_to_usize(offsets[index])?;
    let end = offset_i32_to_usize(offsets[index + 1])?;
    let strings = array
        .values()
        .as_any()
        .downcast_ref::<LanceStringArray>()
        .ok_or_else(|| "repo-search list value must be utf8".to_string())?;
    Ok((start..end)
        .map(|inner| strings.value(inner).to_string())
        .collect())
}

fn repo_search_string_column<'a>(
    batch: &'a LanceRecordBatch,
    column_name: &str,
) -> Result<&'a LanceStringArray, String> {
    batch
        .column_by_name(column_name)
        .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
        .ok_or_else(|| format!("missing `{column_name}` string column"))
}

fn repo_search_int32_column<'a>(
    batch: &'a LanceRecordBatch,
    column_name: &str,
) -> Result<&'a LanceInt32Array, String> {
    batch
        .column_by_name(column_name)
        .and_then(|column| column.as_any().downcast_ref::<LanceInt32Array>())
        .ok_or_else(|| format!("missing `{column_name}` int32 column"))
}

fn repo_search_list_column<'a>(
    batch: &'a LanceRecordBatch,
    column_name: &str,
) -> Result<&'a LanceListArray, String> {
    batch
        .column_by_name(column_name)
        .and_then(|column| column.as_any().downcast_ref::<LanceListArray>())
        .ok_or_else(|| format!("missing `{column_name}` list column"))
}

fn repo_search_float64_column<'a>(
    batch: &'a LanceRecordBatch,
    column_name: &str,
) -> Result<&'a xiuxian_vector::LanceFloat64Array, String> {
    batch
        .column_by_name(column_name)
        .and_then(|column| {
            column
                .as_any()
                .downcast_ref::<xiuxian_vector::LanceFloat64Array>()
        })
        .ok_or_else(|| format!("missing `{column_name}` float64 column"))
}

fn repo_search_hit_stem(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(path)
        .to_string()
}

fn normalize_repo_search_tags(
    repo_id: &str,
    mut tags: Vec<String>,
    language: Option<&str>,
) -> Vec<String> {
    if let Some(language_value) = language
        && !tags
            .iter()
            .any(|tag| tag.eq_ignore_ascii_case(format!("lang:{language_value}").as_str()))
    {
        tags.push(language_value.to_string());
        tags.push(format!("lang:{language_value}"));
    }
    if !tags.iter().any(|tag| tag == "code") {
        tags.push("code".to_string());
    }
    if !tags.iter().any(|tag| tag == "file") {
        tags.push("file".to_string());
    }
    if !tags.iter().any(|tag| tag == "kind:file") {
        tags.push("kind:file".to_string());
    }
    if !tags.iter().any(|tag| tag == repo_id) {
        tags.push(repo_id.to_string());
    }
    tags
}

fn offset_i32_to_usize(value: i32) -> Result<usize, String> {
    usize::try_from(
        u32::try_from(value)
            .map_err(|_| format!("repo-search list offset must be non-negative, got {value}"))?,
    )
    .map_err(|_| format!("repo-search list offset is too large: {value}"))
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

#[cfg(test)]
mod tests {
    use xiuxian_vector::{LanceInt32Array, LanceListBuilder, LanceStringBuilder};

    use super::{offset_i32_to_usize, positive_int32_to_usize, utf8_list_value};

    #[test]
    fn positive_int32_to_usize_rejects_non_positive_values() {
        assert_eq!(positive_int32_to_usize(-1), None);
        assert_eq!(positive_int32_to_usize(0), None);
        assert_eq!(positive_int32_to_usize(7), Some(7));
    }

    #[test]
    fn offset_i32_to_usize_rejects_negative_offsets() {
        assert!(offset_i32_to_usize(-1).is_err());
        assert_eq!(offset_i32_to_usize(3).unwrap(), 3);
    }

    #[test]
    fn utf8_list_value_reads_values_for_one_row() {
        let mut builder = LanceListBuilder::new(LanceStringBuilder::new());
        builder.values().append_value("alpha");
        builder.values().append_value("beta");
        builder.append(true);
        builder.values().append_value("gamma");
        builder.append(true);
        let array = builder.finish();

        assert_eq!(
            utf8_list_value(&array, 0).unwrap(),
            vec!["alpha".to_string(), "beta".to_string()]
        );
        assert_eq!(
            utf8_list_value(&array, 1).unwrap(),
            vec!["gamma".to_string()]
        );
    }

    #[test]
    fn offset_i32_to_usize_accepts_int32_builder_offsets() {
        let offsets = LanceInt32Array::from(vec![0, 2, 3]);
        assert_eq!(offset_i32_to_usize(offsets.value(1)).unwrap(), 2);
    }
}
