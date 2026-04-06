use arrow::record_batch::RecordBatch;
use xiuxian_memory_engine::MemoryProjectionRow;
use xiuxian_wendao_core::repo_intelligence::RepoIntelligenceError;

use crate::memory::{
    MemoryJuliaEpisodicRecallRequestRow, build_memory_julia_episodic_recall_request_batch,
};

use super::common::{
    optional_text, required_text, validate_embedding, validate_finite, validate_non_negative_finite,
};

const SURFACE: &str = "memory Julia episodic_recall host staging";

/// Host-owned shared query inputs for one Julia `episodic_recall` downcall.
#[derive(Debug, Clone, PartialEq)]
pub struct EpisodicRecallQueryInputs {
    /// Host query id used as the join key across candidate rows.
    pub query_id: String,
    /// Optional scenario pack forwarded into the Julia compute lane.
    pub scenario_pack: Option<String>,
    /// Optional raw query text.
    pub query_text: Option<String>,
    /// Semantic embedding for the current query.
    pub query_embedding: Vec<f32>,
    /// Semantic recall tuning weight.
    pub k1: f32,
    /// Utility rerank tuning weight.
    pub k2: f32,
    /// Fusion lambda.
    pub lambda: f32,
    /// Minimum accepted score.
    pub min_score: f32,
}

/// Build typed Julia `episodic_recall` request rows from a Rust memory
/// projection snapshot.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the shared query inputs or any
/// projection row violate the staged `episodic_recall` contract.
pub fn build_episodic_recall_request_rows_from_projection(
    query: &EpisodicRecallQueryInputs,
    projection_rows: &[MemoryProjectionRow],
) -> Result<Vec<MemoryJuliaEpisodicRecallRequestRow>, RepoIntelligenceError> {
    let normalized_query = normalize_query_inputs(query)?;
    projection_rows
        .iter()
        .map(|row| build_request_row(&normalized_query, row))
        .collect()
}

/// Build one Julia `episodic_recall` request batch from a Rust memory
/// projection snapshot.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the projection is empty or any
/// staged row violates the Julia `episodic_recall` request contract.
pub fn build_episodic_recall_request_batch_from_projection(
    query: &EpisodicRecallQueryInputs,
    projection_rows: &[MemoryProjectionRow],
) -> Result<RecordBatch, RepoIntelligenceError> {
    let rows = build_episodic_recall_request_rows_from_projection(query, projection_rows)?;
    if rows.is_empty() {
        return Err(staging_error(
            "memory Julia episodic_recall host staging requires at least one projection row",
        ));
    }
    build_memory_julia_episodic_recall_request_batch(&rows)
}

#[derive(Debug, Clone, PartialEq)]
struct NormalizedEpisodicRecallQuery {
    query_id: String,
    scenario_pack: Option<String>,
    query_text: Option<String>,
    query_embedding: Vec<f32>,
    k1: f32,
    k2: f32,
    lambda: f32,
    min_score: f32,
}

fn normalize_query_inputs(
    query: &EpisodicRecallQueryInputs,
) -> Result<NormalizedEpisodicRecallQuery, RepoIntelligenceError> {
    let query_id = required_text(&query.query_id, "query_id", SURFACE)?;
    let scenario_pack = optional_text(query.scenario_pack.as_deref());
    let query_text = optional_text(query.query_text.as_deref());
    validate_embedding("query_embedding", &query.query_embedding, SURFACE)?;
    validate_non_negative_finite("k1", query.k1, SURFACE)?;
    validate_non_negative_finite("k2", query.k2, SURFACE)?;
    validate_non_negative_finite("lambda", query.lambda, SURFACE)?;
    validate_non_negative_finite("min_score", query.min_score, SURFACE)?;

    Ok(NormalizedEpisodicRecallQuery {
        query_id,
        scenario_pack,
        query_text,
        query_embedding: query.query_embedding.clone(),
        k1: query.k1,
        k2: query.k2,
        lambda: query.lambda,
        min_score: query.min_score,
    })
}

fn build_request_row(
    query: &NormalizedEpisodicRecallQuery,
    projection_row: &MemoryProjectionRow,
) -> Result<MemoryJuliaEpisodicRecallRequestRow, RepoIntelligenceError> {
    validate_projection_row(projection_row)?;

    Ok(MemoryJuliaEpisodicRecallRequestRow {
        query_id: query.query_id.clone(),
        scenario_pack: query.scenario_pack.clone(),
        scope: projection_row.scope.clone(),
        query_text: query.query_text.clone(),
        query_embedding: query.query_embedding.clone(),
        candidate_id: projection_row.episode_id.clone(),
        intent_embedding: projection_row.intent_embedding.clone(),
        q_value: projection_row.q_value,
        success_count: projection_row.success_count,
        failure_count: projection_row.failure_count,
        retrieval_count: projection_row.retrieval_count,
        created_at_ms: projection_row.created_at_ms,
        updated_at_ms: projection_row.updated_at_ms,
        k1: query.k1,
        k2: query.k2,
        lambda: query.lambda,
        min_score: query.min_score,
    })
}

fn validate_projection_row(row: &MemoryProjectionRow) -> Result<(), RepoIntelligenceError> {
    required_text(&row.episode_id, "candidate_id", SURFACE)?;
    required_text(&row.scope, "scope", SURFACE)?;
    validate_embedding("intent_embedding", &row.intent_embedding, SURFACE)?;
    validate_finite("q_value", row.q_value, SURFACE)?;
    if row.updated_at_ms < row.created_at_ms {
        return Err(staging_error(format!(
            "projection row `{}` has updated_at_ms earlier than created_at_ms",
            row.episode_id
        )));
    }
    Ok(())
}

