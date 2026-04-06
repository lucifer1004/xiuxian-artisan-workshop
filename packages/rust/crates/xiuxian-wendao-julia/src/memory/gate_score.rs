use std::sync::Arc;

use arrow::array::{Array, Float32Array, StringArray, UInt32Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use xiuxian_wendao_core::repo_intelligence::RepoIntelligenceError;

/// Request column carrying the stable memory id.
pub const MEMORY_JULIA_GATE_SCORE_MEMORY_ID_COLUMN: &str = "memory_id";
/// Request column carrying the optional scenario pack.
pub const MEMORY_JULIA_GATE_SCORE_SCENARIO_PACK_COLUMN: &str = "scenario_pack";
/// Request column carrying the `ReAct` revalidation score.
pub const MEMORY_JULIA_GATE_SCORE_REACT_REVALIDATION_SCORE_COLUMN: &str =
    "react_revalidation_score";
/// Request column carrying the graph consistency score.
pub const MEMORY_JULIA_GATE_SCORE_GRAPH_CONSISTENCY_SCORE_COLUMN: &str = "graph_consistency_score";
/// Request column carrying the omega alignment score.
pub const MEMORY_JULIA_GATE_SCORE_OMEGA_ALIGNMENT_SCORE_COLUMN: &str = "omega_alignment_score";
/// Request column carrying the host utility estimate.
pub const MEMORY_JULIA_GATE_SCORE_Q_VALUE_COLUMN: &str = "q_value";
/// Request column carrying the usage counter.
pub const MEMORY_JULIA_GATE_SCORE_USAGE_COUNT_COLUMN: &str = "usage_count";
/// Request column carrying the failure rate.
pub const MEMORY_JULIA_GATE_SCORE_FAILURE_RATE_COLUMN: &str = "failure_rate";
/// Request column carrying the TTL score.
pub const MEMORY_JULIA_GATE_SCORE_TTL_SCORE_COLUMN: &str = "ttl_score";
/// Request column carrying the current lifecycle state.
pub const MEMORY_JULIA_GATE_SCORE_CURRENT_STATE_COLUMN: &str = "current_state";

/// Response column carrying the recommendation verdict.
pub const MEMORY_JULIA_GATE_SCORE_VERDICT_COLUMN: &str = "verdict";
/// Response column carrying the confidence score.
pub const MEMORY_JULIA_GATE_SCORE_CONFIDENCE_COLUMN: &str = "confidence";
/// Response column carrying the utility score.
pub const MEMORY_JULIA_GATE_SCORE_UTILITY_SCORE_COLUMN: &str = "utility_score";
/// Response column carrying the next action string.
pub const MEMORY_JULIA_GATE_SCORE_NEXT_ACTION_COLUMN: &str = "next_action";
/// Response column carrying the rationale.
pub const MEMORY_JULIA_GATE_SCORE_REASON_COLUMN: &str = "reason";
/// Response column carrying the physical schema version echoed by the provider.
pub const MEMORY_JULIA_GATE_SCORE_SCHEMA_VERSION_COLUMN: &str = "schema_version";

/// Canonical request column order for the staged gate-score profile.
pub const MEMORY_JULIA_GATE_SCORE_REQUEST_COLUMNS: [&str; 10] = [
    MEMORY_JULIA_GATE_SCORE_MEMORY_ID_COLUMN,
    MEMORY_JULIA_GATE_SCORE_SCENARIO_PACK_COLUMN,
    MEMORY_JULIA_GATE_SCORE_REACT_REVALIDATION_SCORE_COLUMN,
    MEMORY_JULIA_GATE_SCORE_GRAPH_CONSISTENCY_SCORE_COLUMN,
    MEMORY_JULIA_GATE_SCORE_OMEGA_ALIGNMENT_SCORE_COLUMN,
    MEMORY_JULIA_GATE_SCORE_Q_VALUE_COLUMN,
    MEMORY_JULIA_GATE_SCORE_USAGE_COUNT_COLUMN,
    MEMORY_JULIA_GATE_SCORE_FAILURE_RATE_COLUMN,
    MEMORY_JULIA_GATE_SCORE_TTL_SCORE_COLUMN,
    MEMORY_JULIA_GATE_SCORE_CURRENT_STATE_COLUMN,
];

/// Canonical response column order for the staged gate-score profile.
pub const MEMORY_JULIA_GATE_SCORE_RESPONSE_COLUMNS: [&str; 8] = [
    MEMORY_JULIA_GATE_SCORE_MEMORY_ID_COLUMN,
    MEMORY_JULIA_GATE_SCORE_VERDICT_COLUMN,
    MEMORY_JULIA_GATE_SCORE_CONFIDENCE_COLUMN,
    MEMORY_JULIA_GATE_SCORE_UTILITY_SCORE_COLUMN,
    MEMORY_JULIA_GATE_SCORE_TTL_SCORE_COLUMN,
    MEMORY_JULIA_GATE_SCORE_NEXT_ACTION_COLUMN,
    MEMORY_JULIA_GATE_SCORE_REASON_COLUMN,
    MEMORY_JULIA_GATE_SCORE_SCHEMA_VERSION_COLUMN,
];

const MEMORY_GATE_SCORE_ALLOWED_STATES: [&str; 6] = [
    "open",
    "active",
    "cooling",
    "revalidate_pending",
    "purged",
    "promoted",
];

const MEMORY_GATE_SCORE_ALLOWED_VERDICTS: [&str; 3] = ["retain", "obsolete", "promote"];

/// One typed request row for the staged gate-score profile.
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryJuliaGateScoreRequestRow {
    /// Stable memory episode id.
    pub memory_id: String,
    /// Optional scenario pack forwarded into Julia.
    pub scenario_pack: Option<String>,
    /// `ReAct` revalidation score in `[0, 1]`.
    pub react_revalidation_score: f32,
    /// Graph consistency score in `[0, 1]`.
    pub graph_consistency_score: f32,
    /// Omega alignment score in `[0, 1]`.
    pub omega_alignment_score: f32,
    /// Host utility estimate in `[0, 1]`.
    pub q_value: f32,
    /// Host usage count.
    pub usage_count: u32,
    /// Failure rate in `[0, 1]`.
    pub failure_rate: f32,
    /// TTL score in `[0, 1]`.
    pub ttl_score: f32,
    /// Current lifecycle state string.
    pub current_state: String,
}

/// One decoded recommendation row for the staged gate-score profile.
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryJuliaGateScoreRecommendationRow {
    /// Stable memory episode id echoed by the provider.
    pub memory_id: String,
    /// Recommendation-only verdict string.
    pub verdict: String,
    /// Confidence score in `[0, 1]`.
    pub confidence: f32,
    /// Utility score in `[0, 1]`.
    pub utility_score: f32,
    /// TTL score in `[0, 1]`.
    pub ttl_score: f32,
    /// Suggested next action string.
    pub next_action: String,
    /// Human-readable rationale.
    pub reason: String,
    /// Physical schema version echoed by the provider.
    pub schema_version: String,
}

/// Build the staged gate-score request schema.
#[must_use]
pub fn memory_julia_gate_score_request_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new(
            MEMORY_JULIA_GATE_SCORE_MEMORY_ID_COLUMN,
            DataType::Utf8,
            false,
        ),
        Field::new(
            MEMORY_JULIA_GATE_SCORE_SCENARIO_PACK_COLUMN,
            DataType::Utf8,
            true,
        ),
        Field::new(
            MEMORY_JULIA_GATE_SCORE_REACT_REVALIDATION_SCORE_COLUMN,
            DataType::Float32,
            false,
        ),
        Field::new(
            MEMORY_JULIA_GATE_SCORE_GRAPH_CONSISTENCY_SCORE_COLUMN,
            DataType::Float32,
            false,
        ),
        Field::new(
            MEMORY_JULIA_GATE_SCORE_OMEGA_ALIGNMENT_SCORE_COLUMN,
            DataType::Float32,
            false,
        ),
        Field::new(
            MEMORY_JULIA_GATE_SCORE_Q_VALUE_COLUMN,
            DataType::Float32,
            false,
        ),
        Field::new(
            MEMORY_JULIA_GATE_SCORE_USAGE_COUNT_COLUMN,
            DataType::UInt32,
            false,
        ),
        Field::new(
            MEMORY_JULIA_GATE_SCORE_FAILURE_RATE_COLUMN,
            DataType::Float32,
            false,
        ),
        Field::new(
            MEMORY_JULIA_GATE_SCORE_TTL_SCORE_COLUMN,
            DataType::Float32,
            false,
        ),
        Field::new(
            MEMORY_JULIA_GATE_SCORE_CURRENT_STATE_COLUMN,
            DataType::Utf8,
            false,
        ),
    ]))
}

