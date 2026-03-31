/// Canonical schema-version metadata header for Wendao Flight requests.
pub const WENDAO_SCHEMA_VERSION_HEADER: &str = "x-wendao-schema-version";
/// Canonical rerank-embedding dimension metadata header for Wendao Flight exchange requests.
pub const WENDAO_RERANK_DIMENSION_HEADER: &str = "x-wendao-rerank-embedding-dimension";
/// Canonical repo-search query text metadata header for Wendao Flight requests.
pub const WENDAO_REPO_SEARCH_QUERY_HEADER: &str = "x-wendao-repo-search-query";
/// Canonical repo-search result-limit metadata header for Wendao Flight requests.
pub const WENDAO_REPO_SEARCH_LIMIT_HEADER: &str = "x-wendao-repo-search-limit";
/// Canonical repo-search language-filter metadata header for Wendao Flight requests.
pub const WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER: &str = "x-wendao-repo-search-language-filters";
/// Canonical repo-search path-prefix metadata header for Wendao Flight requests.
pub const WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER: &str = "x-wendao-repo-search-path-prefixes";
/// Canonical repo-search title-filter metadata header for Wendao Flight requests.
pub const WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER: &str = "x-wendao-repo-search-title-filters";
/// Canonical repo-search tag-filter metadata header for Wendao Flight requests.
pub const WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER: &str = "x-wendao-repo-search-tag-filters";
/// Canonical repo-search filename-filter metadata header for Wendao Flight requests.
pub const WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER: &str =
    "x-wendao-repo-search-filename-filters";
/// Stable route for the repo-search query contract.
pub const REPO_SEARCH_ROUTE: &str = "/search/repos/main";
/// Stable route for the rerank exchange contract.
pub const RERANK_EXCHANGE_ROUTE: &str = "/rerank/flight";
/// Stable default result limit for repo-search requests.
pub const REPO_SEARCH_DEFAULT_LIMIT: usize = 10;
/// Canonical rerank request `doc_id` column.
pub const RERANK_REQUEST_DOC_ID_COLUMN: &str = "doc_id";
/// Canonical rerank request `vector_score` column.
pub const RERANK_REQUEST_VECTOR_SCORE_COLUMN: &str = "vector_score";
/// Canonical rerank request `embedding` column.
pub const RERANK_REQUEST_EMBEDDING_COLUMN: &str = "embedding";
/// Canonical rerank request `query_embedding` column.
pub const RERANK_REQUEST_QUERY_EMBEDDING_COLUMN: &str = "query_embedding";
/// Canonical rerank response `doc_id` column.
pub const RERANK_RESPONSE_DOC_ID_COLUMN: &str = "doc_id";
/// Canonical rerank response `final_score` column.
pub const RERANK_RESPONSE_FINAL_SCORE_COLUMN: &str = "final_score";
/// Canonical rerank response `rank` column.
pub const RERANK_RESPONSE_RANK_COLUMN: &str = "rank";
/// Canonical repo-search response `doc_id` column.
pub const REPO_SEARCH_DOC_ID_COLUMN: &str = "doc_id";
/// Canonical repo-search response `path` column.
pub const REPO_SEARCH_PATH_COLUMN: &str = "path";
/// Canonical repo-search response `title` column.
pub const REPO_SEARCH_TITLE_COLUMN: &str = "title";
/// Canonical repo-search response `best_section` column.
pub const REPO_SEARCH_BEST_SECTION_COLUMN: &str = "best_section";
/// Canonical repo-search response `score` column.
pub const REPO_SEARCH_SCORE_COLUMN: &str = "score";
/// Canonical repo-search response `language` column.
pub const REPO_SEARCH_LANGUAGE_COLUMN: &str = "language";

#[cfg(feature = "julia")]
use arrow_array::{
    FixedSizeListArray, Float32Array, Float64Array, Int32Array, RecordBatch, StringArray,
};
#[cfg(feature = "julia")]
use arrow_schema::{DataType, Schema};
#[cfg(feature = "julia")]
use std::collections::HashSet;

/// Normalize one route into the canonical leading-slash Flight form.
///
/// # Errors
///
/// Returns an error when the route resolves to no descriptor segments.
pub fn normalize_flight_route(route: impl AsRef<str>) -> Result<String, String> {
    let route = route.as_ref();
    let normalized = if route.starts_with('/') {
        route.to_string()
    } else {
        format!("/{route}")
    };
    if normalized.trim_matches('/').is_empty() {
        return Err(
            "Arrow Flight route must resolve to at least one descriptor segment".to_string(),
        );
    }
    Ok(normalized)
}

