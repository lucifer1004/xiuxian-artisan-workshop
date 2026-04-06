use std::collections::BTreeSet;

use arrow::array::{Array, BooleanArray, Float64Array, Int32Array, ListArray, StringArray};
use arrow::datatypes::{DataType, Schema};
use arrow::record_batch::RecordBatch;
use xiuxian_wendao_runtime::transport::normalize_flight_route;

/// Default schema version for the staged Julia graph-structural contract.
pub const JULIA_GRAPH_STRUCTURAL_SCHEMA_VERSION: &str = "v0-draft";

/// Stable route for the structural-rerank graph-search exchange contract.
pub const GRAPH_STRUCTURAL_RERANK_ROUTE: &str = "/graph/structural/rerank";
/// Stable route for the constraint-filter graph-search exchange contract.
pub const GRAPH_STRUCTURAL_FILTER_ROUTE: &str = "/graph/structural/filter";

/// Canonical graph-structural request `query_id` column.
pub const GRAPH_STRUCTURAL_QUERY_ID_COLUMN: &str = "query_id";
/// Canonical graph-structural request or response `candidate_id` column.
pub const GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN: &str = "candidate_id";
/// Canonical graph-structural request `retrieval_layer` column.
pub const GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN: &str = "retrieval_layer";
/// Canonical graph-structural request `query_max_layers` column.
pub const GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN: &str = "query_max_layers";
/// Canonical structural-rerank request `semantic_score` column.
pub const GRAPH_STRUCTURAL_SEMANTIC_SCORE_COLUMN: &str = "semantic_score";
/// Canonical structural-rerank request `dependency_score` column.
pub const GRAPH_STRUCTURAL_DEPENDENCY_SCORE_COLUMN: &str = "dependency_score";
/// Canonical structural-rerank request `keyword_score` column.
pub const GRAPH_STRUCTURAL_KEYWORD_SCORE_COLUMN: &str = "keyword_score";
/// Canonical structural-rerank request `tag_score` column.
pub const GRAPH_STRUCTURAL_TAG_SCORE_COLUMN: &str = "tag_score";
/// Canonical graph-structural request `constraint_kind` column.
pub const GRAPH_STRUCTURAL_CONSTRAINT_KIND_COLUMN: &str = "constraint_kind";
/// Canonical graph-structural request `required_boundary_size` column.
pub const GRAPH_STRUCTURAL_REQUIRED_BOUNDARY_SIZE_COLUMN: &str = "required_boundary_size";
/// Canonical graph-structural request `anchor_planes` column.
pub const GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN: &str = "anchor_planes";
/// Canonical graph-structural request `anchor_values` column.
pub const GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN: &str = "anchor_values";
/// Canonical graph-structural request `edge_constraint_kinds` column.
pub const GRAPH_STRUCTURAL_EDGE_CONSTRAINT_KINDS_COLUMN: &str = "edge_constraint_kinds";
/// Canonical graph-structural request `candidate_node_ids` column.
pub const GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN: &str = "candidate_node_ids";
/// Canonical graph-structural request `candidate_edge_sources` column.
pub const GRAPH_STRUCTURAL_CANDIDATE_EDGE_SOURCES_COLUMN: &str = "candidate_edge_sources";
/// Canonical graph-structural request `candidate_edge_destinations` column.
pub const GRAPH_STRUCTURAL_CANDIDATE_EDGE_DESTINATIONS_COLUMN: &str = "candidate_edge_destinations";
/// Canonical graph-structural request `candidate_edge_kinds` column.
pub const GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN: &str = "candidate_edge_kinds";
/// Canonical structural-rerank response `feasible` column.
pub const GRAPH_STRUCTURAL_FEASIBLE_COLUMN: &str = "feasible";
/// Canonical constraint-filter response `accepted` column.
pub const GRAPH_STRUCTURAL_ACCEPTED_COLUMN: &str = "accepted";
/// Canonical graph-structural response `structural_score` column.
pub const GRAPH_STRUCTURAL_STRUCTURAL_SCORE_COLUMN: &str = "structural_score";
/// Canonical structural-rerank response `final_score` column.
pub const GRAPH_STRUCTURAL_FINAL_SCORE_COLUMN: &str = "final_score";
/// Canonical graph-structural response `pin_assignment` column.
pub const GRAPH_STRUCTURAL_PIN_ASSIGNMENT_COLUMN: &str = "pin_assignment";
/// Canonical structural-rerank response `explanation` column.
pub const GRAPH_STRUCTURAL_EXPLANATION_COLUMN: &str = "explanation";
/// Canonical constraint-filter response `rejection_reason` column.
pub const GRAPH_STRUCTURAL_REJECTION_REASON_COLUMN: &str = "rejection_reason";

/// Canonical structural-rerank request column order.
pub const GRAPH_STRUCTURAL_RERANK_REQUEST_COLUMNS: [&str; 15] = [
    GRAPH_STRUCTURAL_QUERY_ID_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN,
    GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN,
    GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN,
    GRAPH_STRUCTURAL_SEMANTIC_SCORE_COLUMN,
    GRAPH_STRUCTURAL_DEPENDENCY_SCORE_COLUMN,
    GRAPH_STRUCTURAL_KEYWORD_SCORE_COLUMN,
    GRAPH_STRUCTURAL_TAG_SCORE_COLUMN,
    GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN,
    GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN,
    GRAPH_STRUCTURAL_EDGE_CONSTRAINT_KINDS_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_EDGE_SOURCES_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_EDGE_DESTINATIONS_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN,
];

/// Canonical structural-rerank response column order.
pub const GRAPH_STRUCTURAL_RERANK_RESPONSE_COLUMNS: [&str; 6] = [
    GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN,
    GRAPH_STRUCTURAL_FEASIBLE_COLUMN,
    GRAPH_STRUCTURAL_STRUCTURAL_SCORE_COLUMN,
    GRAPH_STRUCTURAL_FINAL_SCORE_COLUMN,
    GRAPH_STRUCTURAL_PIN_ASSIGNMENT_COLUMN,
    GRAPH_STRUCTURAL_EXPLANATION_COLUMN,
];

