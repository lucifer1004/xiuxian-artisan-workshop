use std::collections::HashSet;

use xiuxian_vector::{
    ColumnarScanOptions, LanceArray, LanceRecordBatch, LanceStringArray, VectorStore,
    VectorStoreError,
};

use crate::gateway::studio::types::AttachmentSearchHit;
use crate::search_plane::ranking::{
    RetainedWindow, StreamingRerankSource, StreamingRerankTelemetry, sort_by_rank, trim_ranked_vec,
};
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

use super::build::attachment_kind_label;
use super::schema::{
    attachment_ext_column, attachment_name_column, attachment_name_folded_column, hit_json_column,
    kind_column, projected_columns_with_hit_json,
};

const MIN_RETAINED_ATTACHMENTS: usize = 32;
const RETAINED_ATTACHMENT_MULTIPLIER: usize = 2;

#[cfg(test)]
use super::schema::{attachment_batches, attachment_schema, search_text_column};

#[derive(Debug, thiserror::Error)]
pub(crate) enum AttachmentSearchError {
    #[error("attachment index has no published epoch")]
    NotReady,
    #[error(transparent)]
    Storage(#[from] VectorStoreError),
    #[error("{0}")]
    Decode(String),
}

pub(crate) async fn search_attachment_hits(
    service: &SearchPlaneService,
    query: &str,
    limit: usize,
    extensions: &[String],
    kinds: &[crate::link_graph::LinkGraphAttachmentKind],
    case_sensitive: bool,
) -> Result<Vec<AttachmentSearchHit>, AttachmentSearchError> {
    let status = service
        .coordinator()
        .status_for(SearchCorpusKind::Attachment);
    let Some(active_epoch) = status.active_epoch else {
        return Err(AttachmentSearchError::NotReady);
    };

    let query_text = query.trim();
    if query_text.is_empty() {
        return Ok(Vec::new());
    }

    let normalized_extensions = normalize_extension_filters(extensions);
    let normalized_kinds = normalize_kind_filters(kinds);

    let store = service.open_store(SearchCorpusKind::Attachment).await?;
    let table_name = SearchPlaneService::table_name(SearchCorpusKind::Attachment, active_epoch);
    let options = build_attachment_scan_options(
        query_text,
        limit,
        case_sensitive,
        &normalized_extensions,
        &normalized_kinds,
    );
    let normalized_query = if case_sensitive {
        query_text.to_string()
    } else {
        query_text.to_ascii_lowercase()
    };
    let query_tokens = build_query_tokens(normalized_query.as_str());
    let candidate_query = AttachmentCandidateQuery {
        case_sensitive,
        normalized_query: normalized_query.as_str(),
        query_tokens: query_tokens.as_slice(),
        extensions: &normalized_extensions,
        kinds: &normalized_kinds,
        window: retained_window(limit),
    };
    let fts_eligible = !case_sensitive && should_use_fts(query_text);
    let execution = execute_attachment_search(
        &store,
        table_name.as_str(),
        query_text,
        options,
        &candidate_query,
        fts_eligible,
    )
    .await?;
    let mut candidates = execution.candidates;
    sort_by_rank(&mut candidates, compare_candidates);
    candidates.truncate(limit);
    let hits = decode_attachment_hits(candidates)?;
    service.record_query_telemetry(
        SearchCorpusKind::Attachment,
        execution
            .telemetry
            .finish(execution.source, None, hits.len()),
    );
    Ok(hits)
}

#[derive(Debug, Clone)]
struct AttachmentCandidate {
    score: f64,
    source_path: String,
    attachment_path: String,
    hit_json: String,
}

struct AttachmentCandidateQuery<'a> {
    case_sensitive: bool,
    normalized_query: &'a str,
    query_tokens: &'a [String],
    extensions: &'a HashSet<String>,
    kinds: &'a HashSet<String>,
    window: RetainedWindow,
}