/// Build the staged gate-score response schema.
#[must_use]
pub fn memory_julia_gate_score_response_schema() -> Arc<Schema> {
    Arc::new(Schema::new(vec![
        Field::new(
            MEMORY_JULIA_GATE_SCORE_MEMORY_ID_COLUMN,
            DataType::Utf8,
            false,
        ),
        Field::new(
            MEMORY_JULIA_GATE_SCORE_VERDICT_COLUMN,
            DataType::Utf8,
            false,
        ),
        Field::new(
            MEMORY_JULIA_GATE_SCORE_CONFIDENCE_COLUMN,
            DataType::Float32,
            false,
        ),
        Field::new(
            MEMORY_JULIA_GATE_SCORE_UTILITY_SCORE_COLUMN,
            DataType::Float32,
            false,
        ),
        Field::new(
            MEMORY_JULIA_GATE_SCORE_TTL_SCORE_COLUMN,
            DataType::Float32,
            false,
        ),
        Field::new(
            MEMORY_JULIA_GATE_SCORE_NEXT_ACTION_COLUMN,
            DataType::Utf8,
            false,
        ),
        Field::new(MEMORY_JULIA_GATE_SCORE_REASON_COLUMN, DataType::Utf8, false),
        Field::new(
            MEMORY_JULIA_GATE_SCORE_SCHEMA_VERSION_COLUMN,
            DataType::Utf8,
            false,
        ),
    ]))
}