/// Canonical constraint-filter request column order.
pub const GRAPH_STRUCTURAL_FILTER_REQUEST_COLUMNS: [&str; 13] = [
    GRAPH_STRUCTURAL_QUERY_ID_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN,
    GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN,
    GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN,
    GRAPH_STRUCTURAL_CONSTRAINT_KIND_COLUMN,
    GRAPH_STRUCTURAL_REQUIRED_BOUNDARY_SIZE_COLUMN,
    GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN,
    GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN,
    GRAPH_STRUCTURAL_EDGE_CONSTRAINT_KINDS_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_EDGE_SOURCES_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_EDGE_DESTINATIONS_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN,
];

/// Canonical constraint-filter response column order.
pub const GRAPH_STRUCTURAL_FILTER_RESPONSE_COLUMNS: [&str; 5] = [
    GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN,
    GRAPH_STRUCTURAL_ACCEPTED_COLUMN,
    GRAPH_STRUCTURAL_STRUCTURAL_SCORE_COLUMN,
    GRAPH_STRUCTURAL_PIN_ASSIGNMENT_COLUMN,
    GRAPH_STRUCTURAL_REJECTION_REASON_COLUMN,
];

/// Stable graph-structural exchange route kind owned by the Julia plugin crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphStructuralRouteKind {
    /// Soft-score structural rerank lane.
    StructuralRerank,
    /// Hard-gate constraint-filter lane.
    ConstraintFilter,
}

impl GraphStructuralRouteKind {
    /// Return the canonical route path for this graph-structural exchange kind.
    #[must_use]
    pub fn route(self) -> &'static str {
        match self {
            Self::StructuralRerank => GRAPH_STRUCTURAL_RERANK_ROUTE,
            Self::ConstraintFilter => GRAPH_STRUCTURAL_FILTER_ROUTE,
        }
    }

    /// Return the staged schema version for this graph-structural exchange kind.
    #[must_use]
    pub fn schema_version(self) -> &'static str {
        match self {
            Self::StructuralRerank | Self::ConstraintFilter => {
                JULIA_GRAPH_STRUCTURAL_SCHEMA_VERSION
            }
        }
    }

    /// Return the capability-manifest variant tag for this exchange kind.
    #[must_use]
    pub fn capability_variant(self) -> &'static str {
        match self {
            Self::StructuralRerank => "structural_rerank",
            Self::ConstraintFilter => "constraint_filter",
        }
    }

    /// Return the canonical request columns for this graph-structural exchange kind.
    #[must_use]
    pub fn request_columns(self) -> &'static [&'static str] {
        match self {
            Self::StructuralRerank => &GRAPH_STRUCTURAL_RERANK_REQUEST_COLUMNS,
            Self::ConstraintFilter => &GRAPH_STRUCTURAL_FILTER_REQUEST_COLUMNS,
        }
    }

    /// Return the canonical response columns for this graph-structural exchange kind.
    #[must_use]
    pub fn response_columns(self) -> &'static [&'static str] {
        match self {
            Self::StructuralRerank => &GRAPH_STRUCTURAL_RERANK_RESPONSE_COLUMNS,
            Self::ConstraintFilter => &GRAPH_STRUCTURAL_FILTER_RESPONSE_COLUMNS,
        }
    }
}

/// Resolve one route into the staged graph-structural exchange kind.
///
/// # Errors
///
/// Returns an error when the route does not normalize into one of the staged
/// graph-structural exchange paths.
pub fn graph_structural_route_kind(
    route: impl AsRef<str>,
) -> Result<GraphStructuralRouteKind, String> {
    let normalized = normalize_flight_route(route)?;
    match normalized.as_str() {
        GRAPH_STRUCTURAL_RERANK_ROUTE => Ok(GraphStructuralRouteKind::StructuralRerank),
        GRAPH_STRUCTURAL_FILTER_ROUTE => Ok(GraphStructuralRouteKind::ConstraintFilter),
        _ => Err(format!(
            "unsupported graph-structural Flight route `{normalized}`"
        )),
    }
}

/// Return whether one route belongs to the staged graph-structural exchange family.
#[must_use]
pub fn is_graph_structural_route(route: impl AsRef<str>) -> bool {
    graph_structural_route_kind(route).is_ok()
}

/// Validate the staged structural-rerank request schema.
///
/// # Errors
///
/// Returns an error when the schema does not match the staged structural-rerank
/// request contract.
pub fn validate_graph_structural_rerank_request_schema(schema: &Schema) -> Result<(), String> {
    validate_utf8_field(schema, GRAPH_STRUCTURAL_QUERY_ID_COLUMN)?;
    validate_utf8_field(schema, GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN)?;
    validate_int32_field(schema, GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN)?;
    validate_int32_field(schema, GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN)?;
    validate_float64_field(schema, GRAPH_STRUCTURAL_SEMANTIC_SCORE_COLUMN)?;
    validate_float64_field(schema, GRAPH_STRUCTURAL_DEPENDENCY_SCORE_COLUMN)?;
    validate_float64_field(schema, GRAPH_STRUCTURAL_KEYWORD_SCORE_COLUMN)?;
    validate_float64_field(schema, GRAPH_STRUCTURAL_TAG_SCORE_COLUMN)?;
    validate_list_utf8_field(schema, GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN)?;
    validate_list_utf8_field(schema, GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN)?;
    validate_list_utf8_field(schema, GRAPH_STRUCTURAL_EDGE_CONSTRAINT_KINDS_COLUMN)?;
    validate_list_utf8_field(schema, GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN)?;
    validate_list_utf8_field(schema, GRAPH_STRUCTURAL_CANDIDATE_EDGE_SOURCES_COLUMN)?;
    validate_list_utf8_field(schema, GRAPH_STRUCTURAL_CANDIDATE_EDGE_DESTINATIONS_COLUMN)?;
    validate_list_utf8_field(schema, GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN)?;
    Ok(())
}

