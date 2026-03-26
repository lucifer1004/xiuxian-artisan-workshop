use arrow::array::ListArray;
use xiuxian_vector::{LanceFloat64Array, LanceRecordBatch, LanceStringArray, LanceUInt32Array};

use crate::search_plane::repo_entity::schema::{
    COLUMN_ATTRIBUTES_JSON, COLUMN_AUDIT_STATUS, COLUMN_HIERARCHICAL_URI, COLUMN_HIERARCHY,
    COLUMN_IMPLICIT_BACKLINK_ITEMS_JSON, COLUMN_IMPLICIT_BACKLINKS, COLUMN_LINE_END,
    COLUMN_LINE_START, COLUMN_MODULE_ID, COLUMN_NAME, COLUMN_PATH, COLUMN_PROJECTION_PAGE_IDS,
    COLUMN_QUALIFIED_NAME, COLUMN_SALIENCY_SCORE, COLUMN_SIGNATURE, COLUMN_SUMMARY,
    COLUMN_SYMBOL_KIND, COLUMN_VERIFICATION_STATE, hit_json_column, id_column,
};

use crate::search_plane::repo_entity::query::types::RepoEntitySearchError;

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

pub(crate) fn hit_json_projection_columns() -> Vec<String> {
    vec![id_column().to_string(), hit_json_column().to_string()]
}