/// Convert one canonical Flight route into descriptor-path segments.
///
/// # Errors
///
/// Returns an error when the route resolves to no descriptor segments.
pub fn flight_descriptor_path(route: impl AsRef<str>) -> Result<Vec<String>, String> {
    let normalized = normalize_flight_route(route)?;
    let path = normalized
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if path.is_empty() {
        return Err(
            "Arrow Flight route must resolve to at least one descriptor segment".to_string(),
        );
    }
    Ok(path)
}

/// Validate the stable repo-search request contract.
///
/// # Errors
///
/// Returns an error when the repo-search query text is blank or the requested
/// limit is zero.
pub fn validate_repo_search_request(
    query_text: &str,
    limit: usize,
    language_filters: &[String],
    path_prefixes: &[String],
    title_filters: &[String],
    tag_filters: &[String],
    filename_filters: &[String],
) -> Result<(), String> {
    if query_text.trim().is_empty() {
        return Err("repo search query text must not be blank".to_string());
    }
    if limit == 0 {
        return Err("repo search limit must be greater than zero".to_string());
    }
    for language_filter in language_filters {
        if language_filter.trim().is_empty() {
            return Err("repo search language filters must not contain blank values".to_string());
        }
    }
    for path_prefix in path_prefixes {
        if path_prefix.trim().is_empty() {
            return Err("repo search path prefixes must not contain blank values".to_string());
        }
    }
    for title_filter in title_filters {
        if title_filter.trim().is_empty() {
            return Err("repo search title filters must not contain blank values".to_string());
        }
    }
    for tag_filter in tag_filters {
        if tag_filter.trim().is_empty() {
            return Err("repo search tag filters must not contain blank values".to_string());
        }
    }
    for filename_filter in filename_filters {
        if filename_filter.trim().is_empty() {
            return Err("repo search filename filters must not contain blank values".to_string());
        }
    }
    Ok(())
}

/// Validate the stable rerank request schema for one expected embedding dimension.
///
/// # Errors
///
/// Returns an error when the rerank request schema does not match the stable
/// Rust-owned column set or Arrow types.
#[cfg(feature = "julia")]
pub fn validate_rerank_request_schema(
    schema: &Schema,
    expected_dimension: usize,
) -> Result<(), String> {
    let doc_id = schema
        .field_with_name(RERANK_REQUEST_DOC_ID_COLUMN)
        .map_err(|_| format!("missing rerank request column `{RERANK_REQUEST_DOC_ID_COLUMN}`"))?;
    if !matches!(doc_id.data_type(), DataType::Utf8) {
        return Err(format!(
            "rerank request column `{RERANK_REQUEST_DOC_ID_COLUMN}` must be Utf8"
        ));
    }

    let vector_score = schema
        .field_with_name(RERANK_REQUEST_VECTOR_SCORE_COLUMN)
        .map_err(|_| {
            format!("missing rerank request column `{RERANK_REQUEST_VECTOR_SCORE_COLUMN}`")
        })?;
    if !matches!(vector_score.data_type(), DataType::Float32) {
        return Err(format!(
            "rerank request column `{RERANK_REQUEST_VECTOR_SCORE_COLUMN}` must be Float32"
        ));
    }

    validate_embedding_field(schema, RERANK_REQUEST_EMBEDDING_COLUMN, expected_dimension)?;
    validate_embedding_field(
        schema,
        RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
        expected_dimension,
    )?;
    Ok(())
}

