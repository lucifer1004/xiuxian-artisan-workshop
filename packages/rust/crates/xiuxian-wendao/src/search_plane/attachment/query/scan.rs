use std::collections::HashSet;

use xiuxian_vector::{
    ColumnarScanOptions, LanceArray, LanceRecordBatch, LanceStringArray, VectorStore,
};

use crate::search_plane::attachment::query::scoring::{
    candidate_score, compare_candidates, should_use_fts,
};
use crate::search_plane::attachment::query::types::{
    AttachmentCandidate, AttachmentCandidateQuery, AttachmentSearchError, AttachmentSearchExecution,
};
use crate::search_plane::attachment::schema::{
    attachment_ext_column, attachment_name_column, attachment_name_folded_column, hit_json_column,
    kind_column, projected_columns_with_hit_json,
};
use crate::search_plane::ranking::{
    StreamingRerankSource, StreamingRerankTelemetry, trim_ranked_vec,
};

pub(crate) async fn execute_attachment_search(
    store: &VectorStore,
    table_name: &str,
    query_text: &str,
    options: ColumnarScanOptions,
    candidate_query: &AttachmentCandidateQuery<'_>,
    fts_eligible: bool,
) -> Result<AttachmentSearchExecution, AttachmentSearchError> {
    let mut telemetry =
        StreamingRerankTelemetry::new(candidate_query.window, options.batch_size, options.limit);
    let mut candidates = Vec::with_capacity(candidate_query.window.target);
    let mut saw_fts_batch = false;
    let mut fell_back_to_scan = false;

    if fts_eligible {
        match store
            .search_fts_batches_streaming(table_name, query_text, options.clone(), |batch| {
                saw_fts_batch = true;
                collect_candidates(&batch, candidate_query, &mut candidates, &mut telemetry)
            })
            .await
        {
            Ok(()) if saw_fts_batch => {}
            Ok(()) => {
                fell_back_to_scan = true;
                candidates.clear();
                store
                    .scan_record_batches_streaming(table_name, options, |batch| {
                        collect_candidates(&batch, candidate_query, &mut candidates, &mut telemetry)
                    })
                    .await?;
            }
            Err(error) => return Err(error),
        }
    } else {
        store
            .scan_record_batches_streaming(table_name, options, |batch| {
                collect_candidates(&batch, candidate_query, &mut candidates, &mut telemetry)
            })
            .await?;
    }

    Ok(AttachmentSearchExecution {
        candidates,
        telemetry,
        source: match (fts_eligible, fell_back_to_scan) {
            (true, true) => StreamingRerankSource::FtsFallbackScan,
            (true, false) => StreamingRerankSource::Fts,
            (false, _) => StreamingRerankSource::Scan,
        },
    })
}

fn collect_candidates(
    batch: &LanceRecordBatch,
    query: &AttachmentCandidateQuery<'_>,
    candidates: &mut Vec<AttachmentCandidate>,
    telemetry: &mut StreamingRerankTelemetry,
) -> Result<(), AttachmentSearchError> {
    telemetry.observe_batch(batch.num_rows());
    let source_path = string_column(batch, "source_path")?;
    let source_title = string_column(batch, "source_title")?;
    let source_stem = string_column(batch, "source_stem")?;
    let attachment_path = string_column(batch, "attachment_path")?;
    let attachment_name = string_column(batch, attachment_name_column())?;
    let attachment_ext = string_column(batch, attachment_ext_column())?;
    let kind = string_column(batch, kind_column())?;
    let source_path_folded = string_column(batch, "source_path_folded")?;
    let source_title_folded = string_column(batch, "source_title_folded")?;
    let source_stem_folded = string_column(batch, "source_stem_folded")?;
    let attachment_path_folded = string_column(batch, "attachment_path_folded")?;
    let attachment_name_folded = string_column(batch, attachment_name_folded_column())?;
    let hit_json = string_column(batch, hit_json_column())?;

    for row in 0..batch.num_rows() {
        if !query.extensions.is_empty() && !query.extensions.contains(attachment_ext.value(row)) {
            continue;
        }
        if !query.kinds.is_empty() && !query.kinds.contains(kind.value(row)) {
            continue;
        }

        let fields = if query.case_sensitive {
            [
                attachment_path.value(row),
                attachment_name.value(row),
                source_path.value(row),
                source_title.value(row),
                source_stem.value(row),
            ]
        } else {
            [
                attachment_path_folded.value(row),
                attachment_name_folded.value(row),
                source_path_folded.value(row),
                source_title_folded.value(row),
                source_stem_folded.value(row),
            ]
        };
        let score = candidate_score(query.normalized_query, query.query_tokens, &fields);
        if score <= 0.0 {
            continue;
        }

        telemetry.observe_match();
        candidates.push(AttachmentCandidate {
            score,
            source_path: source_path.value(row).to_string(),
            attachment_path: attachment_path.value(row).to_string(),
            hit_json: hit_json.value(row).to_string(),
        });
        telemetry.observe_working_set(candidates.len());
        if candidates.len() > query.window.threshold {
            let before_len = candidates.len();
            trim_ranked_vec(candidates, query.window.target, compare_candidates);
            telemetry.observe_trim(before_len, candidates.len());
        }
    }

    Ok(())
}

pub(crate) fn build_attachment_scan_options(
    query_text: &str,
    limit: usize,
    case_sensitive: bool,
    normalized_extensions: &HashSet<String>,
    normalized_kinds: &HashSet<String>,
) -> ColumnarScanOptions {
    ColumnarScanOptions {
        where_filter: filter_expression(normalized_extensions, normalized_kinds),
        projected_columns: projected_columns_with_hit_json()
            .into_iter()
            .map(str::to_string)
            .collect(),
        batch_size: Some(256),
        limit: if case_sensitive || !should_use_fts(query_text) {
            None
        } else {
            Some(limit.saturating_mul(32).max(128))
        },
        ..ColumnarScanOptions::default()
    }
}

fn filter_expression(extensions: &HashSet<String>, kinds: &HashSet<String>) -> Option<String> {
    let extension_clause = disjunction(attachment_ext_column(), extensions);
    let kind_clause = disjunction(kind_column(), kinds);
    match (extension_clause, kind_clause) {
        (Some(left), Some(right)) => Some(format!("({left}) AND ({right})")),
        (Some(clause), None) | (None, Some(clause)) => Some(clause),
        (None, None) => None,
    }
}

fn disjunction(column: &str, values: &HashSet<String>) -> Option<String> {
    if values.is_empty() {
        return None;
    }

    let mut sorted = values.iter().cloned().collect::<Vec<_>>();
    sorted.sort_unstable();
    Some(
        sorted
            .into_iter()
            .map(|value| format!("{column} = '{}'", lance_string_literal(value.as_str())))
            .collect::<Vec<_>>()
            .join(" OR "),
    )
}

fn lance_string_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn string_column<'a>(
    batch: &'a LanceRecordBatch,
    name: &str,
) -> Result<&'a LanceStringArray, AttachmentSearchError> {
    batch
        .column_by_name(name)
        .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
        .ok_or_else(|| AttachmentSearchError::Decode(format!("missing string column `{name}`")))
}
