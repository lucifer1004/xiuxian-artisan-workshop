use std::collections::BTreeMap;

use arrow::array::ListArray;
use xiuxian_vector::{
    ColumnarScanOptions, LanceArray, LanceFloat64Array, LanceRecordBatch, LanceStringArray,
    LanceUInt32Array, VectorStore,
};

use crate::analyzers::query::{
    ExampleSearchHit, ExampleSearchResult, ModuleSearchHit, ModuleSearchResult, RepoBacklinkItem,
    SymbolSearchHit, SymbolSearchResult,
};
use crate::analyzers::{ExampleRecord, ModuleRecord, RepoSymbolKind, SymbolRecord};
use crate::gateway::studio::types::SearchHit;
use crate::search_plane::repo_entity::query::types::{
    HydratedRepoEntityRow, RepoEntityCandidate, RepoEntitySearchError,
};
use crate::search_plane::repo_entity::schema::{
    COLUMN_ATTRIBUTES_JSON, COLUMN_AUDIT_STATUS, COLUMN_HIERARCHICAL_URI, COLUMN_HIERARCHY,
    COLUMN_IMPLICIT_BACKLINK_ITEMS_JSON, COLUMN_IMPLICIT_BACKLINKS, COLUMN_LINE_END,
    COLUMN_LINE_START, COLUMN_MODULE_ID, COLUMN_NAME, COLUMN_PATH, COLUMN_PROJECTION_PAGE_IDS,
    COLUMN_QUALIFIED_NAME, COLUMN_SALIENCY_SCORE, COLUMN_SIGNATURE, COLUMN_SUMMARY,
    COLUMN_SYMBOL_KIND, COLUMN_VERIFICATION_STATE, hit_json_column, id_column,
};

pub(crate) async fn hydrate_repo_entity_hits(
    store: &VectorStore,
    table_name: &str,
    candidates: Vec<RepoEntityCandidate>,
) -> Result<Vec<SearchHit>, RepoEntitySearchError> {
    let ids = candidates
        .iter()
        .map(|candidate| candidate.id.clone())
        .collect::<Vec<_>>();
    let payloads = load_hit_payloads_by_id(store, table_name, ids.as_slice()).await?;
    candidates
        .into_iter()
        .map(|candidate| {
            let hit_json = payloads.get(candidate.id.as_str()).ok_or_else(|| {
                RepoEntitySearchError::Decode(format!(
                    "repo entity hydration missing payload for id `{}`",
                    candidate.id
                ))
            })?;
            let mut hit: SearchHit = serde_json::from_str(hit_json.as_str())
                .map_err(|error| RepoEntitySearchError::Decode(error.to_string()))?;
            hit.score = candidate.score;
            Ok(hit)
        })
        .collect()
}

pub(crate) async fn load_hydrated_rows_by_id(
    store: &VectorStore,
    table_name: &str,
    ids: &[String],
    projected_columns: &[String],
) -> Result<BTreeMap<String, HydratedRepoEntityRow>, RepoEntitySearchError> {
    if ids.is_empty() {
        return Ok(BTreeMap::new());
    }

    let mut rows = BTreeMap::new();
    let options = ColumnarScanOptions {
        where_filter: Some(id_filter_expression(ids)),
        projected_columns: projected_columns.to_vec(),
        batch_size: Some(ids.len().min(128)),
        limit: Some(ids.len()),
        ..ColumnarScanOptions::default()
    };

    store
        .scan_record_batches_streaming(table_name, options, |batch| {
            let id = string_column(&batch, "id")?;
            let name = string_column(&batch, "name")?;
            let qualified_name = string_column(&batch, "qualified_name")?;
            let path = string_column(&batch, "path")?;
            let symbol_kind = string_column(&batch, "symbol_kind")?;
            let module_id = string_column(&batch, "module_id")?;
            let signature = string_column(&batch, "signature")?;
            let summary = string_column(&batch, "summary")?;
            let line_start = uint32_column(&batch, "line_start")?;
            let line_end = uint32_column(&batch, "line_end")?;
            let audit_status = string_column(&batch, "audit_status")?;
            let verification_state = string_column(&batch, "verification_state")?;
            let attributes_json = string_column(&batch, "attributes_json")?;
            let hierarchical_uri = string_column(&batch, "hierarchical_uri")?;
            let hierarchy = list_string_column(&batch, "hierarchy")?;
            let implicit_backlinks = list_string_column(&batch, "implicit_backlinks")?;
            let implicit_backlink_items_json =
                string_column(&batch, "implicit_backlink_items_json")?;
            let projection_page_ids = list_string_column(&batch, "projection_page_ids")?;
            let saliency_score = float64_column(&batch, "saliency_score")?;

            for row in 0..batch.num_rows() {
                let id_value = id.value(row).to_string();
                rows.insert(
                    id_value.clone(),
                    HydratedRepoEntityRow {
                        id: id_value,
                        name: name.value(row).to_string(),
                        qualified_name: qualified_name.value(row).to_string(),
                        path: path.value(row).to_string(),
                        symbol_kind: symbol_kind.value(row).to_string(),
                        module_id: optional_string_value(module_id, row),
                        signature: optional_string_value(signature, row),
                        summary: optional_string_value(summary, row),
                        line_start: optional_u32_value(line_start, row),
                        line_end: optional_u32_value(line_end, row),
                        audit_status: optional_string_value(audit_status, row),
                        verification_state: optional_string_value(verification_state, row),
                        attributes_json: optional_string_value(attributes_json, row),
                        hierarchical_uri: optional_string_value(hierarchical_uri, row),
                        hierarchy: list_string_values(hierarchy, row),
                        implicit_backlinks: list_string_values(implicit_backlinks, row),
                        implicit_backlink_items_json: optional_string_value(
                            implicit_backlink_items_json,
                            row,
                        ),
                        projection_page_ids: list_string_values(projection_page_ids, row),
                        saliency_score: saliency_score.value(row),
                    },
                );
            }
            Ok::<(), RepoEntitySearchError>(())
        })
        .await?;

    Ok(rows)
}

