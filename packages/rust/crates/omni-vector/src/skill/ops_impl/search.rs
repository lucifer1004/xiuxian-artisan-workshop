#[allow(clippy::doc_markdown)]
impl VectorStore {
    /// Search for tools using hybrid search (vector + keyword).
    ///
    /// # Errors
    ///
    /// Returns an error if table scanning fails or if an invalid `where_filter`
    /// is provided for Lance filter parsing.
    pub async fn search_tools(
        &self,
        table_name: &str,
        query_vector: &[f32],
        query_text: Option<&str>,
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<skill::ToolSearchResult>, VectorStoreError> {
        self.search_tools_with_options(
            table_name,
            query_vector,
            query_text,
            limit,
            threshold,
            skill::ToolSearchOptions::default(),
            None,
        )
        .await
    }

    /// Search for tools with explicit ranking options.
    /// When `where_filter` is set (e.g. `skill_name = 'git'`), only rows matching the predicate are scanned.
    ///
    /// # Errors
    ///
    /// Returns an error if projection/filter configuration fails, if stream scans
    /// fail at the table boundary, or if `where_filter` is invalid.
    #[allow(clippy::too_many_arguments)]
    pub async fn search_tools_with_options(
        &self,
        table_name: &str,
        query_vector: &[f32],
        query_text: Option<&str>,
        limit: usize,
        threshold: f32,
        options: skill::ToolSearchOptions,
        where_filter: Option<&str>,
    ) -> Result<Vec<skill::ToolSearchResult>, VectorStoreError> {
        let mut results_map = self
            .collect_vector_tool_results(table_name, query_vector, where_filter)
            .await?;
        if let Some(text) = query_text {
            results_map = self
                .fuse_tool_results_with_keyword(table_name, text, limit, options, results_map)
                .await;
        }
        Ok(finalize_tool_results(results_map, threshold, limit))
    }

    async fn collect_vector_tool_results(
        &self,
        table_name: &str,
        query_vector: &[f32],
        where_filter: Option<&str>,
    ) -> Result<ToolResultsMap, VectorStoreError> {
        let mut results_map = ToolResultsMap::new();
        let table_path = self.table_path(table_name);
        if !table_path.exists() {
            return Ok(results_map);
        }

        let Ok(dataset) = self
            .open_dataset_at_uri(table_path.to_string_lossy().as_ref())
            .await
        else {
            return Ok(results_map);
        };
        let schema = dataset.schema();
        let has_metadata = schema.field(METADATA_COLUMN).is_some();
        let project_cols = search_project_columns(has_metadata);
        let mut scanner = dataset.scan();
        scanner.project(&project_cols).ok();

        let skill_filter_from_where = where_filter.and_then(parse_skill_name_from_where_filter);
        if let Some(filter) = where_filter
            && skill_filter_from_where.is_none()
        {
            scanner
                .filter(filter)
                .map_err(|e| VectorStoreError::General(format!("Invalid where_filter: {e}")))?;
        }

        let Ok(mut stream) = scanner.try_into_stream().await else {
            return Ok(results_map);
        };
        while let Ok(Some(batch)) = stream.try_next().await {
            append_vector_results_from_batch(
                &batch,
                query_vector,
                skill_filter_from_where.as_deref(),
                &mut results_map,
            );
        }
        Ok(results_map)
    }

    async fn fuse_tool_results_with_keyword(
        &self,
        table_name: &str,
        query_text: &str,
        limit: usize,
        options: skill::ToolSearchOptions,
        vector_results: ToolResultsMap,
    ) -> ToolResultsMap {
        let mut vector_scores: Vec<(String, f32)> = vector_results
            .iter()
            .map(|(name, result)| (name.clone(), result.score))
            .collect();
        vector_scores.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

        let kw_hits = self
            .keyword_search(table_name, query_text, limit * 2)
            .await
            .unwrap_or_default();
        let fused = apply_weighted_rrf(
            vector_scores,
            kw_hits.clone(),
            keyword::RRF_K,
            options.semantic_weight.unwrap_or(keyword::SEMANTIC_WEIGHT),
            options.keyword_weight.unwrap_or(keyword::KEYWORD_WEIGHT),
            query_text,
        );
        let kw_lookup: ToolResultsMap = kw_hits
            .into_iter()
            .map(|result| (result.tool_name.clone(), result))
            .collect();
        let query_parts = normalize_query_terms(query_text);
        let file_discovery_intent = query_has_file_discovery_intent(&query_parts);
        let mut merged_results = ToolResultsMap::new();

        for fused_item in fused {
            let Some(mut tool) = vector_results
                .get(&fused_item.tool_name)
                .cloned()
                .or_else(|| kw_lookup.get(&fused_item.tool_name).cloned())
            else {
                continue;
            };
            tool.score = fused_item.rrf_score;
            if options.rerank {
                let mut rerank_bonus = tool_metadata_alignment_boost(&tool, &query_parts);
                if file_discovery_intent {
                    if tool.tool_name == "advanced_tools.smart_find" {
                        rerank_bonus += 0.70;
                    } else if tool_file_discovery_match(&tool) {
                        rerank_bonus += 0.30;
                    }
                }
                tool.score += rerank_bonus;
            }
            tool.vector_score = Some(fused_item.vector_score);
            tool.keyword_score = Some(fused_item.keyword_score);
            merged_results.insert(fused_item.tool_name, tool);
        }
        merged_results
    }
}

type ToolResultsMap = std::collections::HashMap<String, skill::ToolSearchResult>;
type SearchDynArrayRef<'a> = Option<&'a std::sync::Arc<dyn lance::deps::arrow_array::Array>>;

struct SearchBatchColumns<'a> {
    values: &'a lance::deps::arrow_array::Float32Array,
    content: &'a lance::deps::arrow_array::StringArray,
    ids: &'a lance::deps::arrow_array::StringArray,
    metadata: Option<&'a lance::deps::arrow_array::StringArray>,
    skill_name: SearchDynArrayRef<'a>,
    category: SearchDynArrayRef<'a>,
    tool_name: SearchDynArrayRef<'a>,
    file_path: SearchDynArrayRef<'a>,
    routing_keywords: SearchDynArrayRef<'a>,
    intents: SearchDynArrayRef<'a>,
    row_count: usize,
}

