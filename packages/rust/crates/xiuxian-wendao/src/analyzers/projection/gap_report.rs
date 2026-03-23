use std::collections::BTreeMap;

use crate::analyzers::plugin::RepositoryAnalysisOutput;
use crate::analyzers::projection::contracts::{ProjectedPageRecord, ProjectionPageKind};
use crate::analyzers::query::{
    ProjectedGapKind, ProjectedGapRecord, ProjectedGapSummary, ProjectedGapSummaryEntry,
    RepoProjectedGapReportQuery, RepoProjectedGapReportResult,
};

use super::pages::build_projected_pages;

/// Build a deterministic deep-wiki projected gap report from repository truth.
#[must_use]
pub fn build_projected_gap_report(
    query: &RepoProjectedGapReportQuery,
    analysis: &RepositoryAnalysisOutput,
) -> RepoProjectedGapReportResult {
    let pages = build_projected_pages(analysis);
    let module_pages = module_page_lookup(pages.as_slice());
    let symbol_pages = symbol_page_lookup(pages.as_slice());
    let example_pages = example_page_lookup(pages.as_slice());
    let doc_pages = doc_page_lookup(pages.as_slice());
    let mut gaps = Vec::new();

    for module in &analysis.modules {
        let Some(page) = module_pages.get(module.module_id.as_str()) else {
            continue;
        };
        if page.doc_ids.is_empty() {
            gaps.push(ProjectedGapRecord {
                repo_id: query.repo_id.clone(),
                gap_id: gap_id(
                    query.repo_id.as_str(),
                    ProjectedGapKind::ModuleReferenceWithoutDocumentation,
                    module.module_id.as_str(),
                ),
                kind: ProjectedGapKind::ModuleReferenceWithoutDocumentation,
                page_kind: ProjectionPageKind::Reference,
                page_id: page.page_id.clone(),
                entity_id: module.module_id.clone(),
                title: page.title.clone(),
                path: page.path.clone(),
                module_ids: page.module_ids.clone(),
                symbol_ids: page.symbol_ids.clone(),
                example_ids: page.example_ids.clone(),
                doc_ids: page.doc_ids.clone(),
                format_hints: page.format_hints.clone(),
            });
        }
    }

    for symbol in &analysis.symbols {
        let Some(page) = symbol_pages.get(symbol.symbol_id.as_str()) else {
            continue;
        };
        if page.doc_ids.is_empty() {
            gaps.push(ProjectedGapRecord {
                repo_id: query.repo_id.clone(),
                gap_id: gap_id(
                    query.repo_id.as_str(),
                    ProjectedGapKind::SymbolReferenceWithoutDocumentation,
                    symbol.symbol_id.as_str(),
                ),
                kind: ProjectedGapKind::SymbolReferenceWithoutDocumentation,
                page_kind: ProjectionPageKind::Reference,
                page_id: page.page_id.clone(),
                entity_id: symbol.symbol_id.clone(),
                title: page.title.clone(),
                path: page.path.clone(),
                module_ids: page.module_ids.clone(),
                symbol_ids: page.symbol_ids.clone(),
                example_ids: page.example_ids.clone(),
                doc_ids: page.doc_ids.clone(),
                format_hints: page.format_hints.clone(),
            });
            continue;
        }

        if symbol.verification_state.as_deref() == Some("unverified") {
            gaps.push(ProjectedGapRecord {
                repo_id: query.repo_id.clone(),
                gap_id: gap_id(
                    query.repo_id.as_str(),
                    ProjectedGapKind::SymbolReferenceUnverified,
                    symbol.symbol_id.as_str(),
                ),
                kind: ProjectedGapKind::SymbolReferenceUnverified,
                page_kind: ProjectionPageKind::Reference,
                page_id: page.page_id.clone(),
                entity_id: symbol.symbol_id.clone(),
                title: page.title.clone(),
                path: page.path.clone(),
                module_ids: page.module_ids.clone(),
                symbol_ids: page.symbol_ids.clone(),
                example_ids: page.example_ids.clone(),
                doc_ids: page.doc_ids.clone(),
                format_hints: page.format_hints.clone(),
            });
        }
    }

    for example in &analysis.examples {
        let Some(page) = example_pages.get(example.example_id.as_str()) else {
            continue;
        };
        if page.module_ids.is_empty() && page.symbol_ids.is_empty() {
            gaps.push(ProjectedGapRecord {
                repo_id: query.repo_id.clone(),
                gap_id: gap_id(
                    query.repo_id.as_str(),
                    ProjectedGapKind::ExampleHowToWithoutAnchor,
                    example.example_id.as_str(),
                ),
                kind: ProjectedGapKind::ExampleHowToWithoutAnchor,
                page_kind: ProjectionPageKind::HowTo,
                page_id: page.page_id.clone(),
                entity_id: example.example_id.clone(),
                title: page.title.clone(),
                path: page.path.clone(),
                module_ids: page.module_ids.clone(),
                symbol_ids: page.symbol_ids.clone(),
                example_ids: page.example_ids.clone(),
                doc_ids: page.doc_ids.clone(),
                format_hints: page.format_hints.clone(),
            });
        }
    }

    for doc in &analysis.docs {
        let Some(page) = doc_pages.get(doc.doc_id.as_str()) else {
            continue;
        };
        if page.module_ids.is_empty() && page.symbol_ids.is_empty() {
            gaps.push(ProjectedGapRecord {
                repo_id: query.repo_id.clone(),
                gap_id: gap_id(
                    query.repo_id.as_str(),
                    ProjectedGapKind::DocumentationPageWithoutAnchor,
                    doc.doc_id.as_str(),
                ),
                kind: ProjectedGapKind::DocumentationPageWithoutAnchor,
                page_kind: page.kind,
                page_id: page.page_id.clone(),
                entity_id: doc.doc_id.clone(),
                title: page.title.clone(),
                path: page.path.clone(),
                module_ids: page.module_ids.clone(),
                symbol_ids: page.symbol_ids.clone(),
                example_ids: page.example_ids.clone(),
                doc_ids: page.doc_ids.clone(),
                format_hints: page.format_hints.clone(),
            });
        }
    }

    gaps.sort_by(|left, right| {
        left.kind
            .cmp(&right.kind)
            .then_with(|| left.title.cmp(&right.title))
            .then_with(|| left.page_id.cmp(&right.page_id))
            .then_with(|| left.gap_id.cmp(&right.gap_id))
    });

    RepoProjectedGapReportResult {
        repo_id: query.repo_id.clone(),
        summary: build_gap_summary(gaps.as_slice(), pages.len()),
        gaps,
    }
}

