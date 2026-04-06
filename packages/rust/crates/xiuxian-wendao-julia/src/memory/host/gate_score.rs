use arrow::record_batch::RecordBatch;
use xiuxian_memory_engine::{Episode, EpisodeStore, MemoryLifecycleState, MemoryUtilityLedger};
use xiuxian_wendao_core::repo_intelligence::RepoIntelligenceError;

use crate::memory::{MemoryJuliaGateScoreRequestRow, build_memory_julia_gate_score_request_batch};

use super::common::{optional_text, required_text, validate_probability};

const SURFACE: &str = "memory Julia memory_gate_score host staging";

/// Host-owned evidence row for one Julia `memory_gate_score` downcall.
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryGateScoreEvidenceRow {
    /// Stable host memory id used as the join key across recommendation rows.
    pub memory_id: String,
    /// Optional scenario pack forwarded into the Julia compute lane.
    pub scenario_pack: Option<String>,
    /// Rust-owned utility ledger for the target memory item.
    pub ledger: MemoryUtilityLedger,
    /// Current Rust-owned lifecycle state.
    pub current_state: MemoryLifecycleState,
}

/// Build typed Julia `memory_gate_score` request rows from Rust-owned gate
/// evidence.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when any host evidence row violates the
/// staged `memory_gate_score` request contract.
pub fn build_memory_gate_score_request_rows_from_evidence(
    evidence_rows: &[MemoryGateScoreEvidenceRow],
) -> Result<Vec<MemoryJuliaGateScoreRequestRow>, RepoIntelligenceError> {
    evidence_rows.iter().map(build_request_row).collect()
}

/// Build one Julia `memory_gate_score` request batch from Rust-owned gate
/// evidence.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the evidence is empty or any staged
/// row violates the Julia `memory_gate_score` request contract.
pub fn build_memory_gate_score_request_batch_from_evidence(
    evidence_rows: &[MemoryGateScoreEvidenceRow],
) -> Result<RecordBatch, RepoIntelligenceError> {
    let rows = build_memory_gate_score_request_rows_from_evidence(evidence_rows)?;
    if rows.is_empty() {
        return Err(staging_error(
            "memory Julia memory_gate_score host staging requires at least one evidence row",
        ));
    }
    build_memory_julia_gate_score_request_batch(&rows)
}

/// Build one canonical gate-score evidence row from a host episode plus
/// already-computed evidence signals.
#[must_use]
pub fn build_memory_gate_score_evidence_row_from_episode(
    episode: &Episode,
    scenario_pack: Option<String>,
    react_revalidation_score: f32,
    graph_consistency_score: f32,
    omega_alignment_score: f32,
    current_state: MemoryLifecycleState,
) -> MemoryGateScoreEvidenceRow {
    MemoryGateScoreEvidenceRow {
        memory_id: episode.id.clone(),
        scenario_pack,
        ledger: MemoryUtilityLedger::from_episode(
            episode,
            react_revalidation_score,
            graph_consistency_score,
            omega_alignment_score,
        ),
        current_state,
    }
}

/// Build one canonical gate-score evidence row from a stored episode id.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the requested episode does not
/// exist in the store.
pub fn build_memory_gate_score_evidence_row_from_store(
    store: &EpisodeStore,
    memory_id: &str,
    scenario_pack: Option<String>,
    react_revalidation_score: f32,
    graph_consistency_score: f32,
    omega_alignment_score: f32,
    current_state: MemoryLifecycleState,
) -> Result<MemoryGateScoreEvidenceRow, RepoIntelligenceError> {
    let Some(episode) = store.get(memory_id) else {
        return Err(staging_error(format!(
            "memory Julia memory_gate_score host staging could not find episode `{}`",
            memory_id.trim()
        )));
    };

    Ok(build_memory_gate_score_evidence_row_from_episode(
        &episode,
        scenario_pack,
        react_revalidation_score,
        graph_consistency_score,
        omega_alignment_score,
        current_state,
    ))
}

fn build_request_row(
    evidence_row: &MemoryGateScoreEvidenceRow,
) -> Result<MemoryJuliaGateScoreRequestRow, RepoIntelligenceError> {
    let memory_id = required_text(&evidence_row.memory_id, "memory_id", SURFACE)?;
    let scenario_pack = optional_text(evidence_row.scenario_pack.as_deref());
    validate_probability(
        "react_revalidation_score",
        evidence_row.ledger.react_revalidation_score,
        SURFACE,
    )?;
    validate_probability(
        "graph_consistency_score",
        evidence_row.ledger.graph_consistency_score,
        SURFACE,
    )?;
    validate_probability(
        "omega_alignment_score",
        evidence_row.ledger.omega_alignment_score,
        SURFACE,
    )?;
    validate_probability("q_value", evidence_row.ledger.q_value, SURFACE)?;
    validate_probability("failure_rate", evidence_row.ledger.failure_rate, SURFACE)?;
    validate_probability("ttl_score", evidence_row.ledger.ttl_score, SURFACE)?;

    Ok(MemoryJuliaGateScoreRequestRow {
        memory_id,
        scenario_pack,
        react_revalidation_score: evidence_row.ledger.react_revalidation_score,
        graph_consistency_score: evidence_row.ledger.graph_consistency_score,
        omega_alignment_score: evidence_row.ledger.omega_alignment_score,
        q_value: evidence_row.ledger.q_value,
        usage_count: evidence_row.ledger.usage_count,
        failure_rate: evidence_row.ledger.failure_rate,
        ttl_score: evidence_row.ledger.ttl_score,
        current_state: evidence_row.current_state.as_str().to_string(),
    })
}