/// Build one staged gate-score request batch from typed rows.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the request rows violate the staged
/// gate-score contract.
pub fn build_memory_julia_gate_score_request_batch(
    rows: &[MemoryJuliaGateScoreRequestRow],
) -> Result<RecordBatch, RepoIntelligenceError> {
    let batch = RecordBatch::try_new(
        memory_julia_gate_score_request_schema(),
        vec![
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.memory_id.as_str())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.scenario_pack.as_deref())
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float32Array::from(
                rows.iter()
                    .map(|row| row.react_revalidation_score)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float32Array::from(
                rows.iter()
                    .map(|row| row.graph_consistency_score)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float32Array::from(
                rows.iter()
                    .map(|row| row.omega_alignment_score)
                    .collect::<Vec<_>>(),
            )),
            Arc::new(Float32Array::from(
                rows.iter().map(|row| row.q_value).collect::<Vec<_>>(),
            )),
            Arc::new(UInt32Array::from(
                rows.iter().map(|row| row.usage_count).collect::<Vec<_>>(),
            )),
            Arc::new(Float32Array::from(
                rows.iter().map(|row| row.failure_rate).collect::<Vec<_>>(),
            )),
            Arc::new(Float32Array::from(
                rows.iter().map(|row| row.ttl_score).collect::<Vec<_>>(),
            )),
            Arc::new(StringArray::from(
                rows.iter()
                    .map(|row| row.current_state.as_str())
                    .collect::<Vec<_>>(),
            )),
        ],
    )
    .map_err(|error| gate_score_contract_error(&error.to_string()))?;

    validate_memory_julia_gate_score_request_batch(&batch)
        .map_err(|error| gate_score_contract_error(&error))?;
    Ok(batch)
}

