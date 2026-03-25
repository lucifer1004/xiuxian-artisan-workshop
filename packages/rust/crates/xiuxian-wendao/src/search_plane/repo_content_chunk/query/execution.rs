use std::collections::HashMap;

use xiuxian_vector::{ColumnarScanOptions, VectorStore, VectorStoreError};

use crate::search_plane::ranking::{
    RetainedWindow, StreamingRerankSource, StreamingRerankTelemetry,
};

use super::RepoContentChunkCandidate;
use super::RepoContentChunkSearchError;
use super::helpers::should_use_fts;
use super::scan::collect_candidates;

pub(super) struct RepoContentChunkSearchExecution {
    pub(super) candidates: Vec<RepoContentChunkCandidate>,
    pub(super) telemetry: StreamingRerankTelemetry,
    pub(super) source: StreamingRerankSource,
}

pub(super) async fn execute_repo_content_search(
    store: &VectorStore,
    table_name: &str,
    raw_needle: &str,
    needle: &str,
    options: ColumnarScanOptions,
    window: RetainedWindow,
) -> Result<RepoContentChunkSearchExecution, RepoContentChunkSearchError> {
    let fts_eligible = should_use_fts(raw_needle);
    let mut telemetry = StreamingRerankTelemetry::new(window, options.batch_size, options.limit);
    let mut saw_fts_batch = false;
    let mut fell_back_to_scan = false;
    let mut best_by_path =
        HashMap::<String, RepoContentChunkCandidate>::with_capacity(window.target);

    if fts_eligible {
        match store
            .search_fts_batches_streaming(table_name, raw_needle, options.clone(), |batch| {
                saw_fts_batch = true;
                collect_candidates(
                    &batch,
                    raw_needle,
                    needle,
                    &mut best_by_path,
                    window,
                    &mut telemetry,
                )
            })
            .await
        {
            Ok(()) if saw_fts_batch => {}
            Ok(()) | Err(RepoContentChunkSearchError::Storage(VectorStoreError::LanceDB(_))) => {
                fell_back_to_scan = true;
                best_by_path.clear();
                store
                    .scan_record_batches_streaming(table_name, options, |batch| {
                        collect_candidates(
                            &batch,
                            raw_needle,
                            needle,
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
                    raw_needle,
                    needle,
                    &mut best_by_path,
                    window,
                    &mut telemetry,
                )
            })
            .await?;
    }

    Ok(RepoContentChunkSearchExecution {
        candidates: best_by_path.into_values().collect(),
        telemetry,
        source: match (fts_eligible, fell_back_to_scan) {
            (true, true) => StreamingRerankSource::FtsFallbackScan,
            (true, false) => StreamingRerankSource::Fts,
            (false, _) => StreamingRerankSource::Scan,
        },
    })
}