/// Validate one staged structural-rerank request batch.
///
/// # Errors
///
/// Returns an error when the batch does not satisfy the staged structural-rerank
/// request semantics.
pub fn validate_graph_structural_rerank_request_batch(batch: &RecordBatch) -> Result<(), String> {
    validate_graph_structural_rerank_request_schema(batch.schema().as_ref())?;
    if batch.num_rows() == 0 {
        return Err(
            "graph structural rerank request batch must contain at least one row".to_string(),
        );
    }
    require_non_blank_utf8_column(batch, GRAPH_STRUCTURAL_QUERY_ID_COLUMN, false)?;
    require_non_blank_utf8_column(batch, GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN, true)?;
    require_int32_column(batch, GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN, 0)?;
    require_int32_column(batch, GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN, 1)?;
    require_non_negative_float64_column(batch, GRAPH_STRUCTURAL_SEMANTIC_SCORE_COLUMN)?;
    require_non_negative_float64_column(batch, GRAPH_STRUCTURAL_DEPENDENCY_SCORE_COLUMN)?;
    require_non_negative_float64_column(batch, GRAPH_STRUCTURAL_KEYWORD_SCORE_COLUMN)?;
    require_non_negative_float64_column(batch, GRAPH_STRUCTURAL_TAG_SCORE_COLUMN)?;
    let anchor_planes =
        require_utf8_list_column(batch, GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN, false)?;
    let anchor_values =
        require_utf8_list_column(batch, GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN, false)?;
    require_utf8_list_column(batch, GRAPH_STRUCTURAL_EDGE_CONSTRAINT_KINDS_COLUMN, true)?;
    let candidate_node_ids =
        require_utf8_list_column(batch, GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN, false)?;
    let candidate_edge_sources =
        require_utf8_list_column(batch, GRAPH_STRUCTURAL_CANDIDATE_EDGE_SOURCES_COLUMN, true)?;
    let candidate_edge_destinations = require_utf8_list_column(
        batch,
        GRAPH_STRUCTURAL_CANDIDATE_EDGE_DESTINATIONS_COLUMN,
        true,
    )?;
    let candidate_edge_kinds =
        require_utf8_list_column(batch, GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN, true)?;
    for row_index in 0..batch.num_rows() {
        if anchor_planes[row_index].len() != anchor_values[row_index].len() {
            return Err(format!(
                "graph structural rerank request anchor columns must stay aligned; row {row_index} has {} planes but {} values",
                anchor_planes[row_index].len(),
                anchor_values[row_index].len(),
            ));
        }
        validate_candidate_edge_lists(
            "graph structural rerank request",
            row_index,
            &candidate_node_ids[row_index],
            &candidate_edge_sources[row_index],
            &candidate_edge_destinations[row_index],
            &candidate_edge_kinds[row_index],
        )?;
    }
    Ok(())
}

/// Validate the staged structural-rerank response schema.
///
/// # Errors
///
/// Returns an error when the schema does not match the staged structural-rerank
/// response contract.
pub fn validate_graph_structural_rerank_response_schema(schema: &Schema) -> Result<(), String> {
    validate_utf8_field(schema, GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN)?;
    validate_bool_field(schema, GRAPH_STRUCTURAL_FEASIBLE_COLUMN)?;
    validate_float64_field(schema, GRAPH_STRUCTURAL_STRUCTURAL_SCORE_COLUMN)?;
    validate_float64_field(schema, GRAPH_STRUCTURAL_FINAL_SCORE_COLUMN)?;
    validate_list_utf8_field(schema, GRAPH_STRUCTURAL_PIN_ASSIGNMENT_COLUMN)?;
    validate_utf8_field(schema, GRAPH_STRUCTURAL_EXPLANATION_COLUMN)?;
    Ok(())
}

/// Validate one staged structural-rerank response batch.
///
/// # Errors
///
/// Returns an error when the batch does not satisfy the staged structural-rerank
/// response semantics.
pub fn validate_graph_structural_rerank_response_batch(batch: &RecordBatch) -> Result<(), String> {
    validate_graph_structural_rerank_response_schema(batch.schema().as_ref())?;
    require_non_blank_utf8_column(batch, GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN, true)?;
    require_bool_column(batch, GRAPH_STRUCTURAL_FEASIBLE_COLUMN)?;
    require_finite_float64_column(batch, GRAPH_STRUCTURAL_STRUCTURAL_SCORE_COLUMN)?;
    require_finite_float64_column(batch, GRAPH_STRUCTURAL_FINAL_SCORE_COLUMN)?;
    require_utf8_list_column(batch, GRAPH_STRUCTURAL_PIN_ASSIGNMENT_COLUMN, true)?;
    require_utf8_column(batch, GRAPH_STRUCTURAL_EXPLANATION_COLUMN, true)?;
    Ok(())
}

/// Validate the staged constraint-filter request schema.
///
/// # Errors
///
/// Returns an error when the schema does not match the staged constraint-filter
/// request contract.
pub fn validate_graph_structural_filter_request_schema(schema: &Schema) -> Result<(), String> {
    validate_utf8_field(schema, GRAPH_STRUCTURAL_QUERY_ID_COLUMN)?;
    validate_utf8_field(schema, GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN)?;
    validate_int32_field(schema, GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN)?;
    validate_int32_field(schema, GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN)?;
    validate_utf8_field(schema, GRAPH_STRUCTURAL_CONSTRAINT_KIND_COLUMN)?;
    validate_int32_field(schema, GRAPH_STRUCTURAL_REQUIRED_BOUNDARY_SIZE_COLUMN)?;
    validate_list_utf8_field(schema, GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN)?;
    validate_list_utf8_field(schema, GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN)?;
    validate_list_utf8_field(schema, GRAPH_STRUCTURAL_EDGE_CONSTRAINT_KINDS_COLUMN)?;
    validate_list_utf8_field(schema, GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN)?;
    validate_list_utf8_field(schema, GRAPH_STRUCTURAL_CANDIDATE_EDGE_SOURCES_COLUMN)?;
    validate_list_utf8_field(schema, GRAPH_STRUCTURAL_CANDIDATE_EDGE_DESTINATIONS_COLUMN)?;
    validate_list_utf8_field(schema, GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN)?;
    Ok(())
}