struct ResolvedSearchRow {
    canonical_tool_name: String,
    skill_name: String,
    file_path: String,
    routing_keywords: Vec<String>,
    intents: Vec<String>,
    category: String,
    input_schema: serde_json::Value,
}

fn search_project_columns(has_metadata: bool) -> Vec<&'static str> {
    if has_metadata {
        vec![
            VECTOR_COLUMN,
            METADATA_COLUMN,
            CONTENT_COLUMN,
            "id",
            crate::SKILL_NAME_COLUMN,
            crate::CATEGORY_COLUMN,
            crate::TOOL_NAME_COLUMN,
            crate::FILE_PATH_COLUMN,
            crate::ROUTING_KEYWORDS_COLUMN,
            crate::INTENTS_COLUMN,
        ]
    } else {
        vec![
            VECTOR_COLUMN,
            CONTENT_COLUMN,
            "id",
            crate::SKILL_NAME_COLUMN,
            crate::CATEGORY_COLUMN,
            crate::TOOL_NAME_COLUMN,
            crate::FILE_PATH_COLUMN,
            crate::ROUTING_KEYWORDS_COLUMN,
            crate::INTENTS_COLUMN,
        ]
    }
}

fn query_has_file_discovery_intent(query_parts: &[String]) -> bool {
    query_parts.iter().any(|part| {
        matches!(
            part.as_str(),
            "find" | "list" | "file" | "files" | "directory" | "folder" | "path" | "glob"
        ) || part.starts_with("*.")
    })
}

fn finalize_tool_results(
    results_map: ToolResultsMap,
    threshold: f32,
    limit: usize,
) -> Vec<skill::ToolSearchResult> {
    let mut results: Vec<_> = results_map.into_values().collect();
    if threshold > 0.0 {
        results.retain(|result| result.score >= threshold);
    }
    results.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.tool_name.cmp(&b.tool_name))
    });
    results.truncate(limit);
    results
}