/// Validate the stable rerank request payload semantics for one decoded batch.
///
/// # Errors
///
/// Returns an error when the rerank request batch is empty, contains blank
/// document IDs, contains duplicate document IDs, contains non-finite or
/// out-of-range vector scores, or carries drifted `query_embedding` values
/// across rows.
#[cfg(feature = "julia")]
pub fn validate_rerank_request_batch(
    batch: &RecordBatch,
    expected_dimension: usize,
) -> Result<(), String> {
    validate_rerank_request_schema(batch.schema().as_ref(), expected_dimension)?;
    if batch.num_rows() == 0 {
        return Err("rerank request batch must contain at least one row".to_string());
    }

    let doc_ids = batch
        .column_by_name(RERANK_REQUEST_DOC_ID_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| {
            format!("rerank request column `{RERANK_REQUEST_DOC_ID_COLUMN}` must decode as Utf8")
        })?;
    let mut seen_doc_ids = HashSet::new();
    for row_index in 0..batch.num_rows() {
        let doc_id = doc_ids.value(row_index).trim();
        if doc_id.is_empty() {
            return Err(format!(
                "rerank request column `{RERANK_REQUEST_DOC_ID_COLUMN}` must not contain blank values; row {row_index} is blank"
            ));
        }
        if !seen_doc_ids.insert(doc_id.to_string()) {
            return Err(format!(
                "rerank request column `{RERANK_REQUEST_DOC_ID_COLUMN}` must be unique across one batch; row {row_index} duplicates `{doc_id}`"
            ));
        }
    }

    let vector_scores = batch
        .column_by_name(RERANK_REQUEST_VECTOR_SCORE_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<Float32Array>())
        .ok_or_else(|| {
            format!(
                "rerank request column `{RERANK_REQUEST_VECTOR_SCORE_COLUMN}` must decode as Float32"
            )
        })?;
    for row_index in 0..batch.num_rows() {
        let score = vector_scores.value(row_index);
        if !score.is_finite() {
            return Err(format!(
                "rerank request column `{RERANK_REQUEST_VECTOR_SCORE_COLUMN}` must contain finite values; row {row_index} is {score}"
            ));
        }
        if !(0.0..=1.0).contains(&score) {
            return Err(format!(
                "rerank request column `{RERANK_REQUEST_VECTOR_SCORE_COLUMN}` must stay within inclusive range [0.0, 1.0]; row {row_index} is {score}"
            ));
        }
    }

    let query_embeddings = batch
        .column_by_name(RERANK_REQUEST_QUERY_EMBEDDING_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<FixedSizeListArray>())
        .ok_or_else(|| {
            format!(
                "rerank request column `{RERANK_REQUEST_QUERY_EMBEDDING_COLUMN}` must decode as FixedSizeList<Float32>"
            )
        })?;
    let first_query_embedding = fixed_size_list_row_values(query_embeddings, 0)?;
    for row_index in 1..batch.num_rows() {
        let row_query_embedding = fixed_size_list_row_values(query_embeddings, row_index)?;
        if row_query_embedding != first_query_embedding {
            return Err(format!(
                "rerank request column `{RERANK_REQUEST_QUERY_EMBEDDING_COLUMN}` must remain stable across all rows; row {row_index} differs from row 0"
            ));
        }
    }

    Ok(())
}

/// Score one validated rerank request batch with the shared Rust-owned rerank rule.
///
/// The current stable rule blends the inbound vector score with semantic cosine
/// similarity between `embedding` and `query_embedding`:
///
/// - `semantic_score = (cosine_similarity + 1.0) / 2.0`
/// - `final_score = 0.4 * vector_score + 0.6 * semantic_score`
///
/// # Errors
///
/// Returns an error when the request batch fails validation or when any
/// embedding/query vector has zero norm.
#[cfg(feature = "julia")]
pub fn score_rerank_request_batch(
    batch: &RecordBatch,
    expected_dimension: usize,
) -> Result<Vec<(String, f64)>, String> {
    validate_rerank_request_batch(batch, expected_dimension)?;

    let doc_ids = batch
        .column_by_name(RERANK_REQUEST_DOC_ID_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| {
            format!("rerank request column `{RERANK_REQUEST_DOC_ID_COLUMN}` must decode as Utf8")
        })?;
    let vector_scores = batch
        .column_by_name(RERANK_REQUEST_VECTOR_SCORE_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<Float32Array>())
        .ok_or_else(|| {
            format!(
                "rerank request column `{RERANK_REQUEST_VECTOR_SCORE_COLUMN}` must decode as Float32"
            )
        })?;
    let embeddings = batch
        .column_by_name(RERANK_REQUEST_EMBEDDING_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<FixedSizeListArray>())
        .ok_or_else(|| {
            format!(
                "rerank request column `{RERANK_REQUEST_EMBEDDING_COLUMN}` must decode as FixedSizeList<Float32>"
            )
        })?;
    let query_embeddings = batch
        .column_by_name(RERANK_REQUEST_QUERY_EMBEDDING_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<FixedSizeListArray>())
        .ok_or_else(|| {
            format!(
                "rerank request column `{RERANK_REQUEST_QUERY_EMBEDDING_COLUMN}` must decode as FixedSizeList<Float32>"
            )
        })?;

    let mut scored_candidates = Vec::with_capacity(batch.num_rows());
    for row_index in 0..batch.num_rows() {
        let embedding = fixed_size_list_row_values(embeddings, row_index)?;
        let query_embedding = fixed_size_list_row_values(query_embeddings, row_index)?;
        let cosine = cosine_similarity(&embedding, &query_embedding, row_index)?;
        let semantic_score = (cosine + 1.0) / 2.0;
        let final_score = 0.4 * f64::from(vector_scores.value(row_index)) + 0.6 * semantic_score;
        scored_candidates.push((doc_ids.value(row_index).to_string(), final_score));
    }

    Ok(scored_candidates)
}