/// Validate one staged constraint-filter request batch.
///
/// # Errors
///
/// Returns an error when the batch does not satisfy the staged constraint-filter
/// request semantics.
pub fn validate_graph_structural_filter_request_batch(batch: &RecordBatch) -> Result<(), String> {
    validate_graph_structural_filter_request_schema(batch.schema().as_ref())?;
    if batch.num_rows() == 0 {
        return Err(
            "graph structural filter request batch must contain at least one row".to_string(),
        );
    }
    require_non_blank_utf8_column(batch, GRAPH_STRUCTURAL_QUERY_ID_COLUMN, false)?;
    require_non_blank_utf8_column(batch, GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN, true)?;
    require_int32_column(batch, GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN, 0)?;
    require_int32_column(batch, GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN, 1)?;
    require_non_blank_utf8_column(batch, GRAPH_STRUCTURAL_CONSTRAINT_KIND_COLUMN, false)?;
    require_int32_column(batch, GRAPH_STRUCTURAL_REQUIRED_BOUNDARY_SIZE_COLUMN, 0)?;
    let anchor_planes =
        require_utf8_list_column(batch, GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN, false)?;
    let anchor_values =
        require_utf8_list_column(batch, GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN, false)?;
    require_utf8_list_column(batch, GRAPH_STRUCTURAL_EDGE_CONSTRAINT_KINDS_COLUMN, true)?;
    let candidate_node_ids =
        require_utf8_list_column(batch, GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN, false)?;
    let candidate_edge_sources =
        require_utf8_list_column(batch, GRAPH_STRUCTURAL_CANDIDATE_EDGE_SOURCES_COLUMN, true)?;
    let candidate_edge_destinations = require_utf8_list_column(
        batch,
        GRAPH_STRUCTURAL_CANDIDATE_EDGE_DESTINATIONS_COLUMN,
        true,
    )?;
    let candidate_edge_kinds =
        require_utf8_list_column(batch, GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN, true)?;
    for row_index in 0..batch.num_rows() {
        if anchor_planes[row_index].len() != anchor_values[row_index].len() {
            return Err(format!(
                "graph structural filter request anchor columns must stay aligned; row {row_index} has {} planes but {} values",
                anchor_planes[row_index].len(),
                anchor_values[row_index].len(),
            ));
        }
        validate_candidate_edge_lists(
            "graph structural filter request",
            row_index,
            &candidate_node_ids[row_index],
            &candidate_edge_sources[row_index],
            &candidate_edge_destinations[row_index],
            &candidate_edge_kinds[row_index],
        )?;
    }
    Ok(())
}

/// Validate the staged constraint-filter response schema.
///
/// # Errors
///
/// Returns an error when the schema does not match the staged constraint-filter
/// response contract.
pub fn validate_graph_structural_filter_response_schema(schema: &Schema) -> Result<(), String> {
    validate_utf8_field(schema, GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN)?;
    validate_bool_field(schema, GRAPH_STRUCTURAL_ACCEPTED_COLUMN)?;
    validate_float64_field(schema, GRAPH_STRUCTURAL_STRUCTURAL_SCORE_COLUMN)?;
    validate_list_utf8_field(schema, GRAPH_STRUCTURAL_PIN_ASSIGNMENT_COLUMN)?;
    validate_utf8_field(schema, GRAPH_STRUCTURAL_REJECTION_REASON_COLUMN)?;
    Ok(())
}

/// Validate one staged constraint-filter response batch.
///
/// # Errors
///
/// Returns an error when the batch does not satisfy the staged constraint-filter
/// response semantics.
pub fn validate_graph_structural_filter_response_batch(batch: &RecordBatch) -> Result<(), String> {
    validate_graph_structural_filter_response_schema(batch.schema().as_ref())?;
    let candidate_ids =
        require_non_blank_utf8_column(batch, GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN, true)?;
    let accepted = require_bool_column(batch, GRAPH_STRUCTURAL_ACCEPTED_COLUMN)?;
    require_non_negative_float64_column(batch, GRAPH_STRUCTURAL_STRUCTURAL_SCORE_COLUMN)?;
    require_utf8_list_column(batch, GRAPH_STRUCTURAL_PIN_ASSIGNMENT_COLUMN, true)?;
    let rejection_reason =
        require_utf8_column(batch, GRAPH_STRUCTURAL_REJECTION_REASON_COLUMN, true)?;
    for row_index in 0..batch.num_rows() {
        if accepted.value(row_index) {
            if !rejection_reason.value(row_index).trim().is_empty() {
                return Err(format!(
                    "graph structural filter response column `{GRAPH_STRUCTURAL_REJECTION_REASON_COLUMN}` must be blank for accepted candidate `{}` at row {row_index}",
                    candidate_ids.value(row_index),
                ));
            }
        } else if rejection_reason.value(row_index).trim().is_empty() {
            return Err(format!(
                "graph structural filter response column `{GRAPH_STRUCTURAL_REJECTION_REASON_COLUMN}` must be non-blank for rejected candidate `{}` at row {row_index}",
                candidate_ids.value(row_index),
            ));
        }
    }
    Ok(())
}

fn validate_utf8_field(schema: &Schema, field_name: &str) -> Result<(), String> {
    let field = schema
        .field_with_name(field_name)
        .map_err(|_| format!("missing graph structural column `{field_name}`"))?;
    if field.data_type() != &DataType::Utf8 {
        return Err(format!(
            "graph structural column `{field_name}` must be Utf8"
        ));
    }
    Ok(())
}

fn validate_bool_field(schema: &Schema, field_name: &str) -> Result<(), String> {
    let field = schema
        .field_with_name(field_name)
        .map_err(|_| format!("missing graph structural column `{field_name}`"))?;
    if field.data_type() != &DataType::Boolean {
        return Err(format!(
            "graph structural column `{field_name}` must be Boolean"
        ));
    }
    Ok(())
}