/// Validate one staged gate-score request schema.
///
/// # Errors
///
/// Returns an error when the schema does not match the staged gate-score
/// contract.
pub fn validate_memory_julia_gate_score_request_schema(schema: &Schema) -> Result<(), String> {
    validate_utf8_field(schema, MEMORY_JULIA_GATE_SCORE_MEMORY_ID_COLUMN, false)?;
    validate_utf8_field(schema, MEMORY_JULIA_GATE_SCORE_SCENARIO_PACK_COLUMN, true)?;
    validate_primitive_field(
        schema,
        MEMORY_JULIA_GATE_SCORE_REACT_REVALIDATION_SCORE_COLUMN,
        &DataType::Float32,
        false,
    )?;
    validate_primitive_field(
        schema,
        MEMORY_JULIA_GATE_SCORE_GRAPH_CONSISTENCY_SCORE_COLUMN,
        &DataType::Float32,
        false,
    )?;
    validate_primitive_field(
        schema,
        MEMORY_JULIA_GATE_SCORE_OMEGA_ALIGNMENT_SCORE_COLUMN,
        &DataType::Float32,
        false,
    )?;
    validate_primitive_field(
        schema,
        MEMORY_JULIA_GATE_SCORE_Q_VALUE_COLUMN,
        &DataType::Float32,
        false,
    )?;
    validate_primitive_field(
        schema,
        MEMORY_JULIA_GATE_SCORE_USAGE_COUNT_COLUMN,
        &DataType::UInt32,
        false,
    )?;
    validate_primitive_field(
        schema,
        MEMORY_JULIA_GATE_SCORE_FAILURE_RATE_COLUMN,
        &DataType::Float32,
        false,
    )?;
    validate_primitive_field(
        schema,
        MEMORY_JULIA_GATE_SCORE_TTL_SCORE_COLUMN,
        &DataType::Float32,
        false,
    )?;
    validate_utf8_field(schema, MEMORY_JULIA_GATE_SCORE_CURRENT_STATE_COLUMN, false)?;
    Ok(())
}

/// Validate one staged gate-score request batch.
///
/// # Errors
///
/// Returns an error when the batch violates the staged gate-score semantics.
pub fn validate_memory_julia_gate_score_request_batch(batch: &RecordBatch) -> Result<(), String> {
    validate_memory_julia_gate_score_request_schema(batch.schema().as_ref())?;

    if batch.num_rows() == 0 {
        return Err("gate score request batch must contain at least one row".to_string());
    }

    require_non_blank_utf8_column(batch, MEMORY_JULIA_GATE_SCORE_MEMORY_ID_COLUMN)?;
    require_non_blank_optional_utf8_column(batch, MEMORY_JULIA_GATE_SCORE_SCENARIO_PACK_COLUMN)?;
    require_probability_column(
        batch,
        MEMORY_JULIA_GATE_SCORE_REACT_REVALIDATION_SCORE_COLUMN,
    )?;
    require_probability_column(
        batch,
        MEMORY_JULIA_GATE_SCORE_GRAPH_CONSISTENCY_SCORE_COLUMN,
    )?;
    require_probability_column(batch, MEMORY_JULIA_GATE_SCORE_OMEGA_ALIGNMENT_SCORE_COLUMN)?;
    require_probability_column(batch, MEMORY_JULIA_GATE_SCORE_Q_VALUE_COLUMN)?;
    require_probability_column(batch, MEMORY_JULIA_GATE_SCORE_FAILURE_RATE_COLUMN)?;
    require_probability_column(batch, MEMORY_JULIA_GATE_SCORE_TTL_SCORE_COLUMN)?;
    require_allowed_utf8_values(
        batch,
        MEMORY_JULIA_GATE_SCORE_CURRENT_STATE_COLUMN,
        &MEMORY_GATE_SCORE_ALLOWED_STATES,
    )?;

    Ok(())
}

