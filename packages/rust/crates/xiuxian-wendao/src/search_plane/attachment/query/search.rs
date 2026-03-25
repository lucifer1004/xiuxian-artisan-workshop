use crate::gateway::studio::types::AttachmentSearchHit;
use crate::search_plane::attachment::query::scan::{
    build_attachment_scan_options, execute_attachment_search,
};
use crate::search_plane::attachment::query::scoring::{
    build_query_tokens, compare_candidates, normalize_extension_filters, normalize_kind_filters,
    retained_window, should_use_fts,
};
use crate::search_plane::attachment::query::types::{
    AttachmentCandidate, AttachmentCandidateQuery, AttachmentSearchError,
};
use crate::search_plane::ranking::sort_by_rank;
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

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