fn validate_int32_field(schema: &Schema, field_name: &str) -> Result<(), String> {
    let field = schema
        .field_with_name(field_name)
        .map_err(|_| format!("missing graph structural column `{field_name}`"))?;
    if field.data_type() != &DataType::Int32 {
        return Err(format!(
            "graph structural column `{field_name}` must be Int32"
        ));
    }
    Ok(())
}

fn validate_float64_field(schema: &Schema, field_name: &str) -> Result<(), String> {
    let field = schema
        .field_with_name(field_name)
        .map_err(|_| format!("missing graph structural column `{field_name}`"))?;
    if field.data_type() != &DataType::Float64 {
        return Err(format!(
            "graph structural column `{field_name}` must be Float64"
        ));
    }
    Ok(())
}

fn validate_list_utf8_field(schema: &Schema, field_name: &str) -> Result<(), String> {
    let field = schema
        .field_with_name(field_name)
        .map_err(|_| format!("missing graph structural column `{field_name}`"))?;
    match field.data_type() {
        DataType::List(inner) if inner.data_type() == &DataType::Utf8 => Ok(()),
        _ => Err(format!(
            "graph structural column `{field_name}` must be List<Utf8>"
        )),
    }
}

fn require_non_blank_utf8_column<'a>(
    batch: &'a RecordBatch,
    field_name: &str,
    unique: bool,
) -> Result<&'a StringArray, String> {
    let column = utf8_column(batch, field_name)?;
    let mut seen = BTreeSet::new();
    for row_index in 0..batch.num_rows() {
        if column.is_null(row_index) {
            return Err(format!(
                "graph structural column `{field_name}` must not contain null values; row {row_index} is null"
            ));
        }
        let value = column.value(row_index).trim();
        if value.is_empty() {
            return Err(format!(
                "graph structural column `{field_name}` must not contain blank values; row {row_index} is blank"
            ));
        }
        if unique && !seen.insert(value.to_string()) {
            return Err(format!(
                "graph structural column `{field_name}` must be unique across one batch; row {row_index} duplicates `{value}`"
            ));
        }
    }
    Ok(column)
}

fn require_utf8_column<'a>(
    batch: &'a RecordBatch,
    field_name: &str,
    allow_blank: bool,
) -> Result<&'a StringArray, String> {
    let column = utf8_column(batch, field_name)?;
    for row_index in 0..batch.num_rows() {
        if column.is_null(row_index) {
            return Err(format!(
                "graph structural column `{field_name}` must not contain null values; row {row_index} is null"
            ));
        }
        if !allow_blank && column.value(row_index).trim().is_empty() {
            return Err(format!(
                "graph structural column `{field_name}` must not contain blank values; row {row_index} is blank"
            ));
        }
    }
    Ok(column)
}

fn require_bool_column<'a>(
    batch: &'a RecordBatch,
    field_name: &str,
) -> Result<&'a BooleanArray, String> {
    let column = bool_column(batch, field_name)?;
    for row_index in 0..batch.num_rows() {
        if column.is_null(row_index) {
            return Err(format!(
                "graph structural column `{field_name}` must not contain null values; row {row_index} is null"
            ));
        }
    }
    Ok(column)
}

fn require_int32_column(
    batch: &RecordBatch,
    field_name: &str,
    min_value: i32,
) -> Result<(), String> {
    let column = int32_column(batch, field_name)?;
    for row_index in 0..batch.num_rows() {
        if column.is_null(row_index) {
            return Err(format!(
                "graph structural column `{field_name}` must not contain null values; row {row_index} is null"
            ));
        }
        let value = column.value(row_index);
        if value < min_value {
            return Err(format!(
                "graph structural column `{field_name}` must be greater than or equal to {min_value}; row {row_index} is {value}"
            ));
        }
    }
    Ok(())
}

fn require_non_negative_float64_column(
    batch: &RecordBatch,
    field_name: &str,
) -> Result<(), String> {
    let column = float64_column(batch, field_name)?;
    for row_index in 0..batch.num_rows() {
        if column.is_null(row_index) {
            return Err(format!(
                "graph structural column `{field_name}` must not contain null values; row {row_index} is null"
            ));
        }
        let value = column.value(row_index);
        if !value.is_finite() {
            return Err(format!(
                "graph structural column `{field_name}` must contain finite values; row {row_index} is {value}"
            ));
        }
        if value < 0.0 {
            return Err(format!(
                "graph structural column `{field_name}` must be greater than or equal to 0.0; row {row_index} is {value}"
            ));
        }
    }
    Ok(())
}

fn require_finite_float64_column(batch: &RecordBatch, field_name: &str) -> Result<(), String> {
    let column = float64_column(batch, field_name)?;
    for row_index in 0..batch.num_rows() {
        if column.is_null(row_index) {
            return Err(format!(
                "graph structural column `{field_name}` must not contain null values; row {row_index} is null"
            ));
        }
        let value = column.value(row_index);
        if !value.is_finite() {
            return Err(format!(
                "graph structural column `{field_name}` must contain finite values; row {row_index} is {value}"
            ));
        }
    }
    Ok(())
}

fn require_utf8_list_column(
    batch: &RecordBatch,
    field_name: &str,
    allow_empty_lists: bool,
) -> Result<Vec<Vec<String>>, String> {
    let column = list_utf8_column(batch, field_name)?;
    let mut rows = Vec::with_capacity(batch.num_rows());
    for row_index in 0..batch.num_rows() {
        if column.is_null(row_index) {
            return Err(format!(
                "graph structural column `{field_name}` must not contain null lists; row {row_index} is null"
            ));
        }
        let values = column.value(row_index);
        let strings = values
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| {
                format!("graph structural column `{field_name}` must decode as List<Utf8>")
            })?;
        let mut items = Vec::with_capacity(strings.len());
        for value_index in 0..strings.len() {
            if strings.is_null(value_index) {
                return Err(format!(
                    "graph structural column `{field_name}` must not contain null string values; row {row_index} item {value_index} is null"
                ));
            }
            let value = strings.value(value_index).trim();
            if value.is_empty() {
                return Err(format!(
                    "graph structural column `{field_name}` must not contain blank string values; row {row_index} item {value_index} is blank"
                ));
            }
            items.push(value.to_string());
        }
        if !allow_empty_lists && items.is_empty() {
            return Err(format!(
                "graph structural column `{field_name}` must contain at least one item; row {row_index} is empty"
            ));
        }
        rows.push(items);
    }
    Ok(rows)
}

