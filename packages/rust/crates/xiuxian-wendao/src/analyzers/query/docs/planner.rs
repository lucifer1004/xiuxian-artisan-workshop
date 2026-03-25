use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::analyzers::projection::ProjectionPageKind;
use crate::analyzers::query::{
    ProjectedGapKind, ProjectedGapRecord, ProjectedRetrievalHit, RepoProjectedPageNavigationResult,
};

/// Docs-facing query for one deterministic deep-wiki planner item opened by stable gap
/// identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerItemQuery {
    /// Repository identifier to inspect.
    pub repo_id: String,
    /// Stable projected gap identifier.
    pub gap_id: String,
    /// Optional projected-page family to include as a deterministic cluster.
    pub family_kind: Option<ProjectionPageKind>,
    /// Maximum number of related projected pages to return.
    pub related_limit: usize,
    /// Maximum number of related projected pages to return in the requested family cluster.
    pub family_limit: usize,
}

/// Docs-facing deterministic planner item bundle for one stable projected gap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerItemResult {
    /// Repository identifier inspected.
    pub repo_id: String,
    /// The requested deterministic projected gap.
    pub gap: ProjectedGapRecord,
    /// Deterministic mixed retrieval hit for the gap page.
    pub hit: ProjectedRetrievalHit,
    /// Deterministic navigation bundle for the gap page.
    pub navigation: RepoProjectedPageNavigationResult,
}

/// Docs-facing query for deterministic deep-wiki planner discovery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerSearchQuery {
    /// Repository identifier to inspect.
    pub repo_id: String,
    /// User-provided planner search string.
    pub query: String,
    /// Optional projected gap kind filter.
    pub gap_kind: Option<ProjectedGapKind>,
    /// Optional projected-page family filter.
    pub page_kind: Option<ProjectionPageKind>,
    /// Maximum number of planner hits to return.
    pub limit: usize,
}

/// One deterministic planner discovery hit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerSearchHit {
    /// Stable ordering score derived from deterministic planner evidence.
    pub search_score: u8,
    /// Matching deterministic projected gap.
    pub gap: ProjectedGapRecord,
}

/// Docs-facing deterministic planner discovery result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerSearchResult {
    /// Repository identifier inspected.
    pub repo_id: String,
    /// Matching deterministic planner gaps.
    pub hits: Vec<DocsPlannerSearchHit>,
}

/// Docs-facing query for deterministic deep-wiki planner queue shaping.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerQueueQuery {
    /// Repository identifier to inspect.
    pub repo_id: String,
    /// Optional projected gap kind filter.
    pub gap_kind: Option<ProjectedGapKind>,
    /// Optional projected-page family filter.
    pub page_kind: Option<ProjectionPageKind>,
    /// Maximum number of preview gaps to return for each gap kind.
    pub per_kind_limit: usize,
}

/// One grouped deterministic planner queue lane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerQueueGroup {
    /// Projected gap kind carried by this queue group.
    pub kind: ProjectedGapKind,
    /// Total number of matching gaps in this group before preview truncation.
    pub count: usize,
    /// Deterministic preview of matching gaps in this group.
    pub gaps: Vec<ProjectedGapRecord>,
}

/// Docs-facing deterministic planner queue result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerQueueResult {
    /// Repository identifier inspected.
    pub repo_id: String,
    /// Number of projected pages considered in the underlying gap report.
    pub page_count: usize,
    /// Number of matching gaps across all queue groups.
    pub total_gap_count: usize,
    /// Deterministic gap groups for planner queue shaping.
    pub groups: Vec<DocsPlannerQueueGroup>,
}

/// Docs-facing query for deterministic deep-wiki planner ranking.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerRankQuery {
    /// Repository identifier to inspect.
    pub repo_id: String,
    /// Optional projected gap kind filter.
    pub gap_kind: Option<ProjectedGapKind>,
    /// Optional projected-page family filter.
    pub page_kind: Option<ProjectionPageKind>,
    /// Maximum number of ranked planner gaps to return.
    pub limit: usize,
}

/// Machine-readable deterministic priority reason code for one ranked planner gap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DocsPlannerRankReasonCode {
    /// Base score derived from the projected gap kind.
    GapKindBase,
    /// Bonus applied when the gap page is a `Reference` page.
    ReferencePageBonus,
    /// Bonus applied when the gap page is an `Explanation` page.
    ExplanationPageBonus,
    /// Bonus derived from attached module anchors.
    ModuleAnchorBonus,
    /// Bonus derived from attached symbol anchors.
    SymbolAnchorBonus,
    /// Bonus derived from attached example anchors.
    ExampleAnchorBonus,
    /// Bonus derived from attached documentation anchors.
    DocAnchorBonus,
}

