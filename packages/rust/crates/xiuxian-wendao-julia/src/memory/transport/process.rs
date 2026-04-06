use arrow::record_batch::RecordBatch;
use xiuxian_vector::attach_record_batch_metadata;
use xiuxian_wendao_core::repo_intelligence::RepoIntelligenceError;
use xiuxian_wendao_runtime::{
    config::MemoryJuliaComputeRuntimeConfig,
    transport::{FLIGHT_SCHEMA_VERSION_METADATA_KEY, NegotiatedFlightTransportClient},
};

use crate::memory::{
    MemoryJuliaComputeProfile, build_memory_julia_compute_flight_transport_client,
    validate_memory_julia_calibration_request_batches,
    validate_memory_julia_calibration_response_batches,
    validate_memory_julia_episodic_recall_request_batches,
    validate_memory_julia_episodic_recall_response_batches,
    validate_memory_julia_gate_score_request_batches,
    validate_memory_julia_gate_score_response_batches,
    validate_memory_julia_plan_tuning_request_batches,
    validate_memory_julia_plan_tuning_response_batches,
};

/// Validate staged memory-family request batches for one profile.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when any batch violates the staged
/// request contract for the selected profile.
pub fn validate_memory_julia_compute_request_batches(
    profile: MemoryJuliaComputeProfile,
    batches: &[RecordBatch],
) -> Result<(), RepoIntelligenceError> {
    match profile {
        MemoryJuliaComputeProfile::EpisodicRecall => {
            validate_memory_julia_episodic_recall_request_batches(batches)
        }
        MemoryJuliaComputeProfile::MemoryGateScore => {
            validate_memory_julia_gate_score_request_batches(batches)
        }
        MemoryJuliaComputeProfile::MemoryPlanTuning => {
            validate_memory_julia_plan_tuning_request_batches(batches)
        }
        MemoryJuliaComputeProfile::MemoryCalibration => {
            validate_memory_julia_calibration_request_batches(batches)
        }
    }
}

/// Validate staged memory-family response batches for one profile.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when any batch violates the staged
/// response contract for the selected profile.
pub fn validate_memory_julia_compute_response_batches(
    profile: MemoryJuliaComputeProfile,
    batches: &[RecordBatch],
) -> Result<(), RepoIntelligenceError> {
    match profile {
        MemoryJuliaComputeProfile::EpisodicRecall => {
            validate_memory_julia_episodic_recall_response_batches(batches)
        }
        MemoryJuliaComputeProfile::MemoryGateScore => {
            validate_memory_julia_gate_score_response_batches(batches)
        }
        MemoryJuliaComputeProfile::MemoryPlanTuning => {
            validate_memory_julia_plan_tuning_response_batches(batches)
        }
        MemoryJuliaComputeProfile::MemoryCalibration => {
            validate_memory_julia_calibration_response_batches(batches)
        }
    }
}

/// Send staged memory-family request batches through one negotiated Flight
/// client and validate the staged response contract.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the request violates the staged
/// contract, request metadata cannot be attached, the Flight roundtrip fails,
/// or the response violates the staged contract.
pub async fn process_memory_julia_compute_flight_batches(
    client: &NegotiatedFlightTransportClient,
    profile: MemoryJuliaComputeProfile,
    schema_version: &str,
    batches: &[RecordBatch],
) -> Result<Vec<RecordBatch>, RepoIntelligenceError> {
    validate_memory_julia_compute_request_batches(profile, batches)?;
    let request_batches = batches
        .iter()
        .map(|batch| attach_schema_version_metadata(batch, schema_version, profile))
        .collect::<Result<Vec<_>, _>>()?;
    let response_batches = client
        .process_batches(&request_batches)
        .await
        .map_err(|error| RepoIntelligenceError::AnalysisFailed {
            message: format!(
                "memory Julia compute Flight request for profile `{}` failed: {error}",
                profile.profile_id()
            ),
        })?;
    validate_memory_julia_compute_response_batches(profile, response_batches.as_slice())?;
    Ok(response_batches)
}