fn staging_error(message: impl Into<String>) -> RepoIntelligenceError {
    super::common::staging_error(SURFACE, message)
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;
    use xiuxian_memory_engine::{Episode, EpisodeStore, MemoryProjectionFilter, StoreConfig};

    use super::{
        EpisodicRecallQueryInputs, build_episodic_recall_request_batch_from_projection,
        build_episodic_recall_request_rows_from_projection,
    };

    fn sample_query() -> EpisodicRecallQueryInputs {
        EpisodicRecallQueryInputs {
            query_id: "query-1".to_string(),
            scenario_pack: Some("searchinfra".to_string()),
            query_text: Some("how do we fix this".to_string()),
            query_embedding: vec![0.2, 0.4, 0.6],
            k1: 1.0,
            k2: 0.5,
            lambda: 0.7,
            min_score: 0.2,
        }
    }

    fn make_store() -> Result<(TempDir, EpisodeStore), Box<dyn std::error::Error>> {
        let temp = tempfile::tempdir()?;
        let store = EpisodeStore::new(StoreConfig {
            path: temp.path().to_string_lossy().to_string(),
            embedding_dim: 3,
            table_name: "host-staging".to_string(),
        });
        Ok((temp, store))
    }

    #[test]
    fn build_episodic_recall_request_rows_from_projection_maps_host_fields()
    -> Result<(), Box<dyn std::error::Error>> {
        let (_temp, store) = make_store()?;
        let mut episode = Episode::new_scoped(
            "episode-alpha".to_string(),
            "alpha intent".to_string(),
            vec![1.0, 0.0, 0.0],
            "alpha experience".to_string(),
            "pending".to_string(),
            "alpha",
        );
        episode.success_count = 3;
        episode.failure_count = 1;
        episode.retrieval_count = 4;
        episode.created_at = 100;
        episode.updated_at = 200;
        store.store(episode)?;
        let q_value = store.update_q("episode-alpha", 1.0);

        let projection_rows = store.memory_projection_rows(&MemoryProjectionFilter::default());
        let request_rows =
            build_episodic_recall_request_rows_from_projection(&sample_query(), &projection_rows)?;

        assert_eq!(request_rows.len(), 1);
        let row = &request_rows[0];
        assert_eq!(row.query_id, "query-1");
        assert_eq!(row.scenario_pack.as_deref(), Some("searchinfra"));
        assert_eq!(row.query_text.as_deref(), Some("how do we fix this"));
        assert_eq!(row.scope, "alpha");
        assert_eq!(row.candidate_id, "episode-alpha");
        assert_eq!(row.intent_embedding, vec![1.0, 0.0, 0.0]);
        assert!((row.q_value - q_value).abs() < 1e-6);
        assert_eq!(row.success_count, 3);
        assert_eq!(row.failure_count, 1);
        assert_eq!(row.retrieval_count, 4);
        assert_eq!(row.created_at_ms, 100);
        assert_eq!(row.updated_at_ms, 200);

        Ok(())
    }

    #[test]
    fn build_episodic_recall_request_batch_from_projection_materializes_staged_contract()
    -> Result<(), Box<dyn std::error::Error>> {
        let (_temp, store) = make_store()?;

        for episode_id in ["episode-alpha", "episode-beta"] {
            let episode = Episode::new_scoped(
                episode_id.to_string(),
                format!("{episode_id} intent"),
                vec![0.1, 0.2, 0.3],
                format!("{episode_id} experience"),
                "pending".to_string(),
                "alpha",
            );
            store.store(episode)?;
        }

        let projection_rows = store.memory_projection_rows(&MemoryProjectionFilter::default());
        let batch =
            build_episodic_recall_request_batch_from_projection(&sample_query(), &projection_rows)?;

        assert_eq!(batch.num_rows(), 2);
        assert_eq!(batch.schema().fields().len(), 17);
        assert!(batch.column_by_name("query_id").is_some());
        assert!(batch.column_by_name("candidate_id").is_some());
        assert!(batch.column_by_name("intent_embedding").is_some());

        Ok(())
    }

    #[test]
    fn build_episodic_recall_request_batch_from_projection_rejects_invalid_query_inputs() {
        let mut query = sample_query();
        query.query_id = "   ".to_string();
        let Err(error) = build_episodic_recall_request_batch_from_projection(&query, &[]) else {
            panic!("blank query_id must fail");
        };

        assert!(error.to_string().contains("query_id"));
    }

    #[test]
    fn build_episodic_recall_request_batch_from_real_store_projection_respects_scope_filter()
    -> Result<(), Box<dyn std::error::Error>> {
        let (_temp, store) = make_store()?;

        for (episode_id, scope) in [("episode-alpha", "alpha"), ("episode-beta", "beta")] {
            let episode = Episode::new_scoped(
                episode_id.to_string(),
                format!("{scope} intent"),
                vec![0.3, 0.2, 0.1],
                format!("{scope} experience"),
                "pending".to_string(),
                scope,
            );
            store.store(episode)?;
        }

        let projection_rows = store.memory_projection_rows(&MemoryProjectionFilter {
            scope: Some("alpha".to_string()),
            limit: None,
        });
        let batch =
            build_episodic_recall_request_batch_from_projection(&sample_query(), &projection_rows)?;
        let request_rows =
            build_episodic_recall_request_rows_from_projection(&sample_query(), &projection_rows)?;

        assert_eq!(batch.num_rows(), 1);
        assert_eq!(request_rows.len(), 1);
        assert_eq!(request_rows[0].candidate_id, "episode-alpha");
        assert_eq!(request_rows[0].scope, "alpha");

        Ok(())
    }
}
