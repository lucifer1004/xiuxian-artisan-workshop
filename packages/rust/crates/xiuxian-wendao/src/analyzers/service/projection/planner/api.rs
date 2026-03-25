use std::collections::BTreeMap;
use std::path::Path;

use crate::analyzers::errors::RepoIntelligenceError;
use crate::analyzers::plugin::RepositoryAnalysisOutput;
use crate::analyzers::query::{
    DocsNavigationQuery, DocsPlannerItemQuery, DocsPlannerItemResult, DocsPlannerQueueGroup,
    DocsPlannerQueueQuery, DocsPlannerQueueResult, DocsPlannerRankHit, DocsPlannerRankQuery,
    DocsPlannerRankResult, DocsPlannerSearchHit, DocsPlannerSearchQuery, DocsPlannerSearchResult,
    DocsProjectedGapReportQuery,
};
use crate::analyzers::registry::PluginRegistry;

use super::scoring::{
    normalize_planner_search_text, planner_gap_priority_breakdown, planner_gap_search_score,
};
use crate::analyzers::service::projection::gap::build_docs_projected_gap_report;
use crate::analyzers::service::projection::navigation::build_docs_navigation;
use crate::analyzers::service::projection::registry::{
    with_bootstrapped_repository_analysis, with_repository_analysis,
};
use crate::analyzers::service::projection::retrieval::build_docs_retrieval_hit;

/// Build one deterministic docs-facing deep-wiki planner item from a stable projected gap.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError::UnknownProjectedGap`] when the requested projected gap is not
/// present in the analysis output, or propagates the deterministic navigation and retrieval-hit
/// lookup errors for the owning projected page.
pub fn build_docs_planner_item(
    query: &DocsPlannerItemQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<DocsPlannerItemResult, RepoIntelligenceError> {
    let gap_report = build_docs_projected_gap_report(
        &DocsProjectedGapReportQuery {
            repo_id: query.repo_id.clone(),
        },
        analysis,
    );
    let gap = gap_report
        .gaps
        .into_iter()
        .find(|gap| gap.gap_id == query.gap_id)
        .ok_or_else(|| RepoIntelligenceError::UnknownProjectedGap {
            repo_id: query.repo_id.clone(),
            gap_id: query.gap_id.clone(),
        })?;
    let hit = build_docs_retrieval_hit(
        &crate::analyzers::query::DocsRetrievalHitQuery {
            repo: query.repo_id.clone(),
            page: gap.page_id.clone(),
            node: None,
        },
        analysis,
    )?
    .hit;
    let navigation = build_docs_navigation(
        &DocsNavigationQuery {
            repo_id: query.repo_id.clone(),
            page_id: gap.page_id.clone(),
            node_id: None,
            family_kind: query.family_kind,
            related_limit: query.related_limit,
            family_limit: query.family_limit,
        },
        analysis,
    )?;

    Ok(DocsPlannerItemResult {
        repo_id: query.repo_id.clone(),
        gap,
        hit,
        navigation,
    })
}

/// Load configuration, analyze one repository, and return one deterministic docs-facing deep-wiki
/// planner item.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected gap
/// or owning projected page identifiers are not present for the repository.
pub fn docs_planner_item_from_config_with_registry(
    query: &DocsPlannerItemQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<DocsPlannerItemResult, RepoIntelligenceError> {
    with_repository_analysis(&query.repo_id, config_path, cwd, registry, |analysis| {
        build_docs_planner_item(query, analysis)
    })
}

/// Load configuration, analyze one repository, and return one deterministic docs-facing deep-wiki
/// planner item.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or the requested projected gap
/// or owning projected page identifiers are not present for the repository.
pub fn docs_planner_item_from_config(
    query: &DocsPlannerItemQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<DocsPlannerItemResult, RepoIntelligenceError> {
    with_bootstrapped_repository_analysis(&query.repo_id, config_path, cwd, |analysis| {
        build_docs_planner_item(query, analysis)
    })
}

/// Build deterministic docs-facing deep-wiki planner search hits from projected gaps.
#[must_use]
pub fn build_docs_planner_search(
    query: &DocsPlannerSearchQuery,
    analysis: &RepositoryAnalysisOutput,
) -> DocsPlannerSearchResult {
    let normalized_query = normalize_planner_search_text(query.query.as_str());
    let mut hits = build_docs_projected_gap_report(
        &DocsProjectedGapReportQuery {
            repo_id: query.repo_id.clone(),
        },
        analysis,
    )
    .gaps
    .into_iter()
    .filter(|gap| {
        query
            .gap_kind
            .is_none_or(|expected_kind| gap.kind == expected_kind)
            && query
                .page_kind
                .is_none_or(|expected_page_kind| gap.page_kind == expected_page_kind)
    })
    .filter_map(|gap| {
        let score = planner_gap_search_score(&gap, normalized_query.as_str());
        (score > 0).then_some(DocsPlannerSearchHit {
            search_score: score,
            gap,
        })
    })
    .collect::<Vec<_>>();

    hits.sort_by(|left, right| {
        right
            .search_score
            .cmp(&left.search_score)
            .then_with(|| left.gap.kind.cmp(&right.gap.kind))
            .then_with(|| left.gap.title.cmp(&right.gap.title))
            .then_with(|| left.gap.gap_id.cmp(&right.gap.gap_id))
    });
    hits.truncate(query.limit);

    DocsPlannerSearchResult {
        repo_id: query.repo_id.clone(),
        hits,
    }
}

/// Load configuration, analyze one repository, and return deterministic docs-facing deep-wiki
/// planner search hits.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn docs_planner_search_from_config_with_registry(
    query: &DocsPlannerSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<DocsPlannerSearchResult, RepoIntelligenceError> {
    with_repository_analysis(&query.repo_id, config_path, cwd, registry, |analysis| {
        Ok(build_docs_planner_search(query, analysis))
    })
}

/// Load configuration, analyze one repository, and return deterministic docs-facing deep-wiki
/// planner search hits.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn docs_planner_search_from_config(
    query: &DocsPlannerSearchQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<DocsPlannerSearchResult, RepoIntelligenceError> {
    with_bootstrapped_repository_analysis(&query.repo_id, config_path, cwd, |analysis| {
        Ok(build_docs_planner_search(query, analysis))
    })
}

/// Build deterministic docs-facing deep-wiki planner queue groups from projected gaps.
#[must_use]
pub fn build_docs_planner_queue(
    query: &DocsPlannerQueueQuery,
    analysis: &RepositoryAnalysisOutput,
) -> DocsPlannerQueueResult {
    let gap_report = build_docs_projected_gap_report(
        &DocsProjectedGapReportQuery {
            repo_id: query.repo_id.clone(),
        },
        analysis,
    );
    let mut grouped =
        BTreeMap::<crate::analyzers::query::ProjectedGapKind, DocsPlannerQueueGroup>::new();

    for gap in gap_report.gaps.into_iter().filter(|gap| {
        query
            .gap_kind
            .is_none_or(|expected_kind| gap.kind == expected_kind)
            && query
                .page_kind
                .is_none_or(|expected_page_kind| gap.page_kind == expected_page_kind)
    }) {
        let entry = grouped
            .entry(gap.kind)
            .or_insert_with(|| DocsPlannerQueueGroup {
                kind: gap.kind,
                count: 0,
                gaps: Vec::new(),
            });
        entry.count += 1;
        if entry.gaps.len() < query.per_kind_limit {
            entry.gaps.push(gap);
        }
    }

    let mut groups = grouped.into_values().collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.kind.cmp(&right.kind))
    });

    DocsPlannerQueueResult {
        repo_id: query.repo_id.clone(),
        page_count: gap_report.summary.page_count,
        total_gap_count: groups.iter().map(|group| group.count).sum(),
        groups,
    }
}

