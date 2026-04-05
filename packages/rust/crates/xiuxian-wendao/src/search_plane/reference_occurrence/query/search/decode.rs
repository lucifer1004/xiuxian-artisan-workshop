use std::collections::BTreeMap;

use crate::gateway::studio::types::ReferenceSearchHit;
use crate::search_plane::reference_occurrence::ReferenceOccurrenceSearchError;
use xiuxian_vector::SearchEngineContext;

use super::candidates::ReferenceOccurrenceCandidate;
use super::helpers::{sql_string_literal, string_column};

pub(super) async fn decode_reference_hits(
    engine: &SearchEngineContext,
    table_name: &str,
    candidates: Vec<ReferenceOccurrenceCandidate>,
) -> Result<Vec<ReferenceSearchHit>, ReferenceOccurrenceSearchError> {
    let payloads = load_hit_payloads_by_id(engine, table_name, candidates.as_slice()).await?;
    candidates
        .into_iter()
        .map(|candidate| {
            let hit_json = payloads.get(candidate.id.as_str()).ok_or_else(|| {
                ReferenceOccurrenceSearchError::Decode(format!(
                    "reference occurrence hydration missing payload for id `{}`",
                    candidate.id
                ))
            })?;
            let mut hit: ReferenceSearchHit = serde_json::from_str(hit_json.as_str())
                .map_err(|error| ReferenceOccurrenceSearchError::Decode(error.to_string()))?;
            hit.score = candidate.score;
            Ok(hit)
        })
        .collect()
}

async fn load_hit_payloads_by_id(
    engine: &SearchEngineContext,
    table_name: &str,
    candidates: &[ReferenceOccurrenceCandidate],
) -> Result<BTreeMap<String, String>, ReferenceOccurrenceSearchError> {
    if candidates.is_empty() {
        return Ok(BTreeMap::new());
    }

    let sql = format!(
        "SELECT {id_column}, {hit_json_column} FROM {table_name} WHERE {id_column} IN ({ids})",
        id_column = crate::search_plane::reference_occurrence::schema::id_column(),
        hit_json_column = crate::search_plane::reference_occurrence::schema::hit_json_column(),
        ids = candidates
            .iter()
            .map(|candidate| sql_string_literal(candidate.id.as_str()))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let batches = engine.sql_batches(sql.as_str()).await?;
    let mut payloads = BTreeMap::new();

    for batch in batches {
        let id = string_column(
            &batch,
            crate::search_plane::reference_occurrence::schema::id_column(),
        )?;
        let hit_json = string_column(
            &batch,
            crate::search_plane::reference_occurrence::schema::hit_json_column(),
        )?;
        for row in 0..batch.num_rows() {
            payloads.insert(id.value(row).to_string(), hit_json.value(row).to_string());
        }
    }

    Ok(payloads)
}