fn append_vector_results_from_batch(
    batch: &lance::deps::arrow_array::RecordBatch,
    query_vector: &[f32],
    skill_filter: Option<&str>,
    results_map: &mut ToolResultsMap,
) {
    let Some(columns) = extract_search_batch_columns(batch) else {
        return;
    };
    for row_index in 0..columns.row_count {
        let Some((canonical_name, result)) =
            build_vector_result_row(row_index, query_vector, skill_filter, &columns)
        else {
            continue;
        };
        results_map.insert(canonical_name, result);
    }
}

fn extract_search_batch_columns(
    batch: &lance::deps::arrow_array::RecordBatch,
) -> Option<SearchBatchColumns<'_>> {
    use lance::deps::arrow_array::Array;

    let vector_col = batch.column_by_name(VECTOR_COLUMN)?;
    let content_col = batch.column_by_name(CONTENT_COLUMN)?;
    let id_col = batch.column_by_name("id")?;
    let vector_arr = vector_col
        .as_any()
        .downcast_ref::<lance::deps::arrow_array::FixedSizeListArray>()?;
    let content = content_col
        .as_any()
        .downcast_ref::<lance::deps::arrow_array::StringArray>()?;
    let ids = id_col
        .as_any()
        .downcast_ref::<lance::deps::arrow_array::StringArray>()?;
    let values = vector_arr
        .values()
        .as_any()
        .downcast_ref::<lance::deps::arrow_array::Float32Array>()?;

    Some(SearchBatchColumns {
        values,
        content,
        ids,
        metadata: batch.column_by_name(METADATA_COLUMN).and_then(|column| {
            column
                .as_any()
                .downcast_ref::<lance::deps::arrow_array::StringArray>()
        }),
        skill_name: batch.column_by_name(crate::SKILL_NAME_COLUMN),
        category: batch.column_by_name(crate::CATEGORY_COLUMN),
        tool_name: batch.column_by_name(crate::TOOL_NAME_COLUMN),
        file_path: batch.column_by_name(crate::FILE_PATH_COLUMN),
        routing_keywords: batch.column_by_name(crate::ROUTING_KEYWORDS_COLUMN),
        intents: batch.column_by_name(crate::INTENTS_COLUMN),
        row_count: batch.num_rows(),
    })
}

fn build_vector_result_row(
    row_index: usize,
    query_vector: &[f32],
    skill_filter: Option<&str>,
    columns: &SearchBatchColumns<'_>,
) -> Option<(String, skill::ToolSearchResult)> {
    let skill_name = search_utf8_at(columns.skill_name, row_index);
    if skill_filter.is_some_and(|skill| skill_name != skill) {
        return None;
    }
    let category = search_utf8_at(columns.category, row_index);
    let score = vector_score_for_row(columns.values, columns.row_count, row_index, query_vector);
    let row_id = columns.ids.value(row_index).to_string();
    let resolved = resolve_search_row_fields(row_index, &row_id, &skill_name, &category, columns)?;
    let tool_name = if row_id.contains('.') {
        row_id
    } else {
        resolved.canonical_tool_name.clone()
    };
    if !skill::is_routable_tool_name(&tool_name) {
        return None;
    }

    Some((
        resolved.canonical_tool_name,
        skill::ToolSearchResult {
            name: tool_name.clone(),
            description: columns.content.value(row_index).to_string(),
            input_schema: resolved.input_schema,
            score,
            vector_score: Some(score),
            keyword_score: None,
            skill_name: resolved.skill_name,
            tool_name,
            file_path: resolved.file_path,
            routing_keywords: resolved.routing_keywords,
            intents: resolved.intents,
            category: resolved.category,
            parameters: vec![],
        },
    ))
}

fn vector_score_for_row(
    values: &lance::deps::arrow_array::Float32Array,
    row_count: usize,
    row_index: usize,
    query_vector: &[f32],
) -> f32 {
    if row_count == 0 {
        return 0.0;
    }
    let vector_len = values.len() / row_count;
    let mut dist_sq = 0.0f32;
    for (vector_index, query_value) in query_vector.iter().copied().enumerate() {
        let db_value = if vector_index < vector_len {
            values.value(row_index * vector_len + vector_index)
        } else {
            0.0
        };
        let diff = db_value - query_value;
        dist_sq += diff * diff;
    }
    1.0 / (1.0 + dist_sq.sqrt())
}