/// One deterministic priority reason for a ranked planner gap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerRankReason {
    /// Machine-readable reason code.
    pub code: DocsPlannerRankReasonCode,
    /// Number of priority points contributed by this reason.
    pub points: u8,
    /// Deterministic human-readable explanation for the contribution.
    pub detail: String,
}

/// One deterministic planner ranking hit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerRankHit {
    /// Stable deterministic planner priority score.
    pub priority_score: u8,
    /// Deterministic explanation of the score composition.
    pub reasons: Vec<DocsPlannerRankReason>,
    /// Matching deterministic projected gap.
    pub gap: ProjectedGapRecord,
}

/// Docs-facing deterministic planner ranking result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerRankResult {
    /// Repository identifier inspected.
    pub repo_id: String,
    /// Ranked deterministic planner gaps.
    pub hits: Vec<DocsPlannerRankHit>,
}

/// Docs-facing query for deterministic deep-wiki planner workset opening.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerWorksetQuery {
    /// Repository identifier to inspect.
    pub repo_id: String,
    /// Optional projected gap kind filter.
    pub gap_kind: Option<ProjectedGapKind>,
    /// Optional projected-page family filter.
    pub page_kind: Option<ProjectionPageKind>,
    /// Maximum number of preview gaps to keep for each gap kind before batch opening.
    pub per_kind_limit: usize,
    /// Maximum number of planner items to open across the queue preview.
    pub limit: usize,
    /// Optional projected-page family to include as a deterministic cluster in each navigation bundle.
    pub family_kind: Option<ProjectionPageKind>,
    /// Maximum number of related projected pages to return for each opened planner item.
    pub related_limit: usize,
    /// Maximum number of related projected pages to return in the requested family cluster.
    pub family_limit: usize,
}

/// Docs-facing deterministic planner workset result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerWorksetResult {
    /// Repository identifier inspected.
    pub repo_id: String,
    /// Deterministic queue snapshot used to choose the workset.
    pub queue: DocsPlannerQueueResult,
    /// Deterministic ranked planner gaps selected for opening.
    pub ranked_hits: Vec<DocsPlannerRankHit>,
    /// Deterministic balancing summary for the selected workset.
    pub balance: DocsPlannerWorksetBalance,
    /// Deterministic batch-strategy summary for the selected workset.
    pub strategy: DocsPlannerWorksetStrategy,
    /// Deterministic grouped workset lanes derived from the ranked selection.
    pub groups: Vec<DocsPlannerWorksetGroup>,
    /// Opened deterministic planner-item bundles selected from the ranked gaps.
    pub items: Vec<DocsPlannerItemResult>,
}

/// One deterministic grouped planner workset lane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerWorksetGroup {
    /// Projected gap kind carried by this grouped workset lane.
    pub kind: ProjectedGapKind,
    /// Number of ranked gaps selected into this group.
    pub selected_count: usize,
    /// Deterministic quota hint for this grouped workset lane.
    pub quota: DocsPlannerWorksetQuotaHint,
    /// Family-aware grouped workset lanes nested inside this gap-kind group.
    pub families: Vec<DocsPlannerWorksetFamilyGroup>,
    /// Ranked hits selected for this group, preserving global rank order.
    pub ranked_hits: Vec<DocsPlannerRankHit>,
    /// Opened planner-item bundles for this group, preserving global rank order.
    pub items: Vec<DocsPlannerItemResult>,
}

/// One deterministic family-aware grouped planner workset lane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerWorksetFamilyGroup {
    /// Projected page family carried by this nested workset lane.
    pub kind: ProjectionPageKind,
    /// Number of ranked gaps selected into this family group.
    pub selected_count: usize,
    /// Deterministic quota hint for this nested family group.
    pub quota: DocsPlannerWorksetQuotaHint,
    /// Ranked hits selected for this family group, preserving global rank order.
    pub ranked_hits: Vec<DocsPlannerRankHit>,
    /// Opened planner-item bundles for this family group, preserving global rank order.
    pub items: Vec<DocsPlannerItemResult>,
}

/// One deterministic distribution entry for workset balancing summaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerWorksetGapKindBalanceEntry {
    /// Projected gap kind described by this entry.
    pub kind: ProjectedGapKind,
    /// Number of selected ranked gaps in this gap kind.
    pub selected_count: usize,
    /// Whether this gap-kind group stays within the deterministic quota band.
    pub within_target_band: bool,
}