fn staging_error(message: impl Into<String>) -> RepoIntelligenceError {
    super::common::staging_error(SURFACE, message)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use xiuxian_memory_engine::{
        Episode, EpisodeStore, MemoryLifecycleState, MemoryUtilityLedger, StoreConfig,
    };

    use super::{
        MemoryGateScoreEvidenceRow, build_memory_gate_score_evidence_row_from_episode,
        build_memory_gate_score_evidence_row_from_store,
        build_memory_gate_score_request_batch_from_evidence,
        build_memory_gate_score_request_rows_from_evidence,
    };

    fn make_store() -> Result<(TempDir, EpisodeStore), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let store = EpisodeStore::new(StoreConfig {
            path: temp.path().to_string_lossy().to_string(),
            embedding_dim: 3,
            table_name: "gate-score-host-staging".to_string(),
        });
        Ok((temp, store))
    }

    fn sample_episode(memory_id: &str) -> Episode {
        let mut episode = Episode::new_scoped(
            memory_id.to_string(),
            "intent".to_string(),
            vec![0.1, 0.2, 0.3],
            "experience".to_string(),
            "completed".to_string(),
            "alpha",
        );
        episode.q_value = 0.84;
        episode.success_count = 5;
        episode.failure_count = 1;
        episode.retrieval_count = 6;
        episode
    }

    #[test]
    fn build_memory_gate_score_request_rows_from_evidence_maps_host_fields()
    -> Result<(), Box<dyn std::error::Error>> {
        let evidence = build_memory_gate_score_evidence_row_from_episode(
            &sample_episode("memory-alpha"),
            Some("searchinfra".to_string()),
            0.91,
            0.88,
            0.93,
            MemoryLifecycleState::Active,
        );

        let rows = build_memory_gate_score_request_rows_from_evidence(&[evidence])?;

        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.memory_id, "memory-alpha");
        assert_eq!(row.scenario_pack.as_deref(), Some("searchinfra"));
        assert!((row.react_revalidation_score - 0.91).abs() < 1e-6);
        assert!((row.graph_consistency_score - 0.88).abs() < 1e-6);
        assert!((row.omega_alignment_score - 0.93).abs() < 1e-6);
        assert!((row.q_value - 0.84).abs() < 1e-6);
        assert_eq!(row.usage_count, 6);
        assert!((row.failure_rate - (1.0 / 6.0)).abs() < 1e-6);
        assert!(row.ttl_score > 0.0);
        assert_eq!(row.current_state, "active");

        Ok(())
    }

    #[test]
    fn build_memory_gate_score_request_batch_from_evidence_materializes_staged_contract()
    -> Result<(), Box<dyn std::error::Error>> {
        let evidence_rows = vec![
            build_memory_gate_score_evidence_row_from_episode(
                &sample_episode("memory-alpha"),
                Some("searchinfra".to_string()),
                0.91,
                0.88,
                0.93,
                MemoryLifecycleState::Active,
            ),
            build_memory_gate_score_evidence_row_from_episode(
                &sample_episode("memory-beta"),
                None,
                0.77,
                0.74,
                0.81,
                MemoryLifecycleState::Cooling,
            ),
        ];

        let batch = build_memory_gate_score_request_batch_from_evidence(&evidence_rows)?;

        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.schema().fields().len(), 10);
        assert!(batch.column_by_name("memory_id").is_some());
        assert!(batch.column_by_name("ttl_score").is_some());
        assert!(batch.column_by_name("current_state").is_some());

        Ok(())
    }

    #[test]
    fn build_memory_gate_score_request_batch_from_evidence_rejects_invalid_memory_id() {
        let evidence_rows = vec![MemoryGateScoreEvidenceRow {
            memory_id: "   ".to_string(),
            scenario_pack: None,
            ledger: MemoryUtilityLedger {
                react_revalidation_score: 0.9,
                graph_consistency_score: 0.8,
                omega_alignment_score: 0.7,
                ttl_score: 0.6,
                utility_score: 0.75,
                q_value: 0.85,
                usage_count: 4,
                failure_rate: 0.2,
            },
            current_state: MemoryLifecycleState::RevalidatePending,
        }];

        let Err(error) = build_memory_gate_score_request_batch_from_evidence(&evidence_rows) else {
            panic!("blank memory_id must fail");
        };

        assert!(error.to_string().contains("memory_id"));
    }

    #[test]
    fn build_memory_gate_score_evidence_row_from_store_roundtrips_real_episode()
    -> Result<(), Box<dyn std::error::Error>> {
        let (_temp, store) = make_store()?;
        store.store(sample_episode("memory-alpha"))?;

        let evidence = build_memory_gate_score_evidence_row_from_store(
            &store,
            "memory-alpha",
            Some("searchinfra".to_string()),
            0.91,
            0.88,
            0.93,
            MemoryLifecycleState::RevalidatePending,
        )?;
        let rows = build_memory_gate_score_request_rows_from_evidence(&[evidence])?;

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].memory_id, "memory-alpha");
        assert_eq!(rows[0].usage_count, 6);
        assert_eq!(rows[0].current_state, "revalidate_pending");
        assert!((rows[0].failure_rate - (1.0 / 6.0)).abs() < 1e-6);

        Ok(())
    }
}