fn resolve_search_row_fields(
    row_index: usize,
    row_id: &str,
    skill_name: &str,
    category: &str,
    columns: &SearchBatchColumns<'_>,
) -> Option<ResolvedSearchRow> {
    if let Some(metadata_arr) = columns.metadata {
        use lance::deps::arrow_array::Array;

        if metadata_arr.is_null(row_index) {
            return Some(resolve_search_row_from_columns(
                row_index, row_id, skill_name, category, columns,
            ));
        }
        let metadata =
            serde_json::from_str::<serde_json::Value>(metadata_arr.value(row_index)).ok()?;
        return resolve_search_row_from_metadata(&metadata, row_id);
    }

    Some(resolve_search_row_from_columns(
        row_index, row_id, skill_name, category, columns,
    ))
}

fn resolve_search_row_from_columns(
    row_index: usize,
    row_id: &str,
    skill_name: &str,
    category: &str,
    columns: &SearchBatchColumns<'_>,
) -> ResolvedSearchRow {
    let tool_name = search_utf8_at(columns.tool_name, row_index);
    let canonical_tool_name = if tool_name.is_empty() {
        row_id.to_string()
    } else {
        tool_name
    };
    let resolved_skill_name = if skill_name.is_empty() {
        canonical_tool_name
            .split('.')
            .next()
            .unwrap_or("")
            .to_string()
    } else {
        skill_name.to_string()
    };
    let routing_keywords_raw = search_routing_keywords_at(columns.routing_keywords, row_index);
    let intents_raw = search_intents_at(columns.intents, row_index);
    let metadata = search_metadata_from_arrays(&routing_keywords_raw, &intents_raw);

    ResolvedSearchRow {
        canonical_tool_name,
        skill_name: resolved_skill_name.clone(),
        file_path: search_utf8_at(columns.file_path, row_index),
        routing_keywords: skill::resolve_routing_keywords(&metadata),
        intents: skill::resolve_intents(&metadata),
        category: if category.is_empty() {
            resolved_skill_name
        } else {
            category.to_string()
        },
        input_schema: serde_json::json!({}),
    }
}

fn resolve_search_row_from_metadata(
    metadata: &serde_json::Value,
    row_id: &str,
) -> Option<ResolvedSearchRow> {
    if metadata.get("type").and_then(|kind| kind.as_str()) != Some("command") {
        return None;
    }
    let canonical_tool_name = canonical_tool_name_from_result_meta(metadata, row_id)?;
    let skill_name = metadata
        .get("skill_name")
        .and_then(|value| value.as_str())
        .map_or_else(
            || {
                canonical_tool_name
                    .split('.')
                    .next()
                    .unwrap_or("")
                    .to_string()
            },
            String::from,
        );
    let file_path = metadata
        .get("file_path")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let category = metadata
        .get("category")
        .and_then(|value| value.as_str())
        .or_else(|| metadata.get("skill_name").and_then(|value| value.as_str()))
        .unwrap_or("")
        .to_string();
    let input_schema = metadata.get("input_schema").map_or_else(
        || serde_json::json!({}),
        skill::normalize_input_schema_value,
    );

    Some(ResolvedSearchRow {
        canonical_tool_name,
        skill_name,
        file_path,
        routing_keywords: skill::resolve_routing_keywords(metadata),
        intents: skill::resolve_intents(metadata),
        category,
        input_schema,
    })
}

fn search_utf8_at(col: SearchDynArrayRef<'_>, row_index: usize) -> String {
    col.map(|column| crate::ops::get_utf8_at(column.as_ref(), row_index))
        .unwrap_or_default()
}

fn search_routing_keywords_at(col: SearchDynArrayRef<'_>, row_index: usize) -> Vec<String> {
    col.map(|column| crate::ops::get_routing_keywords_at(column.as_ref(), row_index))
        .unwrap_or_default()
}

fn search_intents_at(col: SearchDynArrayRef<'_>, row_index: usize) -> Vec<String> {
    col.map(|column| crate::ops::get_intents_at(column.as_ref(), row_index))
        .unwrap_or_default()
}

