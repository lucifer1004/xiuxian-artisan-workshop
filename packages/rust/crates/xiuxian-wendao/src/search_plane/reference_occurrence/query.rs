use xiuxian_vector::{
    ColumnarScanOptions, LanceArray, LanceRecordBatch, LanceStringArray, LanceUInt64Array,
    VectorStoreError,
};

use crate::gateway::studio::search::support::score_reference_hit;
use crate::gateway::studio::types::ReferenceSearchHit;
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

use super::schema::{filter_column, projected_columns};

#[cfg(test)]
use super::schema::{reference_occurrence_batches, reference_occurrence_schema};

#[derive(Debug, thiserror::Error)]
pub(crate) enum ReferenceOccurrenceSearchError {
    #[error("reference occurrence index has no published epoch")]
    NotReady,
    #[error(transparent)]
    Storage(#[from] VectorStoreError),
    #[error("{0}")]
    Decode(String),
}

pub(crate) async fn search_reference_occurrences(
    service: &SearchPlaneService,
    query: &str,
    limit: usize,
) -> Result<Vec<ReferenceSearchHit>, ReferenceOccurrenceSearchError> {
    let status = service
        .coordinator()
        .status_for(SearchCorpusKind::ReferenceOccurrence);
    let Some(active_epoch) = status.active_epoch else {
        return Err(ReferenceOccurrenceSearchError::NotReady);
    };

    let normalized_query = query.trim().to_ascii_lowercase();
    if normalized_query.is_empty() {
        return Ok(Vec::new());
    }

    let store = service
        .open_store(SearchCorpusKind::ReferenceOccurrence)
        .await?;
    let table_name = service.table_name(SearchCorpusKind::ReferenceOccurrence, active_epoch);
    let batches = store
        .scan_record_batches(
            table_name.as_str(),
            ColumnarScanOptions {
                where_filter: Some(format!(
                    "{} = '{}'",
                    filter_column(),
                    lance_string_literal(normalized_query.as_str())
                )),
                projected_columns: projected_columns()
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                batch_size: Some(256),
                ..ColumnarScanOptions::default()
            },
        )
        .await?;

    let mut candidates = Vec::new();
    for batch in &batches {
        collect_candidates(batch, query, &mut candidates)?;
    }

    candidates.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.column.cmp(&right.column))
    });
    candidates.truncate(limit);

    candidates
        .into_iter()
        .map(|candidate| {
            let mut hit: ReferenceSearchHit = serde_json::from_str(candidate.hit_json.as_str())
                .map_err(|error| ReferenceOccurrenceSearchError::Decode(error.to_string()))?;
            hit.score = candidate.score;
            Ok(hit)
        })
        .collect()
}

#[derive(Debug)]
struct ReferenceOccurrenceCandidate {
    score: f64,
    path: String,
    line: usize,
    column: usize,
    hit_json: String,
}

fn collect_candidates(
    batch: &LanceRecordBatch,
    query: &str,
    candidates: &mut Vec<ReferenceOccurrenceCandidate>,
) -> Result<(), ReferenceOccurrenceSearchError> {
    let path = string_column(batch, "path")?;
    let line = u64_column(batch, "line")?;
    let column = u64_column(batch, "column")?;
    let line_text = string_column(batch, "line_text")?;
    let hit_json = string_column(batch, "hit_json")?;

    for row in 0..batch.num_rows() {
        candidates.push(ReferenceOccurrenceCandidate {
            score: score_reference_hit(line_text.value(row), query),
            path: path.value(row).to_string(),
            line: usize::try_from(line.value(row)).unwrap_or(usize::MAX),
            column: usize::try_from(column.value(row)).unwrap_or(usize::MAX),
            hit_json: hit_json.value(row).to_string(),
        });
    }

    Ok(())
}

fn lance_string_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn string_column<'a>(
    batch: &'a LanceRecordBatch,
    name: &str,
) -> Result<&'a LanceStringArray, ReferenceOccurrenceSearchError> {
    batch
        .column_by_name(name)
        .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
        .ok_or_else(|| {
            ReferenceOccurrenceSearchError::Decode(format!("missing string column `{name}`"))
        })
}

fn u64_column<'a>(
    batch: &'a LanceRecordBatch,
    name: &str,
) -> Result<&'a LanceUInt64Array, ReferenceOccurrenceSearchError> {
    batch
        .column_by_name(name)
        .and_then(|column| column.as_any().downcast_ref::<LanceUInt64Array>())
        .ok_or_else(|| {
            ReferenceOccurrenceSearchError::Decode(format!("missing u64 column `{name}`"))
        })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::gateway::studio::types::{ReferenceSearchHit, StudioNavigationTarget};
    use crate::search_plane::{
        BeginBuildDecision, SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace,
        SearchPlaneService,
    };

    use super::*;

    fn fixture_service(temp_dir: &tempfile::TempDir) -> SearchPlaneService {
        SearchPlaneService::with_paths(
            PathBuf::from("/tmp/project"),
            temp_dir.path().join("search_plane"),
            SearchManifestKeyspace::new("xiuxian:test:reference_occurrence"),
            SearchMaintenancePolicy::default(),
        )
    }

    fn sample_hit(name: &str, path: &str, line: usize) -> ReferenceSearchHit {
        ReferenceSearchHit {
            name: name.to_string(),
            path: path.to_string(),
            language: "rust".to_string(),
            crate_name: "kernel".to_string(),
            project_name: None,
            root_label: None,
            navigation_target: StudioNavigationTarget {
                path: path.to_string(),
                category: "doc".to_string(),
                project_name: None,
                root_label: None,
                line: Some(line),
                line_end: Some(line),
                column: Some(5),
            },
            line,
            column: 5,
            line_text: format!("let _value = {name};"),
            score: 0.0,
        }
    }

    #[tokio::test]
    async fn reference_occurrence_query_reads_hits_from_published_epoch() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let service = fixture_service(&temp_dir);
        let lease = match service.coordinator().begin_build(
            SearchCorpusKind::ReferenceOccurrence,
            "fp-1",
            SearchCorpusKind::ReferenceOccurrence.schema_version(),
        ) {
            BeginBuildDecision::Started(lease) => lease,
            other => panic!("unexpected begin decision: {other:?}"),
        };
        let hits = vec![
            sample_hit("AlphaService", "src/lib.rs", 10),
            sample_hit("BetaThing", "src/beta.rs", 20),
        ];
        let store = service
            .open_store(SearchCorpusKind::ReferenceOccurrence)
            .await
            .expect("open store");
        let table_name = service.table_name(SearchCorpusKind::ReferenceOccurrence, lease.epoch);
        store
            .replace_record_batches(
                table_name.as_str(),
                reference_occurrence_schema(),
                reference_occurrence_batches(&hits).expect("batches"),
            )
            .await
            .expect("replace record batches");
        service
            .coordinator()
            .publish_ready(&lease, hits.len() as u64, 1);

        let results = search_reference_occurrences(&service, "AlphaService", 5)
            .await
            .expect("query should succeed");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "AlphaService");
        assert!(results[0].score > 0.0);
    }
}