pub(crate) fn build_module_search_result(
    repo_id: &str,
    candidates: Vec<RepoEntityCandidate>,
    rows: BTreeMap<String, HydratedRepoEntityRow>,
) -> Result<ModuleSearchResult, RepoEntitySearchError> {
    let mut modules = Vec::with_capacity(candidates.len());
    let mut module_hits = Vec::with_capacity(candidates.len());
    for (index, candidate) in candidates.into_iter().enumerate() {
        let row = rows.get(candidate.id.as_str()).ok_or_else(|| {
            RepoEntitySearchError::Decode(format!(
                "repo entity hydration missing structured row for id `{}`",
                candidate.id
            ))
        })?;
        let module = ModuleRecord {
            repo_id: repo_id.to_string(),
            module_id: row.id.clone(),
            qualified_name: row.qualified_name.clone(),
            path: row.path.clone(),
        };
        modules.push(module.clone());
        module_hits.push(ModuleSearchHit {
            module,
            score: Some(candidate.score),
            rank: Some(index + 1),
            saliency_score: Some(row.saliency_score),
            hierarchical_uri: row.hierarchical_uri.clone(),
            hierarchy: non_empty_vec(row.hierarchy.clone()),
            implicit_backlinks: non_empty_vec(row.implicit_backlinks.clone()),
            implicit_backlink_items: parse_backlink_items(
                row.implicit_backlink_items_json.as_deref(),
            )?,
            projection_page_ids: non_empty_vec(row.projection_page_ids.clone()),
        });
    }

    Ok(ModuleSearchResult {
        repo_id: repo_id.to_string(),
        modules,
        module_hits,
    })
}

pub(crate) fn build_symbol_search_result(
    repo_id: &str,
    candidates: Vec<RepoEntityCandidate>,
    rows: BTreeMap<String, HydratedRepoEntityRow>,
) -> Result<SymbolSearchResult, RepoEntitySearchError> {
    let mut symbols = Vec::with_capacity(candidates.len());
    let mut symbol_hits = Vec::with_capacity(candidates.len());
    for (index, candidate) in candidates.into_iter().enumerate() {
        let row = rows.get(candidate.id.as_str()).ok_or_else(|| {
            RepoEntitySearchError::Decode(format!(
                "repo entity hydration missing structured row for id `{}`",
                candidate.id
            ))
        })?;
        let audit_status = row.audit_status.clone();
        let verification_state = row.verification_state.clone().or_else(|| {
            audit_status.as_deref().map(|status| match status {
                "verified" | "approved" => "verified".to_string(),
                _ => "unverified".to_string(),
            })
        });
        let symbol = SymbolRecord {
            repo_id: repo_id.to_string(),
            symbol_id: row.id.clone(),
            module_id: row.module_id.clone(),
            name: row.name.clone(),
            qualified_name: row.qualified_name.clone(),
            kind: parse_symbol_kind(row.symbol_kind.as_str()),
            path: row.path.clone(),
            line_start: row.line_start.map(|value| value as usize),
            line_end: row.line_end.map(|value| value as usize),
            signature: row.signature.clone(),
            audit_status: audit_status.clone(),
            verification_state: verification_state.clone(),
            attributes: parse_attributes_map(row.attributes_json.as_deref())?,
        };
        symbols.push(symbol.clone());
        symbol_hits.push(SymbolSearchHit {
            symbol,
            score: Some(candidate.score),
            rank: Some(index + 1),
            saliency_score: Some(row.saliency_score),
            hierarchical_uri: row.hierarchical_uri.clone(),
            hierarchy: non_empty_vec(row.hierarchy.clone()),
            implicit_backlinks: non_empty_vec(row.implicit_backlinks.clone()),
            implicit_backlink_items: parse_backlink_items(
                row.implicit_backlink_items_json.as_deref(),
            )?,
            projection_page_ids: non_empty_vec(row.projection_page_ids.clone()),
            audit_status,
            verification_state,
        });
    }

    Ok(SymbolSearchResult {
        repo_id: repo_id.to_string(),
        symbols,
        symbol_hits,
    })
}