fn search_metadata_from_arrays(
    routing_keywords: &[String],
    intents: &[String],
) -> serde_json::Value {
    serde_json::json!({
        "routing_keywords": routing_keywords
            .iter()
            .map(|value| serde_json::Value::String(value.clone()))
            .collect::<Vec<_>>(),
        "intents": intents
            .iter()
            .map(|value| serde_json::Value::String(value.clone()))
            .collect::<Vec<_>>(),
    })
}

/// Parse `skill_name = 'value'` from a `where_filter` string for Rust-side filtering
/// (Lance filter on dictionary columns can return no rows).
fn parse_skill_name_from_where_filter(where_filter: &str) -> Option<String> {
    let prefix = "skill_name = '";
    let f = where_filter.trim();
    if !f.starts_with(prefix) {
        return None;
    }
    let rest = f.get(prefix.len()..)?;
    let mut end = 0usize;
    let mut it = rest.char_indices();
    while let Some((i, c)) = it.next() {
        if c == '\'' {
            if rest.get(i + 1..)?.starts_with('\'') {
                it.next();
                end = i + 2;
                continue;
            }
            end = i;
            break;
        }
        end = i + c.len_utf8();
    }
    Some(rest[..end].replace("''", "'"))
}

fn normalize_query_terms(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '*' || c == '.' || c == '_'))
        .filter(|t| !t.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn canonical_tool_name_from_result_meta(meta: &serde_json::Value, row_id: &str) -> Option<String> {
    let skill_name = meta
        .get("skill_name")
        .and_then(|s| s.as_str())
        .map_or("", str::trim);
    let tool_name = meta
        .get("tool_name")
        .and_then(|s| s.as_str())
        .map_or("", str::trim);
    if skill::is_routable_tool_name(tool_name) && tool_name.contains('.') {
        return Some(tool_name.to_string());
    }
    if !skill_name.is_empty() && skill::is_routable_tool_name(tool_name) {
        let candidate = format!("{skill_name}.{tool_name}");
        if skill::is_routable_tool_name(&candidate) {
            return Some(candidate);
        }
    }

    let command = meta
        .get("command")
        .and_then(|s| s.as_str())
        .map_or("", str::trim);
    if !skill_name.is_empty() && !command.is_empty() {
        let candidate = format!("{skill_name}.{command}");
        if skill::is_routable_tool_name(&candidate) {
            return Some(candidate);
        }
    }

    if skill::is_routable_tool_name(command) {
        return Some(command.to_string());
    }
    if skill::is_routable_tool_name(row_id) {
        return Some(row_id.to_string());
    }
    None
}

fn tool_metadata_alignment_boost(tool: &skill::ToolSearchResult, query_parts: &[String]) -> f32 {
    if query_parts.is_empty() {
        return 0.0;
    }

    let mut boost = 0.0f32;
    let category = tool.category.to_lowercase();
    let description = tool.description.to_lowercase();

    for term in query_parts {
        if term.len() <= 2 {
            continue;
        }
        if !category.is_empty() && category.contains(term) {
            boost += 0.05;
        }
        if description.contains(term) {
            boost += 0.03;
        }
        if tool
            .routing_keywords
            .iter()
            .any(|k| k.to_lowercase().contains(term))
        {
            boost += 0.07;
        }
        if tool.intents.iter().any(|i| i.to_lowercase().contains(term)) {
            boost += 0.08;
        }
    }

    boost.min(0.50)
}

fn tool_file_discovery_match(tool: &skill::ToolSearchResult) -> bool {
    let tool_name = tool.tool_name.to_lowercase();
    if tool_name == "advanced_tools.smart_find" {
        return true;
    }

    let category = tool.category.to_lowercase();
    let description = tool.description.to_lowercase();
    let terms = [
        "find",
        "file",
        "files",
        "directory",
        "folder",
        "path",
        "glob",
    ];
    terms.iter().any(|t| {
        category.contains(t)
            || description.contains(t)
            || tool
                .routing_keywords
                .iter()
                .any(|k| k.to_lowercase().contains(t))
            || tool.intents.iter().any(|i| i.to_lowercase().contains(t))
    })
}
