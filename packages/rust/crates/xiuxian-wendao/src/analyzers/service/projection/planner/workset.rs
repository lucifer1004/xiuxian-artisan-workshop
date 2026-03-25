use std::collections::BTreeMap;
use std::path::Path;

use crate::analyzers::errors::RepoIntelligenceError;
use crate::analyzers::plugin::RepositoryAnalysisOutput;
use crate::analyzers::query::{
    DocsPlannerItemQuery, DocsPlannerItemResult, DocsPlannerQueueQuery, DocsPlannerQueueResult,
    DocsPlannerRankHit, DocsPlannerRankQuery, DocsPlannerWorksetBalance,
    DocsPlannerWorksetFamilyBalanceEntry, DocsPlannerWorksetFamilyGroup,
    DocsPlannerWorksetGapKindBalanceEntry, DocsPlannerWorksetGroup, DocsPlannerWorksetQuery,
    DocsPlannerWorksetQuotaHint, DocsPlannerWorksetResult, DocsPlannerWorksetStrategy,
    DocsPlannerWorksetStrategyCode, DocsPlannerWorksetStrategyReason,
    DocsPlannerWorksetStrategyReasonCode,
};
use crate::analyzers::registry::PluginRegistry;

use super::api::{build_docs_planner_item, build_docs_planner_queue, build_docs_planner_rank};
use crate::analyzers::service::projection::registry::{
    with_bootstrapped_repository_analysis, with_repository_analysis,
};

pub(super) fn planner_queue_snapshot(
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

pub(super) fn planner_ranked_hits(
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

pub(super) fn open_planner_items(
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

pub(super) fn build_planner_workset_groups(
    ranked_hits: &[DocsPlannerRankHit],
    items: &[DocsPlannerItemResult],
) -> Vec<DocsPlannerWorksetGroup> {
    let mut groups = initial_planner_workset_groups(ranked_hits, items);
    populate_family_groups(&mut groups);
    apply_gap_kind_quota_hints(&mut groups);
    groups
}

/// Build a deterministic docs-facing deep-wiki planner workset from ranked projected gaps.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when repository analysis fails or one selected planner item
/// cannot be reopened from the projected page kernels.
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

pub(super) fn build_docs_planner_workset_balance(
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

pub(super) fn build_docs_planner_workset_strategy(
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

pub(super) fn quota_band(selection_count: usize, group_count: usize) -> (usize, usize) {
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