fn build_gap_summary(gaps: &[ProjectedGapRecord], page_count: usize) -> ProjectedGapSummary {
    let mut counts = BTreeMap::<ProjectedGapKind, usize>::new();
    for gap in gaps {
        *counts.entry(gap.kind).or_default() += 1;
    }

    ProjectedGapSummary {
        page_count,
        gap_count: gaps.len(),
        by_kind: counts
            .into_iter()
            .map(|(kind, count)| ProjectedGapSummaryEntry { kind, count })
            .collect(),
    }
}

fn module_page_lookup(pages: &[ProjectedPageRecord]) -> BTreeMap<String, ProjectedPageRecord> {
    pages
        .iter()
        .filter(|page| page.kind == ProjectionPageKind::Reference)
        .filter(|page| page.page_id.contains(":module:"))
        .filter_map(|page| {
            page.module_ids
                .first()
                .cloned()
                .map(|module_id| (module_id, page.clone()))
        })
        .collect()
}

fn symbol_page_lookup(pages: &[ProjectedPageRecord]) -> BTreeMap<String, ProjectedPageRecord> {
    pages
        .iter()
        .filter(|page| page.kind == ProjectionPageKind::Reference)
        .filter(|page| page.page_id.contains(":symbol:"))
        .filter_map(|page| {
            page.symbol_ids
                .first()
                .cloned()
                .map(|symbol_id| (symbol_id, page.clone()))
        })
        .collect()
}

fn example_page_lookup(pages: &[ProjectedPageRecord]) -> BTreeMap<String, ProjectedPageRecord> {
    pages
        .iter()
        .filter(|page| page.kind == ProjectionPageKind::HowTo)
        .filter(|page| page.page_id.contains(":example:"))
        .filter_map(|page| {
            page.example_ids
                .first()
                .cloned()
                .map(|example_id| (example_id, page.clone()))
        })
        .collect()
}

fn doc_page_lookup(pages: &[ProjectedPageRecord]) -> BTreeMap<String, ProjectedPageRecord> {
    pages
        .iter()
        .filter(|page| page.page_id.contains(":doc:"))
        .filter_map(|page| {
            page.doc_ids
                .first()
                .cloned()
                .map(|doc_id| (doc_id, page.clone()))
        })
        .collect()
}

fn gap_id(repo_id: &str, kind: ProjectedGapKind, entity_id: &str) -> String {
    format!(
        "repo:{repo_id}:projection-gap:{}:{entity_id}",
        gap_kind_token(kind)
    )
}

fn gap_kind_token(kind: ProjectedGapKind) -> &'static str {
    match kind {
        ProjectedGapKind::ModuleReferenceWithoutDocumentation => {
            "module_reference_without_documentation"
        }
        ProjectedGapKind::SymbolReferenceWithoutDocumentation => {
            "symbol_reference_without_documentation"
        }
        ProjectedGapKind::SymbolReferenceUnverified => "symbol_reference_unverified",
        ProjectedGapKind::ExampleHowToWithoutAnchor => "example_howto_without_anchor",
        ProjectedGapKind::DocumentationPageWithoutAnchor => "documentation_page_without_anchor",
    }
}