/// One deterministic family distribution entry for workset balancing summaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerWorksetFamilyBalanceEntry {
    /// Projected page family described by this entry.
    pub kind: ProjectionPageKind,
    /// Number of selected ranked gaps in this page family.
    pub selected_count: usize,
    /// Whether this page-family group stays within the deterministic quota band.
    pub within_target_band: bool,
}

/// Deterministic quota-band hint for one selected planner group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerWorksetQuotaHint {
    /// Deterministic lower quota bound for this group.
    pub target_floor_count: usize,
    /// Deterministic upper quota bound for this group.
    pub target_ceiling_count: usize,
    /// Whether the selected count stays within the deterministic quota band.
    pub within_target_band: bool,
}

/// Machine-readable batch-strategy code for one planner workset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DocsPlannerWorksetStrategyCode {
    /// No gaps were selected into the workset.
    EmptySelection,
    /// The workset concentrates on one gap kind and one page family.
    SingleLaneFocus,
    /// The workset concentrates on one gap kind but spans multiple families.
    FamilySplitFocus,
    /// The workset spans multiple gap kinds but stays in one family.
    GapKindSplitFocus,
    /// The workset spans multiple lanes and remains balanced across them.
    BalancedMultiLane,
    /// The workset spans multiple lanes but keeps a priority-stacked shape.
    PriorityStacked,
}

/// Machine-readable reason code for one planner workset batch strategy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DocsPlannerWorksetStrategyReasonCode {
    /// No selected gaps exist.
    EmptySelection,
    /// Exactly one gap kind was selected.
    SingleGapKind,
    /// Multiple gap kinds were selected.
    MultipleGapKinds,
    /// Exactly one page family was selected.
    SingleFamily,
    /// Multiple page families were selected.
    MultipleFamilies,
    /// Gap-kind lanes remain within the deterministic balance band.
    GapKindBalanced,
    /// Gap-kind lanes exceed the deterministic balance band.
    GapKindStacked,
    /// Page-family lanes remain within the deterministic balance band.
    FamilyBalanced,
    /// Page-family lanes exceed the deterministic balance band.
    FamilyStacked,
}

/// One deterministic reason contributing to the planner workset batch strategy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerWorksetStrategyReason {
    /// Machine-readable reason code.
    pub code: DocsPlannerWorksetStrategyReasonCode,
    /// Deterministic human-readable explanation for the strategy evidence.
    pub detail: String,
}

/// Deterministic batch-strategy summary for one planner workset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerWorksetStrategy {
    /// Machine-readable workset batch-strategy code.
    pub code: DocsPlannerWorksetStrategyCode,
    /// Number of populated gap-kind groups contributing to the strategy.
    pub gap_kind_group_count: usize,
    /// Number of populated page-family groups contributing to the strategy.
    pub family_group_count: usize,
    /// Deterministic rationale for the selected batch strategy.
    pub reasons: Vec<DocsPlannerWorksetStrategyReason>,
}

/// Deterministic balancing evidence for one planner workset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocsPlannerWorksetBalance {
    /// Number of ranked gaps selected into this workset.
    pub selection_count: usize,
    /// Number of populated projected gap-kind groups in the selected workset.
    pub gap_kind_group_count: usize,
    /// Number of populated projected page-family groups in the selected workset.
    pub family_group_count: usize,
    /// Deterministic lower quota bound for each populated gap-kind group.
    pub gap_kind_target_floor_count: usize,
    /// Deterministic upper quota bound for each populated gap-kind group.
    pub gap_kind_target_ceiling_count: usize,
    /// Deterministic lower quota bound for each populated page-family group.
    pub family_target_floor_count: usize,
    /// Deterministic upper quota bound for each populated page-family group.
    pub family_target_ceiling_count: usize,
    /// Distribution of selected ranked gaps by projected gap kind.
    pub gap_kind_distribution: Vec<DocsPlannerWorksetGapKindBalanceEntry>,
    /// Distribution of selected ranked gaps by projected page family.
    pub family_distribution: Vec<DocsPlannerWorksetFamilyBalanceEntry>,
    /// Maximum selected-count spread across populated gap-kind groups.
    pub gap_kind_spread: usize,
    /// Maximum selected-count spread across populated page-family groups.
    pub family_spread: usize,
    /// Whether populated gap-kind groups differ by at most one selected hit.
    pub gap_kind_balanced: bool,
    /// Whether populated family groups differ by at most one selected hit.
    pub family_balanced: bool,
}
