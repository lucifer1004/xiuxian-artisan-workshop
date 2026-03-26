use std::collections::HashMap;

use crate::search_plane::knowledge_section::query::errors::KnowledgeSectionSearchError;
use crate::search_plane::knowledge_section::query::ranking::{
    candidate_path_key, compare_candidates, nullable_value, score_candidate, string_column,
};
use crate::search_plane::ranking::{
    RetainedWindow, StreamingRerankTelemetry, trim_ranked_string_map,
};
use xiuxian_vector::LanceRecordBatch;

const MIN_RETAINED_PATHS: usize = 128;
const RETAINED_PATH_MULTIPLIER: usize = 8;

#[derive(Debug, Clone)]
pub(crate) struct KnowledgeCandidate {
    pub(crate) path: String,
    pub(crate) stem: String,
    pub(crate) score: f64,
    pub(crate) hit_json: String,
}

pub(crate) fn collect_candidates(
    batch: &LanceRecordBatch,
    query_text: &str,
    query_lower: &str,
    best_by_path: &mut HashMap<String, KnowledgeCandidate>,
    window: RetainedWindow,
    telemetry: &mut StreamingRerankTelemetry,
) -> Result<(), KnowledgeSectionSearchError> {
    telemetry.observe_batch(batch.num_rows());
    let path = string_column(batch, "path")?;
    let stem = string_column(batch, "stem")?;
    let title = string_column(batch, "title")?;
    let best_section = string_column(batch, "best_section")?;
    let search_text_folded = string_column(batch, "search_text_folded")?;
    let hit_json = string_column(batch, "hit_json")?;

    for row in 0..batch.num_rows() {
        let score = score_candidate(
            query_text,
            query_lower,
            stem.value(row),
            nullable_value(title, row),
            nullable_value(best_section, row),
            search_text_folded.value(row),
        );
        if score <= 0.0 {
            continue;
        }

        telemetry.observe_match();
        let candidate = KnowledgeCandidate {
            path: path.value(row).to_string(),
            stem: stem.value(row).to_string(),
            score,
            hit_json: hit_json.value(row).to_string(),
        };
        match best_by_path.get(candidate.path.as_str()) {
            Some(existing) if existing.score >= candidate.score => {}
            _ => {
                best_by_path.insert(candidate.path.clone(), candidate);
                telemetry.observe_working_set(best_by_path.len());
                if best_by_path.len() > window.threshold {
                    let before_len = best_by_path.len();
                    trim_ranked_string_map(
                        best_by_path,
                        window.target,
                        compare_candidates,
                        candidate_path_key,
                    );
                    telemetry.observe_trim(before_len, best_by_path.len());
                }
            }
        }
    }

    Ok(())
}

pub(crate) fn retained_window(limit: usize) -> RetainedWindow {
    RetainedWindow::new(limit, RETAINED_PATH_MULTIPLIER, MIN_RETAINED_PATHS)
}
