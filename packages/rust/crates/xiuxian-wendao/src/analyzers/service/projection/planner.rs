use std::collections::BTreeMap;
use std::path::Path;

use crate::analyzers::errors::RepoIntelligenceError;
use crate::analyzers::plugin::RepositoryAnalysisOutput;
use crate::analyzers::query::{
    DocsNavigationQuery, DocsPlannerItemQuery, DocsPlannerItemResult, DocsPlannerQueueGroup,
    DocsPlannerQueueQuery, DocsPlannerQueueResult, DocsPlannerRankHit, DocsPlannerRankQuery,
    DocsPlannerRankReason, DocsPlannerRankReasonCode, DocsPlannerRankResult, DocsPlannerSearchHit,
    DocsPlannerSearchQuery, DocsPlannerSearchResult, DocsPlannerWorksetBalance,
    DocsPlannerWorksetFamilyBalanceEntry, DocsPlannerWorksetFamilyGroup,
    DocsPlannerWorksetGapKindBalanceEntry, DocsPlannerWorksetGroup, DocsPlannerWorksetQuery,
    DocsPlannerWorksetQuotaHint, DocsPlannerWorksetResult, DocsPlannerWorksetStrategy,
    DocsPlannerWorksetStrategyCode, DocsPlannerWorksetStrategyReason,
    DocsPlannerWorksetStrategyReasonCode, DocsProjectedGapReportQuery, DocsRetrievalHitQuery,
};
use crate::analyzers::registry::PluginRegistry;

use super::gap::build_docs_projected_gap_report;
use super::navigation::build_docs_navigation;
use super::registry::{with_bootstrapped_repository_analysis, with_repository_analysis};
use super::retrieval::build_docs_retrieval_hit;

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
        &DocsRetrievalHitQuery {
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

/// Build deterministic docs-facing deep-wiki planner workset bundles from a queue snapshot and a
/// ranked planner selection.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when one selected planner item cannot be reopened from the
/// underlying ranked projected gaps or page kernels.
pub fn build_docs_planner_workset(
    query: &DocsPlannerWorksetQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Result<DocsPlannerWorksetResult, RepoIntelligenceError> {
    let queue = planner_queue_snapshot(query, analysis);
    let ranked_hits = planner_ranked_hits(query, analysis);
    let items = open_planner_items(query, analysis, &ranked_hits)?;
    let groups = build_planner_workset_groups(&ranked_hits, &items);

    let balance = build_docs_planner_workset_balance(&groups);
    let strategy = build_docs_planner_workset_strategy(&balance);

    Ok(DocsPlannerWorksetResult {
        repo_id: query.repo_id.clone(),
        queue,
        ranked_hits,
        balance,
        strategy,
        groups,
        items,
    })
}

fn planner_queue_snapshot(
    query: &DocsPlannerWorksetQuery,
    analysis: &RepositoryAnalysisOutput,
) -> DocsPlannerQueueResult {
    build_docs_planner_queue(
        &DocsPlannerQueueQuery {
            repo_id: query.repo_id.clone(),
            gap_kind: query.gap_kind,
            page_kind: query.page_kind,
            per_kind_limit: query.per_kind_limit,
        },
        analysis,
    )
}

fn planner_ranked_hits(
    query: &DocsPlannerWorksetQuery,
    analysis: &RepositoryAnalysisOutput,
) -> Vec<DocsPlannerRankHit> {
    build_docs_planner_rank(
        &DocsPlannerRankQuery {
            repo_id: query.repo_id.clone(),
            gap_kind: query.gap_kind,
            page_kind: query.page_kind,
            limit: query.limit,
        },
        analysis,
    )
    .hits
}

fn open_planner_items(
    query: &DocsPlannerWorksetQuery,
    analysis: &RepositoryAnalysisOutput,
    ranked_hits: &[DocsPlannerRankHit],
) -> Result<Vec<DocsPlannerItemResult>, RepoIntelligenceError> {
    ranked_hits
        .iter()
        .map(|ranked_hit| {
            build_docs_planner_item(
                &DocsPlannerItemQuery {
                    repo_id: query.repo_id.clone(),
                    gap_id: ranked_hit.gap.gap_id.clone(),
                    family_kind: query.family_kind,
                    related_limit: query.related_limit,
                    family_limit: query.family_limit,
                },
                analysis,
            )
        })
        .collect()
}

fn build_planner_workset_groups(
    ranked_hits: &[DocsPlannerRankHit],
    items: &[DocsPlannerItemResult],
) -> Vec<DocsPlannerWorksetGroup> {
    let mut groups = initial_planner_workset_groups(ranked_hits, items);
    populate_family_groups(&mut groups);
    apply_gap_kind_quota_hints(&mut groups);
    groups
}

fn initial_planner_workset_groups(
    ranked_hits: &[DocsPlannerRankHit],
    items: &[DocsPlannerItemResult],
) -> Vec<DocsPlannerWorksetGroup> {
    let mut grouped =
        BTreeMap::<crate::analyzers::query::ProjectedGapKind, DocsPlannerWorksetGroup>::new();

    for (ranked_hit, item) in ranked_hits.iter().cloned().zip(items.iter().cloned()) {
        let entry = grouped
            .entry(ranked_hit.gap.kind)
            .or_insert_with(|| DocsPlannerWorksetGroup {
                kind: ranked_hit.gap.kind,
                selected_count: 0,
                quota: empty_quota_hint(),
                families: Vec::new(),
                ranked_hits: Vec::new(),
                items: Vec::new(),
            });
        entry.selected_count += 1;
        entry.ranked_hits.push(ranked_hit);
        entry.items.push(item);
    }

    let mut groups = grouped.into_values().collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        right
            .selected_count
            .cmp(&left.selected_count)
            .then_with(|| left.kind.cmp(&right.kind))
    });
    groups
}

