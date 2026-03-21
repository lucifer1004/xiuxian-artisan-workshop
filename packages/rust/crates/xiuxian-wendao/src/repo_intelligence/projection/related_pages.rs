use std::collections::BTreeSet;

use crate::repo_intelligence::plugin::RepositoryAnalysisOutput;

use super::contracts::{ProjectedPageRecord, ProjectionPageKind};
use super::pages::build_projected_pages;

pub(crate) fn scored_related_projected_pages(
    center: &ProjectedPageRecord,
    analysis: &RepositoryAnalysisOutput,
) -> Vec<(usize, ProjectedPageRecord)> {
    let center_module_ids = center.module_ids.iter().cloned().collect::<BTreeSet<_>>();
    let center_symbol_ids = center.symbol_ids.iter().cloned().collect::<BTreeSet<_>>();
    let center_example_ids = center.example_ids.iter().cloned().collect::<BTreeSet<_>>();
    let center_doc_ids = center.doc_ids.iter().cloned().collect::<BTreeSet<_>>();

    let mut matches = build_projected_pages(analysis)
        .into_iter()
        .filter(|page| page.page_id != center.page_id)
        .filter_map(|page| {
            let score = shared_anchor_score(
                &center_module_ids,
                &center_symbol_ids,
                &center_example_ids,
                &center_doc_ids,
                &page,
            );
            (score > 0).then_some((score, page))
        })
        .collect::<Vec<_>>();

    matches.sort_by(|(left_score, left_page), (right_score, right_page)| {
        right_score
            .cmp(left_score)
            .then_with(|| {
                projection_kind_rank(left_page.kind).cmp(&projection_kind_rank(right_page.kind))
            })
            .then_with(|| left_page.title.cmp(&right_page.title))
            .then_with(|| left_page.page_id.cmp(&right_page.page_id))
    });

    matches
}

pub(crate) fn projection_kind_rank(kind: ProjectionPageKind) -> u8 {
    match kind {
        ProjectionPageKind::Reference => 0,
        ProjectionPageKind::HowTo => 1,
        ProjectionPageKind::Tutorial => 2,
        ProjectionPageKind::Explanation => 3,
    }
}

pub(crate) const PROJECTION_PAGE_KIND_ORDER: [ProjectionPageKind; 4] = [
    ProjectionPageKind::Reference,
    ProjectionPageKind::HowTo,
    ProjectionPageKind::Tutorial,
    ProjectionPageKind::Explanation,
];

fn shared_anchor_score(
    center_module_ids: &BTreeSet<String>,
    center_symbol_ids: &BTreeSet<String>,
    center_example_ids: &BTreeSet<String>,
    center_doc_ids: &BTreeSet<String>,
    page: &ProjectedPageRecord,
) -> usize {
    intersection_len(center_module_ids, &page.module_ids)
        + intersection_len(center_symbol_ids, &page.symbol_ids)
        + intersection_len(center_example_ids, &page.example_ids)
        + intersection_len(center_doc_ids, &page.doc_ids)
}

fn intersection_len(center: &BTreeSet<String>, candidates: &[String]) -> usize {
    candidates
        .iter()
        .filter(|candidate| center.contains(candidate.as_str()))
        .count()
}