/// Validate the stable rerank response schema.
///
/// # Errors
///
/// Returns an error when the rerank response schema does not match the
/// Rust-owned column set or Arrow types.
#[cfg(feature = "julia")]
pub fn validate_rerank_response_schema(schema: &Schema) -> Result<(), String> {
    let doc_id = schema
        .field_with_name(RERANK_RESPONSE_DOC_ID_COLUMN)
        .map_err(|_| format!("missing rerank response column `{RERANK_RESPONSE_DOC_ID_COLUMN}`"))?;
    if !matches!(doc_id.data_type(), DataType::Utf8) {
        return Err(format!(
            "rerank response column `{RERANK_RESPONSE_DOC_ID_COLUMN}` must be Utf8"
        ));
    }

    let final_score = schema
        .field_with_name(RERANK_RESPONSE_FINAL_SCORE_COLUMN)
        .map_err(|_| {
            format!("missing rerank response column `{RERANK_RESPONSE_FINAL_SCORE_COLUMN}`")
        })?;
    if !matches!(final_score.data_type(), DataType::Float64) {
        return Err(format!(
            "rerank response column `{RERANK_RESPONSE_FINAL_SCORE_COLUMN}` must be Float64"
        ));
    }

    let rank = schema
        .field_with_name(RERANK_RESPONSE_RANK_COLUMN)
        .map_err(|_| format!("missing rerank response column `{RERANK_RESPONSE_RANK_COLUMN}`"))?;
    if !matches!(rank.data_type(), DataType::Int32) {
        return Err(format!(
            "rerank response column `{RERANK_RESPONSE_RANK_COLUMN}` must be Int32"
        ));
    }

    Ok(())
}

/// Validate the stable rerank response payload semantics for one decoded batch.
///
/// # Errors
///
/// Returns an error when the rerank response batch is empty, contains blank or
/// duplicate document IDs, contains non-finite or out-of-range final scores,
/// or contains non-positive or duplicate rank values.
#[cfg(feature = "julia")]
pub fn validate_rerank_response_batch(batch: &RecordBatch) -> Result<(), String> {
    validate_rerank_response_schema(batch.schema().as_ref())?;
    if batch.num_rows() == 0 {
        return Err("rerank response batch must contain at least one row".to_string());
    }

    let doc_ids = batch
        .column_by_name(RERANK_RESPONSE_DOC_ID_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| {
            format!("rerank response column `{RERANK_RESPONSE_DOC_ID_COLUMN}` must decode as Utf8")
        })?;
    let mut seen_doc_ids = HashSet::new();
    for row_index in 0..batch.num_rows() {
        let doc_id = doc_ids.value(row_index).trim();
        if doc_id.is_empty() {
            return Err(format!(
                "rerank response column `{RERANK_RESPONSE_DOC_ID_COLUMN}` must not contain blank values; row {row_index} is blank"
            ));
        }
        if !seen_doc_ids.insert(doc_id.to_string()) {
            return Err(format!(
                "rerank response column `{RERANK_RESPONSE_DOC_ID_COLUMN}` must be unique across one batch; row {row_index} duplicates `{doc_id}`"
            ));
        }
    }

    let final_scores = batch
        .column_by_name(RERANK_RESPONSE_FINAL_SCORE_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<Float64Array>())
        .ok_or_else(|| {
            format!(
                "rerank response column `{RERANK_RESPONSE_FINAL_SCORE_COLUMN}` must decode as Float64"
            )
        })?;
    for row_index in 0..batch.num_rows() {
        let score = final_scores.value(row_index);
        if !score.is_finite() {
            return Err(format!(
                "rerank response column `{RERANK_RESPONSE_FINAL_SCORE_COLUMN}` must contain finite values; row {row_index} is {score}"
            ));
        }
        if !(0.0..=1.0).contains(&score) {
            return Err(format!(
                "rerank response column `{RERANK_RESPONSE_FINAL_SCORE_COLUMN}` must stay within inclusive range [0.0, 1.0]; row {row_index} is {score}"
            ));
        }
    }

    let ranks = batch
        .column_by_name(RERANK_RESPONSE_RANK_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<Int32Array>())
        .ok_or_else(|| {
            format!("rerank response column `{RERANK_RESPONSE_RANK_COLUMN}` must decode as Int32")
        })?;
    let mut seen_ranks = HashSet::new();
    for row_index in 0..batch.num_rows() {
        let rank = ranks.value(row_index);
        if rank <= 0 {
            return Err(format!(
                "rerank response column `{RERANK_RESPONSE_RANK_COLUMN}` must contain positive values; row {row_index} is {rank}"
            ));
        }
        if !seen_ranks.insert(rank) {
            return Err(format!(
                "rerank response column `{RERANK_RESPONSE_RANK_COLUMN}` must be unique across one batch; row {row_index} duplicates `{rank}`"
            ));
        }
    }

    Ok(())
}