fn populate_family_groups(groups: &mut [DocsPlannerWorksetGroup]) {
    for group in groups {
        group.families = family_groups_for_workset_group(group);
    }
}

fn family_groups_for_workset_group(
    group: &DocsPlannerWorksetGroup,
) -> Vec<DocsPlannerWorksetFamilyGroup> {
    let mut families = BTreeMap::<
        crate::analyzers::projection::ProjectionPageKind,
        DocsPlannerWorksetFamilyGroup,
    >::new();

    for (ranked_hit, item) in group
        .ranked_hits
        .iter()
        .cloned()
        .zip(group.items.iter().cloned())
    {
        let entry = families.entry(ranked_hit.gap.page_kind).or_insert_with(|| {
            DocsPlannerWorksetFamilyGroup {
                kind: ranked_hit.gap.page_kind,
                selected_count: 0,
                quota: empty_quota_hint(),
                ranked_hits: Vec::new(),
                items: Vec::new(),
            }
        });
        entry.selected_count += 1;
        entry.ranked_hits.push(ranked_hit);
        entry.items.push(item);
    }

    let mut family_groups = families.into_values().collect::<Vec<_>>();
    apply_family_quota_hints(group.selected_count, &mut family_groups);
    family_groups.sort_by(|left, right| {
        right
            .selected_count
            .cmp(&left.selected_count)
            .then_with(|| left.kind.cmp(&right.kind))
    });
    family_groups
}

fn apply_family_quota_hints(
    selection_count: usize,
    families: &mut [DocsPlannerWorksetFamilyGroup],
) {
    let (target_floor_count, target_ceiling_count) = quota_band(selection_count, families.len());
    for family in families {
        family.quota = quota_hint_for_selection(
            family.selected_count,
            target_floor_count,
            target_ceiling_count,
        );
    }
}

fn apply_gap_kind_quota_hints(groups: &mut [DocsPlannerWorksetGroup]) {
    let selection_count = groups
        .iter()
        .map(|group| group.selected_count)
        .sum::<usize>();
    let (target_floor_count, target_ceiling_count) = quota_band(selection_count, groups.len());
    for group in groups {
        group.quota = quota_hint_for_selection(
            group.selected_count,
            target_floor_count,
            target_ceiling_count,
        );
    }
}

fn quota_hint_for_selection(
    selected_count: usize,
    target_floor_count: usize,
    target_ceiling_count: usize,
) -> DocsPlannerWorksetQuotaHint {
    DocsPlannerWorksetQuotaHint {
        target_floor_count,
        target_ceiling_count,
        within_target_band: selected_count >= target_floor_count
            && selected_count <= target_ceiling_count,
    }
}

fn empty_quota_hint() -> DocsPlannerWorksetQuotaHint {
    DocsPlannerWorksetQuotaHint {
        target_floor_count: 0,
        target_ceiling_count: 0,
        within_target_band: true,
    }
}