/// Validate many staged gate-score request batches.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when any request batch violates the staged
/// contract.
pub fn validate_memory_julia_gate_score_request_batches(
    batches: &[RecordBatch],
) -> Result<(), RepoIntelligenceError> {
    for batch in batches {
        validate_memory_julia_gate_score_request_batch(batch)
            .map_err(|error| gate_score_contract_error(&error))?;
    }
    Ok(())
}

/// Validate one staged gate-score response schema.
///
/// # Errors
///
/// Returns an error when the schema does not match the staged gate-score
/// response contract.
pub fn validate_memory_julia_gate_score_response_schema(schema: &Schema) -> Result<(), String> {
    validate_utf8_field(schema, MEMORY_JULIA_GATE_SCORE_MEMORY_ID_COLUMN, false)?;
    validate_utf8_field(schema, MEMORY_JULIA_GATE_SCORE_VERDICT_COLUMN, false)?;
    validate_primitive_field(
        schema,
        MEMORY_JULIA_GATE_SCORE_CONFIDENCE_COLUMN,
        &DataType::Float32,
        false,
    )?;
    validate_primitive_field(
        schema,
        MEMORY_JULIA_GATE_SCORE_UTILITY_SCORE_COLUMN,
        &DataType::Float32,
        false,
    )?;
    validate_primitive_field(
        schema,
        MEMORY_JULIA_GATE_SCORE_TTL_SCORE_COLUMN,
        &DataType::Float32,
        false,
    )?;
    validate_utf8_field(schema, MEMORY_JULIA_GATE_SCORE_NEXT_ACTION_COLUMN, false)?;
    validate_utf8_field(schema, MEMORY_JULIA_GATE_SCORE_REASON_COLUMN, false)?;
    validate_utf8_field(schema, MEMORY_JULIA_GATE_SCORE_SCHEMA_VERSION_COLUMN, false)?;
    Ok(())
}

/// Validate one staged gate-score response batch.
///
/// # Errors
///
/// Returns an error when the batch violates the staged gate-score response
/// semantics.
pub fn validate_memory_julia_gate_score_response_batch(batch: &RecordBatch) -> Result<(), String> {
    validate_memory_julia_gate_score_response_schema(batch.schema().as_ref())?;

    if batch.num_rows() == 0 {
        return Err("gate score response batch must contain at least one row".to_string());
    }

    require_non_blank_utf8_column(batch, MEMORY_JULIA_GATE_SCORE_MEMORY_ID_COLUMN)?;
    require_allowed_utf8_values(
        batch,
        MEMORY_JULIA_GATE_SCORE_VERDICT_COLUMN,
        &MEMORY_GATE_SCORE_ALLOWED_VERDICTS,
    )?;
    require_probability_column(batch, MEMORY_JULIA_GATE_SCORE_CONFIDENCE_COLUMN)?;
    require_probability_column(batch, MEMORY_JULIA_GATE_SCORE_UTILITY_SCORE_COLUMN)?;
    require_probability_column(batch, MEMORY_JULIA_GATE_SCORE_TTL_SCORE_COLUMN)?;
    require_non_blank_utf8_column(batch, MEMORY_JULIA_GATE_SCORE_NEXT_ACTION_COLUMN)?;
    require_non_blank_utf8_column(batch, MEMORY_JULIA_GATE_SCORE_REASON_COLUMN)?;
    require_non_blank_utf8_column(batch, MEMORY_JULIA_GATE_SCORE_SCHEMA_VERSION_COLUMN)?;

    Ok(())
}

/// Validate many staged gate-score response batches.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when any response batch violates the
/// staged contract.
pub fn validate_memory_julia_gate_score_response_batches(
    batches: &[RecordBatch],
) -> Result<(), RepoIntelligenceError> {
    for batch in batches {
        validate_memory_julia_gate_score_response_batch(batch)
            .map_err(|error| gate_score_contract_error(&error))?;
    }
    Ok(())
}