fn validate_candidate_edge_lists(
    subject: &str,
    row_index: usize,
    node_ids: &[String],
    edge_sources: &[String],
    edge_destinations: &[String],
    edge_kinds: &[String],
) -> Result<(), String> {
    if edge_sources.len() != edge_destinations.len() {
        return Err(format!(
            "{subject} edge endpoint columns must stay aligned; row {row_index} has {} sources but {} destinations",
            edge_sources.len(),
            edge_destinations.len(),
        ));
    }
    if edge_sources.len() != edge_kinds.len() {
        return Err(format!(
            "{subject} edge columns must align with edge kinds; row {row_index} has {} edge endpoints but {} edge kinds",
            edge_sources.len(),
            edge_kinds.len(),
        ));
    }
    let node_ids = node_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
    for (edge_index, (src_id, dst_id)) in edge_sources
        .iter()
        .zip(edge_destinations.iter())
        .enumerate()
    {
        if src_id == dst_id {
            return Err(format!(
                "{subject} edge endpoints must not be identical; row {row_index} edge {edge_index} repeats `{src_id}`",
            ));
        }
        if !node_ids.contains(src_id.as_str()) {
            return Err(format!(
                "{subject} edge source `{src_id}` is not present in candidate nodes at row {row_index}",
            ));
        }
        if !node_ids.contains(dst_id.as_str()) {
            return Err(format!(
                "{subject} edge destination `{dst_id}` is not present in candidate nodes at row {row_index}",
            ));
        }
    }
    Ok(())
}

fn utf8_column<'a>(batch: &'a RecordBatch, field_name: &str) -> Result<&'a StringArray, String> {
    batch
        .column_by_name(field_name)
        .ok_or_else(|| format!("missing graph structural column `{field_name}`"))?
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| format!("graph structural column `{field_name}` must decode as Utf8"))
}

fn bool_column<'a>(batch: &'a RecordBatch, field_name: &str) -> Result<&'a BooleanArray, String> {
    batch
        .column_by_name(field_name)
        .ok_or_else(|| format!("missing graph structural column `{field_name}`"))?
        .as_any()
        .downcast_ref::<BooleanArray>()
        .ok_or_else(|| format!("graph structural column `{field_name}` must decode as Boolean"))
}

fn int32_column<'a>(batch: &'a RecordBatch, field_name: &str) -> Result<&'a Int32Array, String> {
    batch
        .column_by_name(field_name)
        .ok_or_else(|| format!("missing graph structural column `{field_name}`"))?
        .as_any()
        .downcast_ref::<Int32Array>()
        .ok_or_else(|| format!("graph structural column `{field_name}` must decode as Int32"))
}

fn float64_column<'a>(
    batch: &'a RecordBatch,
    field_name: &str,
) -> Result<&'a Float64Array, String> {
    batch
        .column_by_name(field_name)
        .ok_or_else(|| format!("missing graph structural column `{field_name}`"))?
        .as_any()
        .downcast_ref::<Float64Array>()
        .ok_or_else(|| format!("graph structural column `{field_name}` must decode as Float64"))
}

fn list_utf8_column<'a>(batch: &'a RecordBatch, field_name: &str) -> Result<&'a ListArray, String> {
    batch
        .column_by_name(field_name)
        .ok_or_else(|| format!("missing graph structural column `{field_name}`"))?
        .as_any()
        .downcast_ref::<ListArray>()
        .ok_or_else(|| format!("graph structural column `{field_name}` must decode as List<Utf8>"))
}

#[cfg(test)]
mod tests {
    use super::*;

    use arrow::array::{
        ArrayRef, BooleanArray, Float64Array, Int32Array, ListArray, StringArray,
        builder::{ListBuilder, StringBuilder},
    };
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;
    use std::sync::Arc;

    #[test]
    fn graph_structural_route_staging_resolves_canonical_paths() {
        assert_eq!(
            graph_structural_route_kind("graph/structural/rerank"),
            Ok(GraphStructuralRouteKind::StructuralRerank)
        );
        assert_eq!(
            graph_structural_route_kind("/graph/structural/filter"),
            Ok(GraphStructuralRouteKind::ConstraintFilter)
        );
        assert!(is_graph_structural_route("/graph/structural/rerank"));
        assert!(!is_graph_structural_route("/graph/neighbors"));
        assert_eq!(
            GraphStructuralRouteKind::StructuralRerank.request_columns(),
            &GRAPH_STRUCTURAL_RERANK_REQUEST_COLUMNS
        );
        assert_eq!(
            GraphStructuralRouteKind::ConstraintFilter.response_columns(),
            &GRAPH_STRUCTURAL_FILTER_RESPONSE_COLUMNS
        );
        assert_eq!(
            GraphStructuralRouteKind::StructuralRerank.schema_version(),
            JULIA_GRAPH_STRUCTURAL_SCHEMA_VERSION
        );
    }

    #[test]
    fn structural_rerank_request_batch_validation_accepts_staged_shape() {
        let batch = structural_rerank_request_batch(
            vec!["query-1"],
            vec!["candidate-1"],
            vec![1],
            vec![3],
            vec![0.9],
            vec![0.4],
            vec![0.2],
            vec![0.1],
            vec![vec!["semantic"]],
            vec![vec!["graph retrieval"]],
            vec![vec!["depends_on"]],
            vec![vec!["node-a", "node-b"]],
            vec![vec!["node-a"]],
            vec![vec!["node-b"]],
            vec![vec!["depends_on"]],
        );
        assert!(validate_graph_structural_rerank_request_batch(&batch).is_ok());
    }