struct AttachmentSearchExecution {
    candidates: Vec<AttachmentCandidate>,
    telemetry: StreamingRerankTelemetry,
    source: StreamingRerankSource,
}

async fn execute_attachment_search(
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

fn normalize_extension_filters(extensions: &[String]) -> HashSet<String> {
    extensions
        .iter()
        .map(|value| value.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect()
}

fn normalize_kind_filters(kinds: &[crate::link_graph::LinkGraphAttachmentKind]) -> HashSet<String> {
    kinds
        .iter()
        .copied()
        .map(attachment_kind_label)
        .map(ToString::to_string)
        .collect()
}

fn build_attachment_scan_options(
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

fn build_query_tokens(normalized_query: &str) -> Vec<String> {
    normalized_query
        .split_whitespace()
        .filter(|token| !token.is_empty())
        .map(str::to_string)
        .collect()
}

fn decode_attachment_hits(
    candidates: Vec<AttachmentCandidate>,
) -> Result<Vec<AttachmentSearchHit>, AttachmentSearchError> {
    candidates
        .into_iter()
        .map(|candidate| {
            let mut hit: AttachmentSearchHit = serde_json::from_str(candidate.hit_json.as_str())
                .map_err(|error| AttachmentSearchError::Decode(error.to_string()))?;
            hit.score = candidate.score;
            Ok(hit)
        })
        .collect()
}

fn retained_window(limit: usize) -> RetainedWindow {
    RetainedWindow::new(
        limit,
        RETAINED_ATTACHMENT_MULTIPLIER,
        MIN_RETAINED_ATTACHMENTS,
    )
}

fn compare_candidates(
    left: &AttachmentCandidate,
    right: &AttachmentCandidate,
) -> std::cmp::Ordering {
    right
        .score
        .partial_cmp(&left.score)
        .unwrap_or(std::cmp::Ordering::Equal)
        .then_with(|| left.attachment_path.cmp(&right.attachment_path))
        .then_with(|| left.source_path.cmp(&right.source_path))
}

fn candidate_score(normalized_query: &str, query_tokens: &[String], fields: &[&str; 5]) -> f64 {
    if normalized_query.is_empty() {
        return 1.0;
    }

    let query_hit = fields.iter().any(|value| value.contains(normalized_query));
    let token_hit_count = query_tokens
        .iter()
        .filter(|token| fields.iter().any(|value| value.contains(token.as_str())))
        .count();
    if !query_hit && token_hit_count == 0 {
        return 0.0;
    }

    let exact_name = if fields[1] == normalized_query {
        1.0
    } else {
        0.0
    };
    let path_hit = if fields[0].contains(normalized_query) {
        1.0
    } else {
        0.0
    };
    let token_ratio = if query_tokens.is_empty() {
        0.0
    } else {
        usize_to_f64_saturating(token_hit_count) / usize_to_f64_saturating(query_tokens.len())
    };
    (exact_name * 0.5 + path_hit * 0.3 + token_ratio * 0.2).clamp(0.0, 1.0)
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

fn should_use_fts(query: &str) -> bool {
    query
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch.is_ascii_whitespace() || ch == '_' || ch == '-')
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

fn usize_to_f64_saturating(value: usize) -> f64 {
    u32::try_from(value).map_or(f64::from(u32::MAX), f64::from)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::gateway::studio::types::{AttachmentSearchHit, StudioNavigationTarget};
    use crate::search_plane::{
        BeginBuildDecision, SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace,
        SearchPlaneService,
    };

    use super::*;
    use crate::search_plane::ranking::trim_ranked_vec;

    #[test]
    fn trim_candidates_keeps_highest_ranked_attachment_hits() {
        let mut candidates = vec![
            AttachmentCandidate {
                score: 0.4,
                source_path: "docs/zeta.md".to_string(),
                attachment_path: "assets/zeta.png".to_string(),
                hit_json: "{}".to_string(),
            },
            AttachmentCandidate {
                score: 0.9,
                source_path: "docs/beta.md".to_string(),
                attachment_path: "assets/beta.png".to_string(),
                hit_json: "{}".to_string(),
            },
            AttachmentCandidate {
                score: 0.9,
                source_path: "docs/alpha.md".to_string(),
                attachment_path: "assets/alpha.png".to_string(),
                hit_json: "{}".to_string(),
            },
        ];

        trim_ranked_vec(&mut candidates, 2, compare_candidates);

        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].attachment_path, "assets/alpha.png");
        assert_eq!(candidates[1].attachment_path, "assets/beta.png");
    }

    #[test]
    fn retained_window_scales_with_limit() {
        assert_eq!(retained_window(0).target, 32);
        assert_eq!(retained_window(8).target, 32);
        assert_eq!(retained_window(32).target, 64);
    }

    fn fixture_service(temp_dir: &tempfile::TempDir) -> SearchPlaneService {
        SearchPlaneService::with_paths(
            PathBuf::from("/tmp/project"),
            temp_dir.path().join("search_plane"),
            SearchManifestKeyspace::new("xiuxian:test:attachment"),
            SearchMaintenancePolicy::default(),
        )
    }

    fn sample_hit(
        name: &str,
        source_path: &str,
        attachment_path: &str,
        kind: &str,
    ) -> AttachmentSearchHit {
        AttachmentSearchHit {
            name: name.to_string(),
            path: source_path.to_string(),
            source_id: source_path.trim_end_matches(".md").to_string(),
            source_stem: "alpha".to_string(),
            source_title: "Alpha".to_string(),
            source_path: source_path.to_string(),
            attachment_id: format!("att://{source_path}/{attachment_path}"),
            attachment_path: attachment_path.to_string(),
            attachment_name: name.to_string(),
            attachment_ext: attachment_path
                .split('.')
                .next_back()
                .unwrap_or_default()
                .to_string(),
            kind: kind.to_string(),
            navigation_target: StudioNavigationTarget {
                path: source_path.to_string(),
                category: "doc".to_string(),
                project_name: None,
                root_label: None,
                line: None,
                line_end: None,
                column: None,
            },
            score: 0.0,
            vision_snippet: None,
        }
    }

    #[tokio::test]
    async fn attachment_query_reads_hits_from_published_epoch() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let service = fixture_service(&temp_dir);
        let lease = match service.coordinator().begin_build(
            SearchCorpusKind::Attachment,
            "fp-1",
            SearchCorpusKind::Attachment.schema_version(),
        ) {
            BeginBuildDecision::Started(lease) => lease,
            other => panic!("unexpected begin decision: {other:?}"),
        };
        let hits = vec![
            sample_hit(
                "topology.png",
                "docs/alpha.md",
                "assets/topology.png",
                "image",
            ),
            sample_hit("spec.pdf", "docs/alpha.md", "files/spec.pdf", "pdf"),
        ];
        let store = service
            .open_store(SearchCorpusKind::Attachment)
            .await
            .unwrap_or_else(|error| panic!("open store: {error}"));
        let table_name = SearchPlaneService::table_name(SearchCorpusKind::Attachment, lease.epoch);
        store
            .replace_record_batches(
                table_name.as_str(),
                attachment_schema(),
                attachment_batches(&hits).unwrap_or_else(|error| panic!("batches: {error}")),
            )
            .await
            .unwrap_or_else(|error| panic!("replace record batches: {error}"));
        store
            .create_inverted_index(table_name.as_str(), search_text_column(), None)
            .await
            .unwrap_or_else(|error| panic!("create inverted index: {error}"));
        service
            .coordinator()
            .publish_ready(&lease, hits.len() as u64, 1);

        let results = search_attachment_hits(&service, "topology", 5, &[], &[], false)
            .await
            .unwrap_or_else(|error| panic!("query should succeed: {error}"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].attachment_name, "topology.png");
        assert!(results[0].score > 0.0);
    }
}