fn build_docs_planner_workset_balance(
    groups: &[DocsPlannerWorksetGroup],
) -> DocsPlannerWorksetBalance {
    let selection_count = groups
        .iter()
        .map(|group| group.selected_count)
        .sum::<usize>();
    let gap_kind_distribution = groups
        .iter()
        .map(|group| (group.kind, group.selected_count))
        .collect::<Vec<_>>();
    let (gap_kind_target_floor_count, gap_kind_target_ceiling_count) =
        quota_band(selection_count, gap_kind_distribution.len());
    let gap_kind_distribution = gap_kind_distribution
        .into_iter()
        .map(
            |(kind, selected_count)| DocsPlannerWorksetGapKindBalanceEntry {
                kind,
                selected_count,
                within_target_band: selected_count >= gap_kind_target_floor_count
                    && selected_count <= gap_kind_target_ceiling_count,
            },
        )
        .collect::<Vec<_>>();

    let mut family_counts =
        BTreeMap::<crate::analyzers::projection::ProjectionPageKind, usize>::new();
    for group in groups {
        for family in &group.families {
            *family_counts.entry(family.kind).or_default() += family.selected_count;
        }
    }
    let family_distribution = family_counts.into_iter().collect::<Vec<_>>();
    let (family_target_floor_count, family_target_ceiling_count) =
        quota_band(selection_count, family_distribution.len());
    let mut family_distribution = family_distribution
        .into_iter()
        .map(
            |(kind, selected_count)| DocsPlannerWorksetFamilyBalanceEntry {
                kind,
                selected_count,
                within_target_band: selected_count >= family_target_floor_count
                    && selected_count <= family_target_ceiling_count,
            },
        )
        .collect::<Vec<_>>();
    family_distribution.sort_by(|left, right| {
        right
            .selected_count
            .cmp(&left.selected_count)
            .then_with(|| left.kind.cmp(&right.kind))
    });

    let gap_kind_counts = gap_kind_distribution
        .iter()
        .map(|entry| entry.selected_count)
        .collect::<Vec<_>>();
    let family_counts = family_distribution
        .iter()
        .map(|entry| entry.selected_count)
        .collect::<Vec<_>>();
    let gap_kind_spread = spread_for_counts(&gap_kind_counts);
    let family_spread = spread_for_counts(&family_counts);

    DocsPlannerWorksetBalance {
        selection_count,
        gap_kind_group_count: gap_kind_distribution.len(),
        family_group_count: family_distribution.len(),
        gap_kind_target_floor_count,
        gap_kind_target_ceiling_count,
        family_target_floor_count,
        family_target_ceiling_count,
        gap_kind_balanced: gap_kind_spread <= 1,
        family_balanced: family_spread <= 1,
        gap_kind_distribution,
        family_distribution,
        gap_kind_spread,
        family_spread,
    }
}

fn build_docs_planner_workset_strategy(
    balance: &DocsPlannerWorksetBalance,
) -> DocsPlannerWorksetStrategy {
    let code = if balance.selection_count == 0 {
        DocsPlannerWorksetStrategyCode::EmptySelection
    } else if balance.gap_kind_group_count == 1 && balance.family_group_count == 1 {
        DocsPlannerWorksetStrategyCode::SingleLaneFocus
    } else if balance.gap_kind_group_count == 1 {
        DocsPlannerWorksetStrategyCode::FamilySplitFocus
    } else if balance.family_group_count == 1 {
        DocsPlannerWorksetStrategyCode::GapKindSplitFocus
    } else if balance.gap_kind_balanced && balance.family_balanced {
        DocsPlannerWorksetStrategyCode::BalancedMultiLane
    } else {
        DocsPlannerWorksetStrategyCode::PriorityStacked
    };

    let mut reasons = Vec::new();
    if balance.selection_count == 0 {
        reasons.push(DocsPlannerWorksetStrategyReason {
            code: DocsPlannerWorksetStrategyReasonCode::EmptySelection,
            detail: "no ranked gaps were selected into the workset".to_string(),
        });
    } else {
        let gap_reason = if balance.gap_kind_group_count == 1 {
            DocsPlannerWorksetStrategyReasonCode::SingleGapKind
        } else {
            DocsPlannerWorksetStrategyReasonCode::MultipleGapKinds
        };
        reasons.push(DocsPlannerWorksetStrategyReason {
            code: gap_reason,
            detail: format!(
                "{} populated gap-kind group(s) contribute to the workset",
                balance.gap_kind_group_count
            ),
        });

        let family_reason = if balance.family_group_count == 1 {
            DocsPlannerWorksetStrategyReasonCode::SingleFamily
        } else {
            DocsPlannerWorksetStrategyReasonCode::MultipleFamilies
        };
        reasons.push(DocsPlannerWorksetStrategyReason {
            code: family_reason,
            detail: format!(
                "{} populated page-family group(s) contribute to the workset",
                balance.family_group_count
            ),
        });

        let gap_balance_reason = if balance.gap_kind_balanced {
            DocsPlannerWorksetStrategyReasonCode::GapKindBalanced
        } else {
            DocsPlannerWorksetStrategyReasonCode::GapKindStacked
        };
        reasons.push(DocsPlannerWorksetStrategyReason {
            code: gap_balance_reason,
            detail: if balance.gap_kind_balanced {
                "gap-kind groups stay within the deterministic balance band".to_string()
            } else {
                "gap-kind groups exceed the deterministic balance band".to_string()
            },
        });

        let family_balance_reason = if balance.family_balanced {
            DocsPlannerWorksetStrategyReasonCode::FamilyBalanced
        } else {
            DocsPlannerWorksetStrategyReasonCode::FamilyStacked
        };
        reasons.push(DocsPlannerWorksetStrategyReason {
            code: family_balance_reason,
            detail: if balance.family_balanced {
                "page-family groups stay within the deterministic balance band".to_string()
            } else {
                "page-family groups exceed the deterministic balance band".to_string()
            },
        });
    }

    DocsPlannerWorksetStrategy {
        code,
        gap_kind_group_count: balance.gap_kind_group_count,
        family_group_count: balance.family_group_count,
        reasons,
    }
}