pub(crate) fn build_example_search_result(
    repo_id: &str,
    candidates: Vec<RepoEntityCandidate>,
    rows: BTreeMap<String, HydratedRepoEntityRow>,
) -> Result<ExampleSearchResult, RepoEntitySearchError> {
    let mut examples = Vec::with_capacity(candidates.len());
    let mut example_hits = Vec::with_capacity(candidates.len());
    for (index, candidate) in candidates.into_iter().enumerate() {
        let row = rows.get(candidate.id.as_str()).ok_or_else(|| {
            RepoEntitySearchError::Decode(format!(
                "repo entity hydration missing structured row for id `{}`",
                candidate.id
            ))
        })?;
        let example = ExampleRecord {
            repo_id: repo_id.to_string(),
            example_id: row.id.clone(),
            title: row.name.clone(),
            path: row.path.clone(),
            summary: row.summary.clone(),
        };
        examples.push(example.clone());
        example_hits.push(ExampleSearchHit {
            example,
            score: Some(candidate.score),
            rank: Some(index + 1),
            saliency_score: Some(row.saliency_score),
            hierarchical_uri: row.hierarchical_uri.clone(),
            hierarchy: non_empty_vec(row.hierarchy.clone()),
            implicit_backlinks: non_empty_vec(row.implicit_backlinks.clone()),
            implicit_backlink_items: parse_backlink_items(
                row.implicit_backlink_items_json.as_deref(),
            )?,
            projection_page_ids: non_empty_vec(row.projection_page_ids.clone()),
        });
    }

    Ok(ExampleSearchResult {
        repo_id: repo_id.to_string(),
        examples,
        example_hits,
    })
}

pub(crate) async fn load_hit_payloads_by_id(
    store: &VectorStore,
    table_name: &str,
    ids: &[String],
) -> Result<BTreeMap<String, String>, RepoEntitySearchError> {
    if ids.is_empty() {
        return Ok(BTreeMap::new());
    }

    let mut payloads = BTreeMap::new();
    let options = ColumnarScanOptions {
        where_filter: Some(id_filter_expression(ids)),
        projected_columns: vec![id_column().to_string(), hit_json_column().to_string()],
        batch_size: Some(ids.len().min(128)),
        limit: Some(ids.len()),
        ..ColumnarScanOptions::default()
    };

    store
        .scan_record_batches_streaming(table_name, options, |batch| {
            let id = string_column(&batch, id_column())?;
            let hit_json = string_column(&batch, hit_json_column())?;
            for row in 0..batch.num_rows() {
                payloads.insert(id.value(row).to_string(), hit_json.value(row).to_string());
            }
            Ok::<(), RepoEntitySearchError>(())
        })
        .await?;

    Ok(payloads)
}

