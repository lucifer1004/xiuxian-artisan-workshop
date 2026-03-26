use std::collections::HashMap;

use crate::gateway::studio::types::SearchHit;
use crate::search_plane::knowledge_section::query::candidates::{
    KnowledgeCandidate, collect_candidates, retained_window,
};
use crate::search_plane::knowledge_section::query::errors::KnowledgeSectionSearchError;
use crate::search_plane::knowledge_section::query::ranking::{compare_candidates, should_use_fts};
use crate::search_plane::ranking::{
    RetainedWindow, StreamingRerankSource, StreamingRerankTelemetry, sort_by_rank,
};
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};
use xiuxian_vector::{ColumnarScanOptions, VectorStore, VectorStoreError};

use crate::search_plane::knowledge_section::schema::projected_columns;

#[derive(Debug)]
struct KnowledgeSearchExecution {
    candidates: Vec<KnowledgeCandidate>,
    telemetry: StreamingRerankTelemetry,
    source: StreamingRerankSource,
}

/// Search knowledge-section hits using the active corpus epoch.
pub(crate) async fn search_knowledge_sections(
    service: &SearchPlaneService,
    query: &str,
    limit: usize,
) -> Result<Vec<SearchHit>, KnowledgeSectionSearchError> {
    let status = service
        .coordinator()
        .status_for(SearchCorpusKind::KnowledgeSection);
    let Some(active_epoch) = status.active_epoch else {
        return Err(KnowledgeSectionSearchError::NotReady);
    };

    let query_text = query.trim();
    if query_text.is_empty() {
        return Ok(Vec::new());
    }

    let store = service
        .open_store(SearchCorpusKind::KnowledgeSection)
        .await?;
    let table_name =
        SearchPlaneService::table_name(SearchCorpusKind::KnowledgeSection, active_epoch);
    let options = build_knowledge_scan_options(query_text, limit);
    let query_lower = query_text.to_ascii_lowercase();
    let execution = execute_knowledge_search(
        &store,
        table_name.as_str(),
        query_text,
        query_lower.as_str(),
        options,
        retained_window(limit),
    )
    .await?;
    let mut candidates = execution.candidates;
    sort_by_rank(&mut candidates, compare_candidates);
    candidates.truncate(limit);
    let hits = decode_knowledge_hits(candidates)?;
    service.record_query_telemetry(
        SearchCorpusKind::KnowledgeSection,
        execution
            .telemetry
            .finish(execution.source, None, hits.len()),
    );
    Ok(hits)
}

fn build_knowledge_scan_options(query_text: &str, limit: usize) -> ColumnarScanOptions {
    ColumnarScanOptions {
        projected_columns: projected_columns()
            .into_iter()
            .map(str::to_string)
            .collect(),
        batch_size: Some(256),
        limit: if should_use_fts(query_text) {
            Some(limit.saturating_mul(32).max(128))
        } else {
            None
        },
        ..ColumnarScanOptions::default()
    }
}

async fn execute_knowledge_search(
    store: &VectorStore,
    table_name: &str,
    query_text: &str,
    query_lower: &str,
    options: ColumnarScanOptions,
    window: RetainedWindow,
) -> Result<KnowledgeSearchExecution, KnowledgeSectionSearchError> {
    let fts_eligible = should_use_fts(query_text);
    let mut telemetry = StreamingRerankTelemetry::new(window, options.batch_size, options.limit);
    let mut saw_fts_batch = false;
    let mut fell_back_to_scan = false;
    let mut best_by_path = HashMap::<String, KnowledgeCandidate>::with_capacity(window.target);

    if fts_eligible {
        match store
            .search_fts_batches_streaming(table_name, query_text, options.clone(), |batch| {
                saw_fts_batch = true;
                collect_candidates(
                    &batch,
                    query_text,
                    query_lower,
                    &mut best_by_path,
                    window,
                    &mut telemetry,
                )
            })
            .await
        {
            Ok(()) if saw_fts_batch => {}
            Ok(()) | Err(KnowledgeSectionSearchError::Storage(VectorStoreError::LanceDB(_))) => {
                fell_back_to_scan = true;
                best_by_path.clear();
                store
                    .scan_record_batches_streaming(table_name, options, |batch| {
                        collect_candidates(
                            &batch,
                            query_text,
                            query_lower,
                            &mut best_by_path,
                            window,
                            &mut telemetry,
                        )
                    })
                    .await?;
            }
            Err(error) => return Err(error),
        }
    } else {
        store
            .scan_record_batches_streaming(table_name, options, |batch| {
                collect_candidates(
                    &batch,
                    query_text,
                    query_lower,
                    &mut best_by_path,
                    window,
                    &mut telemetry,
                )
            })
            .await?;
    }

    Ok(KnowledgeSearchExecution {
        candidates: best_by_path.into_values().collect(),
        telemetry,
        source: match (fts_eligible, fell_back_to_scan) {
            (true, true) => StreamingRerankSource::FtsFallbackScan,
            (true, false) => StreamingRerankSource::Fts,
            (false, _) => StreamingRerankSource::Scan,
        },
    })
}

fn decode_knowledge_hits(
    candidates: Vec<KnowledgeCandidate>,
) -> Result<Vec<SearchHit>, KnowledgeSectionSearchError> {
    candidates
        .into_iter()
        .map(|candidate| {
            let mut hit: SearchHit = serde_json::from_str(candidate.hit_json.as_str())
                .map_err(|error| KnowledgeSectionSearchError::Decode(error.to_string()))?;
            hit.score = candidate.score;
            Ok(hit)
        })
        .collect()
}