/// Decode many staged gate-score response batches into typed recommendation
/// rows.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when any batch violates the staged
/// response contract.
pub fn decode_memory_julia_gate_score_recommendation_rows(
    batches: &[RecordBatch],
) -> Result<Vec<MemoryJuliaGateScoreRecommendationRow>, RepoIntelligenceError> {
    validate_memory_julia_gate_score_response_batches(batches)?;

    let mut rows = Vec::new();
    for batch in batches {
        let memory_id = utf8_column(batch, MEMORY_JULIA_GATE_SCORE_MEMORY_ID_COLUMN)?;
        let verdict = utf8_column(batch, MEMORY_JULIA_GATE_SCORE_VERDICT_COLUMN)?;
        let confidence = float32_column(batch, MEMORY_JULIA_GATE_SCORE_CONFIDENCE_COLUMN)?;
        let utility_score = float32_column(batch, MEMORY_JULIA_GATE_SCORE_UTILITY_SCORE_COLUMN)?;
        let ttl_score = float32_column(batch, MEMORY_JULIA_GATE_SCORE_TTL_SCORE_COLUMN)?;
        let next_action = utf8_column(batch, MEMORY_JULIA_GATE_SCORE_NEXT_ACTION_COLUMN)?;
        let reason = utf8_column(batch, MEMORY_JULIA_GATE_SCORE_REASON_COLUMN)?;
        let schema_version = utf8_column(batch, MEMORY_JULIA_GATE_SCORE_SCHEMA_VERSION_COLUMN)?;

        for row in 0..batch.num_rows() {
            rows.push(MemoryJuliaGateScoreRecommendationRow {
                memory_id: memory_id.value(row).to_string(),
                verdict: verdict.value(row).to_string(),
                confidence: confidence.value(row),
                utility_score: utility_score.value(row),
                ttl_score: ttl_score.value(row),
                next_action: next_action.value(row).to_string(),
                reason: reason.value(row).to_string(),
                schema_version: schema_version.value(row).to_string(),
            });
        }
    }

    Ok(rows)
}

fn validate_utf8_field(schema: &Schema, name: &str, nullable: bool) -> Result<(), String> {
    validate_primitive_field(schema, name, &DataType::Utf8, nullable)
}

fn validate_primitive_field(
    schema: &Schema,
    name: &str,
    data_type: &DataType,
    nullable: bool,
) -> Result<(), String> {
    let field = schema
        .field_with_name(name)
        .map_err(|_| format!("missing `{name}` field"))?;
    if field.data_type() != data_type {
        return Err(format!(
            "`{name}` must use {:?}, found {:?}",
            data_type,
            field.data_type()
        ));
    }
    if field.is_nullable() != nullable {
        return Err(format!("`{name}` nullable mismatch"));
    }
    Ok(())
}

fn utf8_column<'a>(
    batch: &'a RecordBatch,
    name: &str,
) -> Result<&'a StringArray, RepoIntelligenceError> {
    batch
        .column_by_name(name)
        .ok_or_else(|| gate_score_contract_error(&format!("missing `{name}` column")))?
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| gate_score_contract_error(&format!("`{name}` must be Utf8")))
}

fn float32_column<'a>(
    batch: &'a RecordBatch,
    name: &str,
) -> Result<&'a Float32Array, RepoIntelligenceError> {
    batch
        .column_by_name(name)
        .ok_or_else(|| gate_score_contract_error(&format!("missing `{name}` column")))?
        .as_any()
        .downcast_ref::<Float32Array>()
        .ok_or_else(|| gate_score_contract_error(&format!("`{name}` must be Float32")))
}

fn require_non_blank_utf8_column(batch: &RecordBatch, name: &str) -> Result<(), String> {
    let column = batch
        .column_by_name(name)
        .ok_or_else(|| format!("missing `{name}` column"))?
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| format!("`{name}` must be Utf8"))?;

    for row in 0..batch.num_rows() {
        if column.is_null(row) || column.value(row).trim().is_empty() {
            return Err(format!("`{name}` contains a blank value at row {row}"));
        }
    }
    Ok(())
}