#[cfg(feature = "julia")]
fn validate_embedding_field(
    schema: &Schema,
    field_name: &str,
    expected_dimension: usize,
) -> Result<(), String> {
    let field = schema
        .field_with_name(field_name)
        .map_err(|_| format!("missing rerank request column `{field_name}`"))?;
    match field.data_type() {
        DataType::FixedSizeList(inner_field, dimension)
            if matches!(inner_field.data_type(), DataType::Float32)
                && usize::try_from(*dimension).ok() == Some(expected_dimension) =>
        {
            Ok(())
        }
        DataType::FixedSizeList(inner_field, dimension)
            if matches!(inner_field.data_type(), DataType::Float32) =>
        {
            Err(format!(
                "rerank request column `{field_name}` must use dimension {expected_dimension}, got {dimension}"
            ))
        }
        _ => Err(format!(
            "rerank request column `{field_name}` must be FixedSizeList<Float32>"
        )),
    }
}

#[cfg(feature = "julia")]
fn fixed_size_list_row_values(
    array: &FixedSizeListArray,
    row_index: usize,
) -> Result<Vec<f32>, String> {
    let row = array.value(row_index);
    let values = row.as_any().downcast_ref::<Float32Array>().ok_or_else(|| {
        "rerank request fixed-size-list values must decode as Float32".to_string()
    })?;
    Ok((0..values.len()).map(|index| values.value(index)).collect())
}