pub(crate) fn typed_repo_entity_columns() -> Vec<String> {
    [
        id_column(),
        COLUMN_NAME,
        COLUMN_QUALIFIED_NAME,
        COLUMN_PATH,
        COLUMN_SYMBOL_KIND,
        COLUMN_MODULE_ID,
        COLUMN_SIGNATURE,
        COLUMN_SUMMARY,
        COLUMN_LINE_START,
        COLUMN_LINE_END,
        COLUMN_AUDIT_STATUS,
        COLUMN_VERIFICATION_STATE,
        COLUMN_ATTRIBUTES_JSON,
        COLUMN_HIERARCHICAL_URI,
        COLUMN_HIERARCHY,
        COLUMN_IMPLICIT_BACKLINKS,
        COLUMN_IMPLICIT_BACKLINK_ITEMS_JSON,
        COLUMN_PROJECTION_PAGE_IDS,
        COLUMN_SALIENCY_SCORE,
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

pub(crate) fn id_filter_expression(ids: &[String]) -> String {
    let escaped = ids
        .iter()
        .map(|value| format!("'{}'", value.replace('\'', "''")))
        .collect::<Vec<_>>();
    format!("{} IN ({})", id_column(), escaped.join(","))
}

pub(crate) fn string_column<'a>(
    batch: &'a LanceRecordBatch,
    name: &str,
) -> Result<&'a LanceStringArray, RepoEntitySearchError> {
    batch
        .column_by_name(name)
        .and_then(|array| array.as_any().downcast_ref::<LanceStringArray>())
        .ok_or_else(|| RepoEntitySearchError::Decode(format!("missing string column `{name}`")))
}

pub(crate) fn float64_column<'a>(
    batch: &'a LanceRecordBatch,
    name: &str,
) -> Result<&'a LanceFloat64Array, RepoEntitySearchError> {
    batch
        .column_by_name(name)
        .and_then(|array| array.as_any().downcast_ref::<LanceFloat64Array>())
        .ok_or_else(|| RepoEntitySearchError::Decode(format!("missing f64 column `{name}`")))
}

pub(crate) fn uint32_column<'a>(
    batch: &'a LanceRecordBatch,
    name: &str,
) -> Result<&'a LanceUInt32Array, RepoEntitySearchError> {
    batch
        .column_by_name(name)
        .and_then(|array| array.as_any().downcast_ref::<LanceUInt32Array>())
        .ok_or_else(|| RepoEntitySearchError::Decode(format!("missing u32 column `{name}`")))
}

pub(crate) fn list_string_column<'a>(
    batch: &'a LanceRecordBatch,
    name: &str,
) -> Result<&'a ListArray, RepoEntitySearchError> {
    batch
        .column_by_name(name)
        .and_then(|array| array.as_any().downcast_ref::<ListArray>())
        .ok_or_else(|| RepoEntitySearchError::Decode(format!("missing list column `{name}`")))
}

pub(crate) fn optional_string_value(column: &LanceStringArray, row: usize) -> Option<String> {
    if column.is_null(row) {
        return None;
    }

    let value = column.value(row).trim().to_string();
    (!value.is_empty()).then_some(value)
}

pub(crate) fn optional_u32_value(column: &LanceUInt32Array, row: usize) -> Option<u32> {
    (!column.is_null(row)).then(|| column.value(row))
}

pub(crate) fn list_string_values(column: &ListArray, row: usize) -> Vec<String> {
    if column.is_null(row) {
        return Vec::new();
    }
    let values = column.value(row);
    let Some(strings) = values.as_any().downcast_ref::<LanceStringArray>() else {
        return Vec::new();
    };
    (0..strings.len())
        .filter(|index| !strings.is_null(*index))
        .map(|index| strings.value(index).trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

pub(crate) fn non_empty_vec(values: Vec<String>) -> Option<Vec<String>> {
    (!values.is_empty()).then_some(values)
}

pub(crate) fn parse_backlink_items(
    value: Option<&str>,
) -> Result<Option<Vec<RepoBacklinkItem>>, RepoEntitySearchError> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(None);
    };
    let items = serde_json::from_str::<Vec<RepoBacklinkItem>>(value)
        .map_err(|error| RepoEntitySearchError::Decode(error.to_string()))?;
    Ok((!items.is_empty()).then_some(items))
}

pub(crate) fn parse_attributes_map(
    value: Option<&str>,
) -> Result<BTreeMap<String, String>, RepoEntitySearchError> {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(BTreeMap::new());
    };
    serde_json::from_str::<BTreeMap<String, String>>(value)
        .map_err(|error| RepoEntitySearchError::Decode(error.to_string()))
}

pub(crate) fn parse_symbol_kind(kind: &str) -> RepoSymbolKind {
    match kind {
        "function" => RepoSymbolKind::Function,
        "type" => RepoSymbolKind::Type,
        "constant" => RepoSymbolKind::Constant,
        "module_export" => RepoSymbolKind::ModuleExport,
        _ => RepoSymbolKind::Other,
    }
}
