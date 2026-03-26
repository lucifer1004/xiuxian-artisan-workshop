use std::collections::BTreeMap;

use arrow::array::ListArray;
use xiuxian_vector::{ColumnarScanOptions, LanceArray, VectorStore};

use crate::gateway::studio::types::SearchHit;
use crate::search_plane::repo_entity::query::hydrate::{
    float64_column, hit_json_projection_columns, id_filter_expression, list_string_column,
    string_column, uint32_column,
};
use crate::search_plane::repo_entity::query::types::{
    HydratedRepoEntityRow, RepoEntityCandidate, RepoEntitySearchError,
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
        projected_columns: hit_json_projection_columns(),
        batch_size: Some(ids.len().min(128)),
        limit: Some(ids.len()),
        ..ColumnarScanOptions::default()
    };

    store
        .scan_record_batches_streaming(table_name, options, |batch| {
            let id = string_column(&batch, "id")?;
            let hit_json = string_column(&batch, "hit_json")?;
            for row in 0..batch.num_rows() {
                payloads.insert(id.value(row).to_string(), hit_json.value(row).to_string());
            }
            Ok::<(), RepoEntitySearchError>(())
        })
        .await?;

    Ok(payloads)
}

fn list_string_values(column: &ListArray, row: usize) -> Vec<String> {
    if column.is_null(row) {
        return Vec::new();
    }
    let values = column.value(row);
    let Some(strings) = values
        .as_any()
        .downcast_ref::<xiuxian_vector::LanceStringArray>()
    else {
        return Vec::new();
    };
    (0..strings.len())
        .filter(|index| !strings.is_null(*index))
        .map(|index| strings.value(index).trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

fn optional_string_value(column: &xiuxian_vector::LanceStringArray, row: usize) -> Option<String> {
    if column.is_null(row) {
        return None;
    }

    let value = column.value(row).trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn optional_u32_value(column: &xiuxian_vector::LanceUInt32Array, row: usize) -> Option<u32> {
    (!column.is_null(row)).then(|| column.value(row))
}