    #[test]
    fn structural_rerank_request_batch_validation_rejects_misaligned_anchor_lists() {
        let batch = structural_rerank_request_batch(
            vec!["query-1"],
            vec!["candidate-1"],
            vec![1],
            vec![3],
            vec![0.9],
            vec![0.4],
            vec![0.2],
            vec![0.1],
            vec![vec!["semantic", "keyword"]],
            vec![vec!["graph retrieval"]],
            vec![vec!["depends_on"]],
            vec![vec!["node-a", "node-b"]],
            vec![vec!["node-a"]],
            vec![vec!["node-b"]],
            vec![vec!["depends_on"]],
        );
        assert_eq!(
            validate_graph_structural_rerank_request_batch(&batch),
            Err(
                "graph structural rerank request anchor columns must stay aligned; row 0 has 2 planes but 1 values"
                    .to_string()
            )
        );
    }

    #[test]
    fn structural_rerank_response_batch_validation_rejects_non_finite_final_score() {
        let batch = structural_rerank_response_batch(
            vec!["candidate-1"],
            vec![true],
            vec![0.8],
            vec![f64::INFINITY],
            vec![vec!["node-a"]],
            vec!["matched"],
        );
        assert_eq!(
            validate_graph_structural_rerank_response_batch(&batch),
            Err(
                "graph structural column `final_score` must contain finite values; row 0 is inf"
                    .to_string()
            )
        );
    }

    #[test]
    fn structural_filter_request_batch_validation_accepts_staged_shape() {
        let batch = structural_filter_request_batch(
            vec!["query-1"],
            vec!["candidate-1"],
            vec![0],
            vec![2],
            vec!["pin_assignment"],
            vec![2],
            vec![vec!["semantic"]],
            vec![vec!["graph retrieval"]],
            vec![vec!["depends_on"]],
            vec![vec!["node-a", "node-b"]],
            vec![vec!["node-a"]],
            vec![vec!["node-b"]],
            vec![vec!["depends_on"]],
        );
        assert!(validate_graph_structural_filter_request_batch(&batch).is_ok());
    }

    #[test]
    fn structural_filter_response_batch_validation_rejects_missing_rejection_reason() {
        let batch = structural_filter_response_batch(
            vec!["candidate-1"],
            vec![false],
            vec![0.4],
            vec![vec!["node-a"]],
            vec![""],
        );
        assert_eq!(
            validate_graph_structural_filter_response_batch(&batch),
            Err(
                "graph structural filter response column `rejection_reason` must be non-blank for rejected candidate `candidate-1` at row 0"
                    .to_string()
            )
        );
    }

    #[test]
    fn structural_filter_response_batch_validation_rejects_duplicate_candidate_id() {
        let batch = structural_filter_response_batch(
            vec!["candidate-1", "candidate-1"],
            vec![true, false],
            vec![0.8, 0.4],
            vec![vec!["node-a"], vec!["node-b"]],
            vec!["", "gap"],
        );
        assert_eq!(
            validate_graph_structural_filter_response_batch(&batch),
            Err(
                "graph structural column `candidate_id` must be unique across one batch; row 1 duplicates `candidate-1`"
                    .to_string()
            )
        );
    }

    fn structural_rerank_request_batch(
        query_ids: Vec<&str>,
        candidate_ids: Vec<&str>,
        retrieval_layers: Vec<i32>,
        query_max_layers: Vec<i32>,
        semantic_scores: Vec<f64>,
        dependency_scores: Vec<f64>,
        keyword_scores: Vec<f64>,
        tag_scores: Vec<f64>,
        anchor_planes: Vec<Vec<&str>>,
        anchor_values: Vec<Vec<&str>>,
        edge_constraint_kinds: Vec<Vec<&str>>,
        candidate_node_ids: Vec<Vec<&str>>,
        candidate_edge_sources: Vec<Vec<&str>>,
        candidate_edge_destinations: Vec<Vec<&str>>,
        candidate_edge_kinds: Vec<Vec<&str>>,
    ) -> RecordBatch {
        RecordBatch::try_new(
            Arc::new(Schema::new(vec![
                utf8_field(GRAPH_STRUCTURAL_QUERY_ID_COLUMN),
                utf8_field(GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN),
                int32_field(GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN),
                int32_field(GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN),
                float64_field(GRAPH_STRUCTURAL_SEMANTIC_SCORE_COLUMN),
                float64_field(GRAPH_STRUCTURAL_DEPENDENCY_SCORE_COLUMN),
                float64_field(GRAPH_STRUCTURAL_KEYWORD_SCORE_COLUMN),
                float64_field(GRAPH_STRUCTURAL_TAG_SCORE_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_EDGE_CONSTRAINT_KINDS_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_CANDIDATE_EDGE_SOURCES_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_CANDIDATE_EDGE_DESTINATIONS_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN),
            ])),
            vec![
                Arc::new(StringArray::from(query_ids)) as ArrayRef,
                Arc::new(StringArray::from(candidate_ids)) as ArrayRef,
                Arc::new(Int32Array::from(retrieval_layers)) as ArrayRef,
                Arc::new(Int32Array::from(query_max_layers)) as ArrayRef,
                Arc::new(Float64Array::from(semantic_scores)) as ArrayRef,
                Arc::new(Float64Array::from(dependency_scores)) as ArrayRef,
                Arc::new(Float64Array::from(keyword_scores)) as ArrayRef,
                Arc::new(Float64Array::from(tag_scores)) as ArrayRef,
                Arc::new(list_utf8_array(anchor_planes)) as ArrayRef,
                Arc::new(list_utf8_array(anchor_values)) as ArrayRef,
                Arc::new(list_utf8_array(edge_constraint_kinds)) as ArrayRef,
                Arc::new(list_utf8_array(candidate_node_ids)) as ArrayRef,
                Arc::new(list_utf8_array(candidate_edge_sources)) as ArrayRef,
                Arc::new(list_utf8_array(candidate_edge_destinations)) as ArrayRef,
                Arc::new(list_utf8_array(candidate_edge_kinds)) as ArrayRef,
            ],
        )
        .expect("structural rerank request batch should build")
    }