/// Load configuration, analyze one repository, and return deterministic docs-facing deep-wiki
/// planner queue groups.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn docs_planner_queue_from_config_with_registry(
    query: &DocsPlannerQueueQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<DocsPlannerQueueResult, RepoIntelligenceError> {
    with_repository_analysis(&query.repo_id, config_path, cwd, registry, |analysis| {
        Ok(build_docs_planner_queue(query, analysis))
    })
}

/// Load configuration, analyze one repository, and return deterministic docs-facing deep-wiki
/// planner queue groups.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn docs_planner_queue_from_config(
    query: &DocsPlannerQueueQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<DocsPlannerQueueResult, RepoIntelligenceError> {
    with_bootstrapped_repository_analysis(&query.repo_id, config_path, cwd, |analysis| {
        Ok(build_docs_planner_queue(query, analysis))
    })
}

/// Build deterministic docs-facing deep-wiki planner ranking hits from projected gaps.
#[must_use]
pub fn build_docs_planner_rank(
    query: &DocsPlannerRankQuery,
    analysis: &RepositoryAnalysisOutput,
) -> DocsPlannerRankResult {
    let mut hits = build_docs_projected_gap_report(
        &DocsProjectedGapReportQuery {
            repo_id: query.repo_id.clone(),
        },
        analysis,
    )
    .gaps
    .into_iter()
    .filter(|gap| {
        query
            .gap_kind
            .is_none_or(|expected_kind| gap.kind == expected_kind)
            && query
                .page_kind
                .is_none_or(|expected_page_kind| gap.page_kind == expected_page_kind)
    })
    .map(|gap| {
        let (priority_score, reasons) = planner_gap_priority_breakdown(&gap);
        DocsPlannerRankHit {
            priority_score,
            reasons,
            gap,
        }
    })
    .collect::<Vec<_>>();

    hits.sort_by(|left, right| {
        right
            .priority_score
            .cmp(&left.priority_score)
            .then_with(|| left.gap.kind.cmp(&right.gap.kind))
            .then_with(|| left.gap.title.cmp(&right.gap.title))
            .then_with(|| left.gap.gap_id.cmp(&right.gap.gap_id))
    });
    hits.truncate(query.limit);

    DocsPlannerRankResult {
        repo_id: query.repo_id.clone(),
        hits,
    }
}

/// Load configuration, analyze one repository, and return deterministic docs-facing deep-wiki
/// planner ranking hits.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn docs_planner_rank_from_config_with_registry(
    query: &DocsPlannerRankQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<DocsPlannerRankResult, RepoIntelligenceError> {
    with_repository_analysis(&query.repo_id, config_path, cwd, registry, |analysis| {
        Ok(build_docs_planner_rank(query, analysis))
    })
}

/// Load configuration, analyze one repository, and return deterministic docs-facing deep-wiki
/// planner ranking hits.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails.
pub fn docs_planner_rank_from_config(
    query: &DocsPlannerRankQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<DocsPlannerRankResult, RepoIntelligenceError> {
    with_bootstrapped_repository_analysis(&query.repo_id, config_path, cwd, |analysis| {
        Ok(build_docs_planner_rank(query, analysis))
    })
}