fn spread_for_counts(counts: &[usize]) -> usize {
    let Some(maximum) = counts.iter().max().copied() else {
        return 0;
    };
    let Some(minimum) = counts.iter().min().copied() else {
        return 0;
    };
    maximum.saturating_sub(minimum)
}

fn quota_band(selection_count: usize, group_count: usize) -> (usize, usize) {
    if group_count == 0 {
        return (0, 0);
    }
    let floor = selection_count / group_count;
    let ceiling = if selection_count.is_multiple_of(group_count) {
        floor
    } else {
        floor + 1
    };
    (floor, ceiling)
}

/// Load configuration, analyze one repository, and return deterministic docs-facing deep-wiki
/// planner workset bundles.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or one selected planner item
/// cannot be reopened from the projected page kernels.
pub fn docs_planner_workset_from_config_with_registry(
    query: &DocsPlannerWorksetQuery,
    config_path: Option<&Path>,
    cwd: &Path,
    registry: &PluginRegistry,
) -> Result<DocsPlannerWorksetResult, RepoIntelligenceError> {
    with_repository_analysis(&query.repo_id, config_path, cwd, registry, |analysis| {
        build_docs_planner_workset(query, analysis)
    })
}

/// Load configuration, analyze one repository, and return deterministic docs-facing deep-wiki
/// planner workset bundles.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or one selected planner item
/// cannot be reopened from the projected page kernels.
pub fn docs_planner_workset_from_config(
    query: &DocsPlannerWorksetQuery,
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<DocsPlannerWorksetResult, RepoIntelligenceError> {
    with_bootstrapped_repository_analysis(&query.repo_id, config_path, cwd, |analysis| {
        build_docs_planner_workset(query, analysis)
    })
}

fn planner_gap_search_score(
    gap: &crate::analyzers::query::ProjectedGapRecord,
    normalized_query: &str,
) -> u8 {
    let mut score = 0_u8;
    score = score.max(match_field_score(
        gap.title.as_str(),
        normalized_query,
        90,
        70,
    ));
    score = score.max(match_field_score(
        gap.path.as_str(),
        normalized_query,
        88,
        68,
    ));
    score = score.max(match_field_score(
        gap.entity_id.as_str(),
        normalized_query,
        86,
        66,
    ));
    score = score.max(match_field_score(
        gap.gap_id.as_str(),
        normalized_query,
        84,
        64,
    ));
    score = score.max(match_field_score(
        gap_kind_token(gap.kind),
        normalized_query,
        82,
        62,
    ));
    score = score.max(match_field_score(
        projection_page_kind_token(gap.page_kind),
        normalized_query,
        80,
        60,
    ));
    for format_hint in &gap.format_hints {
        score = score.max(match_field_score(
            format_hint.as_str(),
            normalized_query,
            72,
            52,
        ));
    }
    score
}

fn planner_gap_priority_breakdown(
    gap: &crate::analyzers::query::ProjectedGapRecord,
) -> (u8, Vec<DocsPlannerRankReason>) {
    let mut score: u8 = match gap.kind {
        crate::analyzers::query::ProjectedGapKind::ModuleReferenceWithoutDocumentation => 96,
        crate::analyzers::query::ProjectedGapKind::SymbolReferenceWithoutDocumentation => 92,
        crate::analyzers::query::ProjectedGapKind::DocumentationPageWithoutAnchor => 88,
        crate::analyzers::query::ProjectedGapKind::ExampleHowToWithoutAnchor => 84,
        crate::analyzers::query::ProjectedGapKind::SymbolReferenceUnverified => 72,
    };
    let mut reasons = vec![DocsPlannerRankReason {
        code: DocsPlannerRankReasonCode::GapKindBase,
        points: score,
        detail: format!(
            "base priority from projected gap kind `{}`",
            gap_kind_token(gap.kind)
        ),
    }];

    let page_bonus = match gap.page_kind {
        crate::analyzers::projection::ProjectionPageKind::Reference => Some((
            DocsPlannerRankReasonCode::ReferencePageBonus,
            2_u8,
            "reference page bonus",
        )),
        crate::analyzers::projection::ProjectionPageKind::Explanation => Some((
            DocsPlannerRankReasonCode::ExplanationPageBonus,
            1_u8,
            "explanation page bonus",
        )),
        crate::analyzers::projection::ProjectionPageKind::HowTo
        | crate::analyzers::projection::ProjectionPageKind::Tutorial => None,
    };
    if let Some((code, points, detail)) = page_bonus {
        score = score.saturating_add(points);
        reasons.push(DocsPlannerRankReason {
            code,
            points,
            detail: detail.to_string(),
        });
    }

    let module_bonus = u8::try_from(gap.module_ids.len().min(2)).unwrap_or(0);
    if module_bonus > 0 {
        score = score.saturating_add(module_bonus);
        reasons.push(DocsPlannerRankReason {
            code: DocsPlannerRankReasonCode::ModuleAnchorBonus,
            points: module_bonus,
            detail: format!("{} attached module anchor(s)", gap.module_ids.len()),
        });
    }

    let symbol_bonus = u8::try_from(gap.symbol_ids.len().min(2)).unwrap_or(0);
    if symbol_bonus > 0 {
        score = score.saturating_add(symbol_bonus);
        reasons.push(DocsPlannerRankReason {
            code: DocsPlannerRankReasonCode::SymbolAnchorBonus,
            points: symbol_bonus,
            detail: format!("{} attached symbol anchor(s)", gap.symbol_ids.len()),
        });
    }

    let example_bonus = u8::from(!gap.example_ids.is_empty());
    if example_bonus > 0 {
        score = score.saturating_add(example_bonus);
        reasons.push(DocsPlannerRankReason {
            code: DocsPlannerRankReasonCode::ExampleAnchorBonus,
            points: example_bonus,
            detail: format!("{} attached example anchor(s)", gap.example_ids.len()),
        });
    }

    let doc_bonus = u8::from(!gap.doc_ids.is_empty());
    if doc_bonus > 0 {
        score = score.saturating_add(doc_bonus);
        reasons.push(DocsPlannerRankReason {
            code: DocsPlannerRankReasonCode::DocAnchorBonus,
            points: doc_bonus,
            detail: format!("{} attached documentation anchor(s)", gap.doc_ids.len()),
        });
    }

    (score.min(100), reasons)
}

fn normalize_planner_search_text(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn match_field_score(field: &str, normalized_query: &str, exact: u8, contains: u8) -> u8 {
    let normalized_field = field.to_ascii_lowercase();
    if normalized_field == normalized_query {
        exact
    } else if normalized_field.contains(normalized_query) {
        contains
    } else {
        0
    }
}

fn gap_kind_token(kind: crate::analyzers::query::ProjectedGapKind) -> &'static str {
    match kind {
        crate::analyzers::query::ProjectedGapKind::ModuleReferenceWithoutDocumentation => {
            "module_reference_without_documentation"
        }
        crate::analyzers::query::ProjectedGapKind::SymbolReferenceWithoutDocumentation => {
            "symbol_reference_without_documentation"
        }
        crate::analyzers::query::ProjectedGapKind::SymbolReferenceUnverified => {
            "symbol_reference_unverified"
        }
        crate::analyzers::query::ProjectedGapKind::ExampleHowToWithoutAnchor => {
            "example_how_to_without_anchor"
        }
        crate::analyzers::query::ProjectedGapKind::DocumentationPageWithoutAnchor => {
            "documentation_page_without_anchor"
        }
    }
}

fn projection_page_kind_token(
    kind: crate::analyzers::projection::ProjectionPageKind,
) -> &'static str {
    match kind {
        crate::analyzers::projection::ProjectionPageKind::Reference => "reference",
        crate::analyzers::projection::ProjectionPageKind::HowTo => "how_to",
        crate::analyzers::projection::ProjectionPageKind::Tutorial => "tutorial",
        crate::analyzers::projection::ProjectionPageKind::Explanation => "explanation",
    }
}