    fn structural_rerank_response_batch(
        candidate_ids: Vec<&str>,
        feasible: Vec<bool>,
        structural_scores: Vec<f64>,
        final_scores: Vec<f64>,
        pin_assignments: Vec<Vec<&str>>,
        explanations: Vec<&str>,
    ) -> RecordBatch {
        RecordBatch::try_new(
            Arc::new(Schema::new(vec![
                utf8_field(GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN),
                bool_field(GRAPH_STRUCTURAL_FEASIBLE_COLUMN),
                float64_field(GRAPH_STRUCTURAL_STRUCTURAL_SCORE_COLUMN),
                float64_field(GRAPH_STRUCTURAL_FINAL_SCORE_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_PIN_ASSIGNMENT_COLUMN),
                utf8_field(GRAPH_STRUCTURAL_EXPLANATION_COLUMN),
            ])),
            vec![
                Arc::new(StringArray::from(candidate_ids)) as ArrayRef,
                Arc::new(BooleanArray::from(feasible)) as ArrayRef,
                Arc::new(Float64Array::from(structural_scores)) as ArrayRef,
                Arc::new(Float64Array::from(final_scores)) as ArrayRef,
                Arc::new(list_utf8_array(pin_assignments)) as ArrayRef,
                Arc::new(StringArray::from(explanations)) as ArrayRef,
            ],
        )
        .expect("structural rerank response batch should build")
    }

    fn structural_filter_request_batch(
        query_ids: Vec<&str>,
        candidate_ids: Vec<&str>,
        retrieval_layers: Vec<i32>,
        query_max_layers: Vec<i32>,
        constraint_kinds: Vec<&str>,
        required_boundary_sizes: Vec<i32>,
        anchor_planes: Vec<Vec<&str>>,
        anchor_values: Vec<Vec<&str>>,
        edge_constraint_kinds: Vec<Vec<&str>>,
        candidate_node_ids: Vec<Vec<&str>>,
        candidate_edge_sources: Vec<Vec<&str>>,
        candidate_edge_destinations: Vec<Vec<&str>>,
        candidate_edge_kinds: Vec<Vec<&str>>,
    ) -> RecordBatch {
        RecordBatch::try_new(
            Arc::new(Schema::new(vec![
                utf8_field(GRAPH_STRUCTURAL_QUERY_ID_COLUMN),
                utf8_field(GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN),
                int32_field(GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN),
                int32_field(GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN),
                utf8_field(GRAPH_STRUCTURAL_CONSTRAINT_KIND_COLUMN),
                int32_field(GRAPH_STRUCTURAL_REQUIRED_BOUNDARY_SIZE_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_EDGE_CONSTRAINT_KINDS_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_CANDIDATE_EDGE_SOURCES_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_CANDIDATE_EDGE_DESTINATIONS_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN),
            ])),
            vec![
                Arc::new(StringArray::from(query_ids)) as ArrayRef,
                Arc::new(StringArray::from(candidate_ids)) as ArrayRef,
                Arc::new(Int32Array::from(retrieval_layers)) as ArrayRef,
                Arc::new(Int32Array::from(query_max_layers)) as ArrayRef,
                Arc::new(StringArray::from(constraint_kinds)) as ArrayRef,
                Arc::new(Int32Array::from(required_boundary_sizes)) as ArrayRef,
                Arc::new(list_utf8_array(anchor_planes)) as ArrayRef,
                Arc::new(list_utf8_array(anchor_values)) as ArrayRef,
                Arc::new(list_utf8_array(edge_constraint_kinds)) as ArrayRef,
                Arc::new(list_utf8_array(candidate_node_ids)) as ArrayRef,
                Arc::new(list_utf8_array(candidate_edge_sources)) as ArrayRef,
                Arc::new(list_utf8_array(candidate_edge_destinations)) as ArrayRef,
                Arc::new(list_utf8_array(candidate_edge_kinds)) as ArrayRef,
            ],
        )
        .expect("structural filter request batch should build")
    }

    fn structural_filter_response_batch(
        candidate_ids: Vec<&str>,
        accepted: Vec<bool>,
        structural_scores: Vec<f64>,
        pin_assignments: Vec<Vec<&str>>,
        rejection_reasons: Vec<&str>,
    ) -> RecordBatch {
        RecordBatch::try_new(
            Arc::new(Schema::new(vec![
                utf8_field(GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN),
                bool_field(GRAPH_STRUCTURAL_ACCEPTED_COLUMN),
                float64_field(GRAPH_STRUCTURAL_STRUCTURAL_SCORE_COLUMN),
                list_utf8_field(GRAPH_STRUCTURAL_PIN_ASSIGNMENT_COLUMN),
                utf8_field(GRAPH_STRUCTURAL_REJECTION_REASON_COLUMN),
            ])),
            vec![
                Arc::new(StringArray::from(candidate_ids)) as ArrayRef,
                Arc::new(BooleanArray::from(accepted)) as ArrayRef,
                Arc::new(Float64Array::from(structural_scores)) as ArrayRef,
                Arc::new(list_utf8_array(pin_assignments)) as ArrayRef,
                Arc::new(StringArray::from(rejection_reasons)) as ArrayRef,
            ],
        )
        .expect("structural filter response batch should build")
    }

    fn utf8_field(name: &str) -> Field {
        Field::new(name, DataType::Utf8, false)
    }

    fn bool_field(name: &str) -> Field {
        Field::new(name, DataType::Boolean, false)
    }

    fn int32_field(name: &str) -> Field {
        Field::new(name, DataType::Int32, false)
    }

    fn float64_field(name: &str) -> Field {
        Field::new(name, DataType::Float64, false)
    }

    fn list_utf8_field(name: &str) -> Field {
        Field::new(
            name,
            DataType::List(Arc::new(Field::new("item", DataType::Utf8, true))),
            false,
        )
    }

    fn list_utf8_array(rows: Vec<Vec<&str>>) -> ListArray {
        let mut builder = ListBuilder::new(StringBuilder::new());
        for row in rows {
            for value in row {
                builder.values().append_value(value);
            }
            builder.append(true);
        }
        builder.finish()
    }
}