fn require_non_blank_optional_utf8_column(batch: &RecordBatch, name: &str) -> Result<(), String> {
    let column = batch
        .column_by_name(name)
        .ok_or_else(|| format!("missing `{name}` column"))?
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| format!("`{name}` must be Utf8"))?;

    for row in 0..batch.num_rows() {
        if !column.is_null(row) && column.value(row).trim().is_empty() {
            return Err(format!("`{name}` contains a blank value at row {row}"));
        }
    }
    Ok(())
}

fn require_probability_column(batch: &RecordBatch, name: &str) -> Result<(), String> {
    let column = batch
        .column_by_name(name)
        .ok_or_else(|| format!("missing `{name}` column"))?
        .as_any()
        .downcast_ref::<Float32Array>()
        .ok_or_else(|| format!("`{name}` must be Float32"))?;

    for row in 0..batch.num_rows() {
        let value = column.value(row);
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(format!(
                "`{name}` must contain finite values in [0, 1]; found {value} at row {row}"
            ));
        }
    }
    Ok(())
}

fn require_allowed_utf8_values(
    batch: &RecordBatch,
    name: &str,
    allowed: &[&str],
) -> Result<(), String> {
    let column = batch
        .column_by_name(name)
        .ok_or_else(|| format!("missing `{name}` column"))?
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| format!("`{name}` must be Utf8"))?;

    for row in 0..batch.num_rows() {
        if column.is_null(row) {
            return Err(format!("`{name}` contains null at row {row}"));
        }
        let value = column.value(row).trim();
        if value.is_empty() {
            return Err(format!("`{name}` contains a blank value at row {row}"));
        }
        if !allowed.contains(&value) {
            return Err(format!(
                "`{name}` contains unsupported value `{value}` at row {row}"
            ));
        }
    }
    Ok(())
}

