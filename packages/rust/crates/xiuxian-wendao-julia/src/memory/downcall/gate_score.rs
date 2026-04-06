use xiuxian_wendao_core::repo_intelligence::RepoIntelligenceError;
use xiuxian_wendao_runtime::config::MemoryJuliaComputeRuntimeConfig;

use crate::memory::host::{
    MemoryGateScoreEvidenceRow, build_memory_gate_score_request_rows_from_evidence,
};
use crate::memory::{
    MemoryJuliaGateScoreRecommendationRow, fetch_memory_julia_gate_score_recommendation_rows,
};

/// Compose Rust gate-evidence staging plus the Julia `memory_gate_score`
/// downcall in one plugin-owned helper.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when host evidence staging fails, the
/// Flight roundtrip fails, or the Julia response cannot be decoded.
pub async fn fetch_gate_score_recommendation_rows_from_evidence(
    runtime: &MemoryJuliaComputeRuntimeConfig,
    evidence_rows: &[MemoryGateScoreEvidenceRow],
) -> Result<Vec<MemoryJuliaGateScoreRecommendationRow>, RepoIntelligenceError> {
    let request_rows = build_memory_gate_score_request_rows_from_evidence(evidence_rows)?;
    fetch_memory_julia_gate_score_recommendation_rows(runtime, request_rows.as_slice()).await
}

#[cfg(test)]
mod tests {
    use xiuxian_memory_engine::{MemoryLifecycleState, MemoryUtilityLedger};

    use super::fetch_gate_score_recommendation_rows_from_evidence;
    use crate::memory::host::MemoryGateScoreEvidenceRow;
    use crate::memory::test_support::{
        gate_score_response_batch, runtime_for_test, spawn_memory_service,
    };

    fn sample_evidence_rows() -> Vec<MemoryGateScoreEvidenceRow> {
        vec![MemoryGateScoreEvidenceRow {
            memory_id: "memory-1".to_string(),
            scenario_pack: Some("searchinfra".to_string()),
            ledger: MemoryUtilityLedger {
                react_revalidation_score: 0.9,
                graph_consistency_score: 0.8,
                omega_alignment_score: 0.85,
                ttl_score: 0.7,
                utility_score: 0.78,
                q_value: 0.75,
                usage_count: 4,
                failure_rate: 0.25,
            },
            current_state: MemoryLifecycleState::Active,
        }]
    }

    #[tokio::test]
    async fn fetch_gate_score_recommendation_rows_from_evidence_roundtrips() {
        let route = "/memory/gate_score";
        let (base_url, server) = spawn_memory_service(gate_score_response_batch()).await;
        let runtime = runtime_for_test(base_url, route);

        let rows =
            fetch_gate_score_recommendation_rows_from_evidence(&runtime, &sample_evidence_rows())
                .await
                .unwrap_or_else(|error| panic!("gate-score downcall should succeed: {error}"));

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].verdict, "retain");

        server.abort();
    }
}