#[cfg(feature = "julia")]
fn cosine_similarity(left: &[f32], right: &[f32], row_index: usize) -> Result<f64, String> {
    let left_norm = left
        .iter()
        .map(|value| f64::from(*value) * f64::from(*value))
        .sum::<f64>()
        .sqrt();
    if left_norm == 0.0 {
        return Err(format!(
            "rerank request column `{RERANK_REQUEST_EMBEDDING_COLUMN}` must not contain zero-norm vectors; row {row_index} is zero"
        ));
    }

    let right_norm = right
        .iter()
        .map(|value| f64::from(*value) * f64::from(*value))
        .sum::<f64>()
        .sqrt();
    if right_norm == 0.0 {
        return Err(format!(
            "rerank request column `{RERANK_REQUEST_QUERY_EMBEDDING_COLUMN}` must not contain zero-norm vectors; row {row_index} is zero"
        ));
    }

    let dot = left
        .iter()
        .zip(right.iter())
        .map(|(left_value, right_value)| f64::from(*left_value) * f64::from(*right_value))
        .sum::<f64>();
    Ok((dot / (left_norm * right_norm)).clamp(-1.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::{
        REPO_SEARCH_DEFAULT_LIMIT, REPO_SEARCH_DOC_ID_COLUMN, REPO_SEARCH_LANGUAGE_COLUMN,
        REPO_SEARCH_PATH_COLUMN, REPO_SEARCH_ROUTE, REPO_SEARCH_SCORE_COLUMN,
        REPO_SEARCH_TITLE_COLUMN, RERANK_EXCHANGE_ROUTE, RERANK_REQUEST_DOC_ID_COLUMN,
        RERANK_REQUEST_EMBEDDING_COLUMN, RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
        RERANK_REQUEST_VECTOR_SCORE_COLUMN, RERANK_RESPONSE_DOC_ID_COLUMN,
        RERANK_RESPONSE_FINAL_SCORE_COLUMN, RERANK_RESPONSE_RANK_COLUMN,
        WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER, WENDAO_REPO_SEARCH_LIMIT_HEADER,
        WENDAO_REPO_SEARCH_QUERY_HEADER, WENDAO_RERANK_DIMENSION_HEADER,
        WENDAO_SCHEMA_VERSION_HEADER, flight_descriptor_path, normalize_flight_route,
        validate_repo_search_request,
    };

    #[test]
    fn query_contract_exposes_stable_routes_and_header() {
        assert_eq!(WENDAO_SCHEMA_VERSION_HEADER, "x-wendao-schema-version");
        assert_eq!(
            WENDAO_REPO_SEARCH_QUERY_HEADER,
            "x-wendao-repo-search-query"
        );
        assert_eq!(
            WENDAO_REPO_SEARCH_LIMIT_HEADER,
            "x-wendao-repo-search-limit"
        );
        assert_eq!(
            WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER,
            "x-wendao-repo-search-language-filters"
        );
        assert_eq!(
            WENDAO_RERANK_DIMENSION_HEADER,
            "x-wendao-rerank-embedding-dimension"
        );
        assert_eq!(REPO_SEARCH_ROUTE, "/search/repos/main");
        assert_eq!(RERANK_EXCHANGE_ROUTE, "/rerank/flight");
        assert_eq!(REPO_SEARCH_DEFAULT_LIMIT, 10);
        assert_eq!(REPO_SEARCH_DOC_ID_COLUMN, "doc_id");
        assert_eq!(REPO_SEARCH_PATH_COLUMN, "path");
        assert_eq!(REPO_SEARCH_TITLE_COLUMN, "title");
        assert_eq!(REPO_SEARCH_SCORE_COLUMN, "score");
        assert_eq!(REPO_SEARCH_LANGUAGE_COLUMN, "language");
        assert_eq!(RERANK_REQUEST_DOC_ID_COLUMN, "doc_id");
        assert_eq!(RERANK_REQUEST_VECTOR_SCORE_COLUMN, "vector_score");
        assert_eq!(RERANK_REQUEST_EMBEDDING_COLUMN, "embedding");
        assert_eq!(RERANK_REQUEST_QUERY_EMBEDDING_COLUMN, "query_embedding");
        assert_eq!(RERANK_RESPONSE_DOC_ID_COLUMN, "doc_id");
        assert_eq!(RERANK_RESPONSE_FINAL_SCORE_COLUMN, "final_score");
        assert_eq!(RERANK_RESPONSE_RANK_COLUMN, "rank");
    }

    #[test]
    fn normalize_flight_route_enforces_canonical_leading_slash() {
        assert_eq!(
            normalize_flight_route("search/repos/main").as_deref(),
            Ok("/search/repos/main")
        );
        assert_eq!(
            normalize_flight_route("/rerank/flight").as_deref(),
            Ok("/rerank/flight")
        );
    }

    #[test]
    fn normalize_flight_route_rejects_empty_segments() {
        assert!(normalize_flight_route("").is_err());
        assert!(normalize_flight_route("/").is_err());
    }

    #[test]
    fn descriptor_path_matches_stable_query_route() {
        assert_eq!(
            flight_descriptor_path(REPO_SEARCH_ROUTE),
            Ok(vec![
                "search".to_string(),
                "repos".to_string(),
                "main".to_string()
            ])
        );
        assert_eq!(
            flight_descriptor_path(RERANK_EXCHANGE_ROUTE),
            Ok(vec!["rerank".to_string(), "flight".to_string()])
        );
    }

    #[test]
    fn repo_search_request_validation_accepts_stable_request() {
        assert!(
            validate_repo_search_request("rerank rust traits", 25, &[], &[], &[], &[], &[])
                .is_ok()
        );
    }

    #[test]
    fn repo_search_request_validation_rejects_blank_query_text() {
        assert_eq!(
            validate_repo_search_request(
                "   ",
                REPO_SEARCH_DEFAULT_LIMIT,
                &[],
                &[],
                &[],
                &[],
                &[],
            ),
            Err("repo search query text must not be blank".to_string())
        );
    }

    #[test]
    fn repo_search_request_validation_rejects_zero_limit() {
        assert_eq!(
            validate_repo_search_request("rerank rust traits", 0, &[], &[], &[], &[], &[]),
            Err("repo search limit must be greater than zero".to_string())
        );
    }

    #[test]
    fn repo_search_request_validation_rejects_blank_language_filters() {
        assert_eq!(
            validate_repo_search_request(
                "rerank rust traits",
                REPO_SEARCH_DEFAULT_LIMIT,
                &["rust".to_string(), "   ".to_string()],
                &[],
                &[],
                &[],
                &[],
            ),
            Err("repo search language filters must not contain blank values".to_string())
        );
    }

    #[test]
    fn repo_search_request_validation_rejects_blank_path_prefixes() {
        assert_eq!(
            validate_repo_search_request(
                "rerank rust traits",
                REPO_SEARCH_DEFAULT_LIMIT,
                &[],
                &["src/".to_string(), "   ".to_string()],
                &[],
                &[],
                &[],
            ),
            Err("repo search path prefixes must not contain blank values".to_string())
        );
    }

    #[test]
    fn repo_search_request_validation_rejects_blank_title_filters() {
        assert_eq!(
            validate_repo_search_request(
                "rerank rust traits",
                REPO_SEARCH_DEFAULT_LIMIT,
                &[],
                &[],
                &["README".to_string(), "   ".to_string()],
                &[],
                &[],
            ),
            Err("repo search title filters must not contain blank values".to_string())
        );
    }

    #[test]
    fn repo_search_request_validation_rejects_blank_tag_filters() {
        assert_eq!(
            validate_repo_search_request(
                "rerank rust traits",
                REPO_SEARCH_DEFAULT_LIMIT,
                &[],
                &[],
                &[],
                &["lang:rust".to_string(), "   ".to_string()],
                &[],
            ),
            Err("repo search tag filters must not contain blank values".to_string())
        );
    }

    #[test]
    fn repo_search_request_validation_rejects_blank_filename_filters() {
        assert_eq!(
            validate_repo_search_request(
                "rerank rust traits",
                REPO_SEARCH_DEFAULT_LIMIT,
                &[],
                &[],
                &[],
                &[],
                &["lib".to_string(), "   ".to_string()],
            ),
            Err("repo search filename filters must not contain blank values".to_string())
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_request_schema_validation_accepts_stable_shape() {
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
        ]);

        assert!(super::validate_rerank_request_schema(&schema, 3).is_ok());
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_request_schema_validation_rejects_wrong_scalar_type() {
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float64, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
        ]);

        assert_eq!(
            super::validate_rerank_request_schema(&schema, 3),
            Err("rerank request column `vector_score` must be Float32".to_string())
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_request_schema_validation_rejects_dimension_drift() {
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 2),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 2),
                false,
            ),
        ]);

        assert_eq!(
            super::validate_rerank_request_schema(&schema, 3),
            Err("rerank request column `embedding` must use dimension 3, got 2".to_string())
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_request_batch_validation_accepts_stable_semantics() {
        use arrow_array::types::Float32Type;
        use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["doc-1", "doc-2"])),
                Arc::new(Float32Array::from(vec![0.9_f32, 0.8_f32])),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        vec![
                            Some(vec![Some(0.1_f32), Some(0.2_f32), Some(0.3_f32)]),
                            Some(vec![Some(0.4_f32), Some(0.5_f32), Some(0.6_f32)]),
                        ],
                        3,
                    ),
                ),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        vec![
                            Some(vec![Some(0.7_f32), Some(0.8_f32), Some(0.9_f32)]),
                            Some(vec![Some(0.7_f32), Some(0.8_f32), Some(0.9_f32)]),
                        ],
                        3,
                    ),
                ),
            ],
        )
        .expect("record batch should build");

        assert!(super::validate_rerank_request_batch(&batch, 3).is_ok());
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_request_batch_validation_rejects_blank_doc_id() {
        use arrow_array::types::Float32Type;
        use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec![" "])),
                Arc::new(Float32Array::from(vec![0.9_f32])),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        vec![Some(vec![Some(0.1_f32), Some(0.2_f32), Some(0.3_f32)])],
                        3,
                    ),
                ),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        vec![Some(vec![Some(0.7_f32), Some(0.8_f32), Some(0.9_f32)])],
                        3,
                    ),
                ),
            ],
        )
        .expect("record batch should build");

        assert_eq!(
            super::validate_rerank_request_batch(&batch, 3),
            Err(
                "rerank request column `doc_id` must not contain blank values; row 0 is blank"
                    .to_string()
            )
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_request_batch_validation_rejects_duplicate_doc_id() {
        use arrow_array::types::Float32Type;
        use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["doc-1", "doc-1"])),
                Arc::new(Float32Array::from(vec![0.9_f32, 0.8_f32])),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        vec![
                            Some(vec![Some(0.1_f32), Some(0.2_f32), Some(0.3_f32)]),
                            Some(vec![Some(0.4_f32), Some(0.5_f32), Some(0.6_f32)]),
                        ],
                        3,
                    ),
                ),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        vec![
                            Some(vec![Some(0.7_f32), Some(0.8_f32), Some(0.9_f32)]),
                            Some(vec![Some(0.7_f32), Some(0.8_f32), Some(0.9_f32)]),
                        ],
                        3,
                    ),
                ),
            ],
        )
        .expect("record batch should build");

        assert_eq!(
            super::validate_rerank_request_batch(&batch, 3),
            Err(
                "rerank request column `doc_id` must be unique across one batch; row 1 duplicates `doc-1`"
                    .to_string()
            )
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_request_batch_validation_rejects_out_of_range_vector_score() {
        use arrow_array::types::Float32Type;
        use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["doc-1"])),
                Arc::new(Float32Array::from(vec![1.2_f32])),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        vec![Some(vec![Some(0.1_f32), Some(0.2_f32), Some(0.3_f32)])],
                        3,
                    ),
                ),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        vec![Some(vec![Some(0.7_f32), Some(0.8_f32), Some(0.9_f32)])],
                        3,
                    ),
                ),
            ],
        )
        .expect("record batch should build");

        assert_eq!(
            super::validate_rerank_request_batch(&batch, 3),
            Err(
                "rerank request column `vector_score` must stay within inclusive range [0.0, 1.0]; row 0 is 1.2"
                    .to_string()
            )
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_request_batch_validation_rejects_query_embedding_drift() {
        use arrow_array::types::Float32Type;
        use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["doc-1", "doc-2"])),
                Arc::new(Float32Array::from(vec![0.9_f32, 0.8_f32])),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        vec![
                            Some(vec![Some(0.1_f32), Some(0.2_f32), Some(0.3_f32)]),
                            Some(vec![Some(0.4_f32), Some(0.5_f32), Some(0.6_f32)]),
                        ],
                        3,
                    ),
                ),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        vec![
                            Some(vec![Some(0.7_f32), Some(0.8_f32), Some(0.9_f32)]),
                            Some(vec![Some(1.0_f32), Some(1.1_f32), Some(1.2_f32)]),
                        ],
                        3,
                    ),
                ),
            ],
        )
        .expect("record batch should build");

        assert_eq!(
            super::validate_rerank_request_batch(&batch, 3),
            Err(
                "rerank request column `query_embedding` must remain stable across all rows; row 1 differs from row 0"
                    .to_string()
            )
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_request_batch_scoring_blends_vector_and_semantic_similarity() {
        use arrow_array::types::Float32Type;
        use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["doc-0", "doc-1"])),
                Arc::new(Float32Array::from(vec![0.5_f32, 0.8_f32])),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        vec![
                            Some(vec![Some(1.0_f32), Some(0.0_f32), Some(0.0_f32)]),
                            Some(vec![Some(0.0_f32), Some(1.0_f32), Some(0.0_f32)]),
                        ],
                        3,
                    ),
                ),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        vec![
                            Some(vec![Some(1.0_f32), Some(0.0_f32), Some(0.0_f32)]),
                            Some(vec![Some(1.0_f32), Some(0.0_f32), Some(0.0_f32)]),
                        ],
                        3,
                    ),
                ),
            ],
        )
        .expect("record batch should build");

        let scored =
            super::score_rerank_request_batch(&batch, 3).expect("rerank scoring should succeed");

        assert_eq!(scored.len(), 2);
        assert_eq!(scored[0].0, "doc-0");
        assert!((scored[0].1 - 0.8).abs() < 1e-6);
        assert_eq!(scored[1].0, "doc-1");
        assert!((scored[1].1 - 0.62).abs() < 1e-6);
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_response_schema_validation_accepts_stable_shape() {
        use arrow_schema::{DataType, Field, Schema};

        let schema = Schema::new(vec![
            Field::new(RERANK_RESPONSE_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_RESPONSE_FINAL_SCORE_COLUMN, DataType::Float64, false),
            Field::new(RERANK_RESPONSE_RANK_COLUMN, DataType::Int32, false),
        ]);

        assert!(super::validate_rerank_response_schema(&schema).is_ok());
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_response_schema_validation_rejects_wrong_rank_type() {
        use arrow_schema::{DataType, Field, Schema};

        let schema = Schema::new(vec![
            Field::new(RERANK_RESPONSE_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_RESPONSE_FINAL_SCORE_COLUMN, DataType::Float64, false),
            Field::new(RERANK_RESPONSE_RANK_COLUMN, DataType::UInt32, false),
        ]);

        assert_eq!(
            super::validate_rerank_response_schema(&schema),
            Err("rerank response column `rank` must be Int32".to_string())
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_response_batch_validation_accepts_stable_semantics() {
        use arrow_array::{Float64Array, Int32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_RESPONSE_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_RESPONSE_FINAL_SCORE_COLUMN, DataType::Float64, false),
            Field::new(RERANK_RESPONSE_RANK_COLUMN, DataType::Int32, false),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["doc-1", "doc-2"])),
                Arc::new(Float64Array::from(vec![0.97_f64, 0.91_f64])),
                Arc::new(Int32Array::from(vec![1_i32, 2_i32])),
            ],
        )
        .expect("record batch should build");

        assert!(super::validate_rerank_response_batch(&batch).is_ok());
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_response_batch_validation_rejects_duplicate_rank() {
        use arrow_array::{Float64Array, Int32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_RESPONSE_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_RESPONSE_FINAL_SCORE_COLUMN, DataType::Float64, false),
            Field::new(RERANK_RESPONSE_RANK_COLUMN, DataType::Int32, false),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["doc-1", "doc-2"])),
                Arc::new(Float64Array::from(vec![0.97_f64, 0.91_f64])),
                Arc::new(Int32Array::from(vec![1_i32, 1_i32])),
            ],
        )
        .expect("record batch should build");

        assert_eq!(
            super::validate_rerank_response_batch(&batch),
            Err(
                "rerank response column `rank` must be unique across one batch; row 1 duplicates `1`"
                    .to_string()
            )
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_response_batch_validation_rejects_out_of_range_final_score() {
        use arrow_array::{Float64Array, Int32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_RESPONSE_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_RESPONSE_FINAL_SCORE_COLUMN, DataType::Float64, false),
            Field::new(RERANK_RESPONSE_RANK_COLUMN, DataType::Int32, false),
        ]));
        let batch = RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["doc-1"])),
                Arc::new(Float64Array::from(vec![1.2_f64])),
                Arc::new(Int32Array::from(vec![1_i32])),
            ],
        )
        .expect("record batch should build");

        assert_eq!(
            super::validate_rerank_response_batch(&batch),
            Err(
                "rerank response column `final_score` must stay within inclusive range [0.0, 1.0]; row 0 is 1.2"
                    .to_string()
            )
        );
    }
}