fn gate_score_contract_error(message: &str) -> RepoIntelligenceError {
    RepoIntelligenceError::AnalysisFailed {
        message: format!("memory Julia memory_gate_score contract violation: {message}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MEMORY_JULIA_GATE_SCORE_REQUEST_COLUMNS, MemoryJuliaGateScoreRecommendationRow,
        MemoryJuliaGateScoreRequestRow, build_memory_julia_gate_score_request_batch,
        decode_memory_julia_gate_score_recommendation_rows,
        memory_julia_gate_score_response_schema, validate_memory_julia_gate_score_response_batch,
    };
    use arrow::array::{Float32Array, StringArray};
    use arrow::record_batch::RecordBatch;
    use std::sync::Arc;

    fn sample_request_row() -> MemoryJuliaGateScoreRequestRow {
        MemoryJuliaGateScoreRequestRow {
            memory_id: "episode-1".to_string(),
            scenario_pack: Some("searchinfra".to_string()),
            react_revalidation_score: 0.91,
            graph_consistency_score: 0.88,
            omega_alignment_score: 0.93,
            q_value: 0.84,
            usage_count: 5,
            failure_rate: 0.10,
            ttl_score: 0.72,
            current_state: "active".to_string(),
        }
    }

    fn sample_recommendation_row() -> MemoryJuliaGateScoreRecommendationRow {
        MemoryJuliaGateScoreRecommendationRow {
            memory_id: "episode-1".to_string(),
            verdict: "promote".to_string(),
            confidence: 0.93,
            utility_score: 0.87,
            ttl_score: 0.72,
            next_action: "promote".to_string(),
            reason: "utility and ttl exceed promotion threshold".to_string(),
            schema_version: "v1".to_string(),
        }
    }

    #[test]
    fn build_memory_julia_gate_score_request_batch_accepts_valid_rows() {
        let batch = build_memory_julia_gate_score_request_batch(&[sample_request_row()])
            .unwrap_or_else(|error| panic!("request batch should build: {error}"));
        assert_eq!(batch.num_rows(), 1);
        assert_eq!(
            batch
                .schema()
                .fields()
                .iter()
                .map(|field| field.name().clone())
                .collect::<Vec<_>>(),
            MEMORY_JULIA_GATE_SCORE_REQUEST_COLUMNS
                .iter()
                .map(std::string::ToString::to_string)
                .collect::<Vec<_>>()
        );
        assert!(batch.column_by_name("current_state").is_some());
    }

    #[test]
    fn build_memory_julia_gate_score_request_batch_rejects_invalid_state() {
        let mut row = sample_request_row();
        row.current_state = "unknown".to_string();
        let Err(error) = build_memory_julia_gate_score_request_batch(&[row]) else {
            panic!("unsupported lifecycle state should fail");
        };
        assert!(error.to_string().contains("unsupported value"));
    }

    #[test]
    fn decode_memory_julia_gate_score_recommendation_rows_decodes_valid_batches() {
        let expected = sample_recommendation_row();
        let batch = RecordBatch::try_new(
            memory_julia_gate_score_response_schema(),
            vec![
                Arc::new(StringArray::from(vec![expected.memory_id.as_str()])),
                Arc::new(StringArray::from(vec![expected.verdict.as_str()])),
                Arc::new(Float32Array::from(vec![expected.confidence])),
                Arc::new(Float32Array::from(vec![expected.utility_score])),
                Arc::new(Float32Array::from(vec![expected.ttl_score])),
                Arc::new(StringArray::from(vec![expected.next_action.as_str()])),
                Arc::new(StringArray::from(vec![expected.reason.as_str()])),
                Arc::new(StringArray::from(vec![expected.schema_version.as_str()])),
            ],
        )
        .unwrap_or_else(|error| panic!("response batch should build: {error}"));

        let rows = decode_memory_julia_gate_score_recommendation_rows(&[batch])
            .unwrap_or_else(|error| panic!("response rows should decode: {error}"));
        assert_eq!(rows, vec![expected]);
    }

    #[test]
    fn validate_memory_julia_gate_score_response_batch_rejects_invalid_confidence() {
        let batch = RecordBatch::try_new(
            memory_julia_gate_score_response_schema(),
            vec![
                Arc::new(StringArray::from(vec!["episode-1"])),
                Arc::new(StringArray::from(vec!["retain"])),
                Arc::new(Float32Array::from(vec![1.2])),
                Arc::new(Float32Array::from(vec![0.64])),
                Arc::new(Float32Array::from(vec![0.58])),
                Arc::new(StringArray::from(vec!["retain"])),
                Arc::new(StringArray::from(vec!["stay active"])),
                Arc::new(StringArray::from(vec!["v1"])),
            ],
        )
        .unwrap_or_else(|error| panic!("response batch should build: {error}"));

        let Err(error) = validate_memory_julia_gate_score_response_batch(&batch) else {
            panic!("confidence above one should fail");
        };
        assert!(error.contains("finite values in [0, 1]"));
    }

    #[test]
    fn validate_memory_julia_gate_score_response_batch_rejects_unknown_verdict() {
        let batch = RecordBatch::try_new(
            memory_julia_gate_score_response_schema(),
            vec![
                Arc::new(StringArray::from(vec!["episode-1"])),
                Arc::new(StringArray::from(vec!["promote_to_working_knowledge"])),
                Arc::new(Float32Array::from(vec![0.82])),
                Arc::new(Float32Array::from(vec![0.79])),
                Arc::new(Float32Array::from(vec![0.61])),
                Arc::new(StringArray::from(vec!["promote"])),
                Arc::new(StringArray::from(vec!["promote candidate"])),
                Arc::new(StringArray::from(vec!["v1"])),
            ],
        )
        .unwrap_or_else(|error| panic!("response batch should build: {error}"));

        let Err(error) = validate_memory_julia_gate_score_response_batch(&batch) else {
            panic!("unsupported verdict should fail");
        };
        assert!(error.contains("unsupported value"));
    }
}