/// Build a runtime-config-driven Flight client for one memory-family profile,
/// execute the request batches, and validate the staged response contract.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the runtime is disabled, cannot be
/// negotiated into a client, the request violates the staged contract, or the
/// response fails validation.
pub async fn process_memory_julia_compute_flight_batches_for_runtime(
    runtime: &MemoryJuliaComputeRuntimeConfig,
    profile: MemoryJuliaComputeProfile,
    batches: &[RecordBatch],
) -> Result<Vec<RecordBatch>, RepoIntelligenceError> {
    let client =
        build_memory_julia_compute_flight_transport_client(runtime, profile)?.ok_or_else(|| {
            RepoIntelligenceError::AnalysisFailed {
                message: format!(
                    "memory Julia compute runtime is disabled or unavailable for profile `{}`",
                    profile.profile_id()
                ),
            }
        })?;
    process_memory_julia_compute_flight_batches(&client, profile, &runtime.schema_version, batches)
        .await
}

fn attach_schema_version_metadata(
    batch: &RecordBatch,
    schema_version: &str,
    profile: MemoryJuliaComputeProfile,
) -> Result<RecordBatch, RepoIntelligenceError> {
    attach_record_batch_metadata(
        batch,
        [(FLIGHT_SCHEMA_VERSION_METADATA_KEY, schema_version)],
    )
    .map_err(|error| RepoIntelligenceError::AnalysisFailed {
        message: format!(
            "failed to attach memory Julia compute schema metadata for profile `{}`: {error}",
            profile.profile_id()
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        validate_memory_julia_compute_request_batches,
        validate_memory_julia_compute_response_batches,
    };
    use crate::memory::{
        MemoryJuliaComputeProfile, MemoryJuliaGateScoreRequestRow,
        build_memory_julia_gate_score_request_batch, memory_julia_gate_score_response_schema,
    };
    use arrow::array::{Float32Array, StringArray};
    use arrow::record_batch::RecordBatch;
    use std::sync::Arc;

    #[test]
    fn validate_memory_julia_compute_request_batches_dispatches_by_profile() {
        let batch =
            build_memory_julia_gate_score_request_batch(&[MemoryJuliaGateScoreRequestRow {
                memory_id: "memory-1".to_string(),
                scenario_pack: Some("searchinfra".to_string()),
                react_revalidation_score: 0.9,
                graph_consistency_score: 0.8,
                omega_alignment_score: 0.85,
                q_value: 0.75,
                usage_count: 4,
                failure_rate: 0.25,
                ttl_score: 0.7,
                current_state: "active".to_string(),
            }])
            .unwrap_or_else(|error| panic!("request batch should build: {error}"));

        validate_memory_julia_compute_request_batches(
            MemoryJuliaComputeProfile::MemoryGateScore,
            &[batch],
        )
        .unwrap_or_else(|error| panic!("request validation should pass: {error}"));
    }

    #[test]
    fn validate_memory_julia_compute_response_batches_dispatches_by_profile() {
        let batch = RecordBatch::try_new(
            memory_julia_gate_score_response_schema(),
            vec![
                Arc::new(StringArray::from(vec!["memory-1"])),
                Arc::new(StringArray::from(vec!["retain"])),
                Arc::new(Float32Array::from(vec![0.9_f32])),
                Arc::new(Float32Array::from(vec![0.75_f32])),
                Arc::new(Float32Array::from(vec![0.7_f32])),
                Arc::new(StringArray::from(vec!["keep"])),
                Arc::new(StringArray::from(vec!["stable"])),
                Arc::new(StringArray::from(vec!["v1"])),
            ],
        )
        .unwrap_or_else(|error| panic!("response batch should build: {error}"));

        validate_memory_julia_compute_response_batches(
            MemoryJuliaComputeProfile::MemoryGateScore,
            &[batch],
        )
        .unwrap_or_else(|error| panic!("response validation should pass: {error}"));
    }
}
