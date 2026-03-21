use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::config::RepositoryRefreshPolicy;
use super::projection::{ProjectedPageIndexTree, ProjectedPageRecord, ProjectionPageKind};
use super::records::{DocRecord, ExampleRecord, ModuleRecord, SymbolRecord};

/// Query for repository source synchronization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoSyncQuery {
    /// Repository identifier to synchronize.
    pub repo_id: String,
    /// Synchronization mode applied to the repository source lifecycle.
    #[serde(default)]
    pub mode: RepoSyncMode,
}

/// Synchronization mode for repository source preparation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RepoSyncMode {
    /// Prepare the repository source while respecting the configured refresh policy.
    #[default]
    Ensure,
    /// Force a remote refresh for managed repositories before returning source state.
    Refresh,
    /// Inspect repository source state without creating or refreshing managed assets.
    Status,
}

/// Source kind resolved for one repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RepoSourceKind {
    /// A user-provided local checkout path.
    LocalCheckout,
    /// A managed checkout materialized from an upstream remote.
    ManagedRemote,
}

/// Lifecycle status reported for one repository source phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RepoSyncState {
    /// No lifecycle phase was required for this repository source.
    NotApplicable,
    /// The lifecycle asset is expected but does not currently exist.
    Missing,
    /// An existing local checkout was validated without materialization.
    Validated,
    /// An existing lifecycle asset was observed without mutation.
    Observed,
    /// A new lifecycle asset was created.
    Created,
    /// An existing lifecycle asset was reused without refresh.
    Reused,
    /// An existing lifecycle asset was refreshed in place.
    Refreshed,
}

/// Drift summary between the managed mirror and managed checkout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RepoSyncDriftState {
    /// Drift does not apply to this repository source kind.
    NotApplicable,
    /// Drift could not be determined from the currently available local metadata.
    Unknown,
    /// Mirror and checkout currently point at the same revision.
    InSync,
    /// The checkout has local commits ahead of the tracked mirror state.
    Ahead,
    /// The checkout is behind the current mirror state.
    Behind,
    /// The checkout and mirror both moved away from their last common tracked state.
    Diverged,
}

/// High-level health summary for one repository source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RepoSyncHealthState {
    /// The repository source is ready for analysis and does not currently need action.
    Healthy,
    /// One or more managed source assets are missing from the local cache.
    MissingAssets,
    /// The managed checkout is behind the current mirror state and should be refreshed.
    NeedsRefresh,
    /// The managed checkout has local commits ahead of the tracked mirror state.
    HasLocalCommits,
    /// The managed checkout and managed mirror have diverged.
    Diverged,
    /// Health could not be determined from the currently available local metadata.
    Unknown,
}

/// Freshness summary for the managed mirror fetch timestamp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RepoSyncStalenessState {
    /// Freshness does not apply to this repository source kind.
    NotApplicable,
    /// Freshness could not be determined from the currently available local metadata.
    Unknown,
    /// The managed mirror was fetched within the last hour.
    Fresh,
    /// The managed mirror was fetched within the last day, but not within the last hour.
    Aging,
    /// The managed mirror has not been fetched in more than one day.
    Stale,
}

/// Grouped lifecycle view for one repository source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoSyncLifecycleSummary {
    /// Resolved source kind.
    pub source_kind: RepoSourceKind,
    /// Lifecycle status for mirror preparation.
    pub mirror_state: RepoSyncState,
    /// Lifecycle status for checkout preparation.
    pub checkout_state: RepoSyncState,
    /// Whether a mirror asset is currently available for managed repositories.
    pub mirror_ready: bool,
    /// Whether a working checkout is currently available locally.
    pub checkout_ready: bool,
}

/// Grouped freshness view for one repository source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoSyncFreshnessSummary {
    /// Observation timestamp for this sync or status operation.
    pub checked_at: String,
    /// Last local fetch timestamp observed from the managed mirror cache.
    pub last_fetched_at: Option<String>,
    /// Freshness summary derived from the local mirror fetch timestamp.
    pub staleness_state: RepoSyncStalenessState,
}

/// Grouped revision view for one repository source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoSyncRevisionSummary {
    /// Active checkout revision after synchronization.
    pub checkout_revision: Option<String>,
    /// Active revision observed from the managed mirror branch or HEAD.
    pub mirror_revision: Option<String>,
    /// Last fetched remote-tracking revision observed from the managed checkout.
    pub tracking_revision: Option<String>,
    /// Whether the active checkout revision matches the managed mirror revision.
    pub aligned_with_mirror: bool,
}

/// Grouped status view for one repository source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoSyncStatusSummary {
    /// Lifecycle view of the repository source.
    pub lifecycle: RepoSyncLifecycleSummary,
    /// Freshness view of the repository source.
    pub freshness: RepoSyncFreshnessSummary,
    /// Revision view of the repository source.
    pub revisions: RepoSyncRevisionSummary,
    /// High-level health summary derived from lifecycle and drift state.
    pub health_state: RepoSyncHealthState,
    /// Drift summary between the managed mirror and the working checkout.
    pub drift_state: RepoSyncDriftState,
    /// Whether the repository source likely needs operator attention.
    pub attention_required: bool,
}

/// Repository source synchronization result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoSyncResult {
    /// Repository identifier.
    pub repo_id: String,
    /// Synchronization mode that was applied.
    pub mode: RepoSyncMode,
    /// Resolved source kind.
    pub source_kind: RepoSourceKind,
    /// Refresh policy applied to the repository source.
    pub refresh: RepositoryRefreshPolicy,
    /// Lifecycle status for mirror preparation.
    pub mirror_state: RepoSyncState,
    /// Lifecycle status for checkout preparation.
    pub checkout_state: RepoSyncState,
    /// Absolute path to the working checkout used for analysis.
    pub checkout_path: String,
    /// Absolute path to the managed mirror, when remote materialization is used.
    pub mirror_path: Option<String>,
    /// Observation timestamp for this sync or status operation.
    pub checked_at: String,
    /// Last local fetch timestamp observed from the managed mirror cache.
    pub last_fetched_at: Option<String>,
    /// Active revision observed from the managed mirror branch or HEAD.
    pub mirror_revision: Option<String>,
    /// Last fetched remote-tracking revision observed from the managed checkout.
    pub tracking_revision: Option<String>,
    /// Upstream URL declared by configuration or discovered from the checkout.
    pub upstream_url: Option<String>,
    /// Drift summary between the managed mirror and the working checkout.
    pub drift_state: RepoSyncDriftState,
    /// High-level health summary derived from lifecycle and drift state.
    pub health_state: RepoSyncHealthState,
    /// Freshness summary derived from the local mirror fetch timestamp.
    pub staleness_state: RepoSyncStalenessState,
    /// Grouped status summary for agent-facing consumption.
    pub status_summary: RepoSyncStatusSummary,
    /// Active checkout revision after synchronization.
    pub revision: Option<String>,
}

/// Query for repository overview data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoOverviewQuery {
    /// Repository identifier to summarize.
    pub repo_id: String,
}

/// Minimal repository overview response for the MVP surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoOverviewResult {
    /// Repository identifier.
    pub repo_id: String,
    /// Primary display name.
    pub display_name: String,
    /// Optional revision string.
    pub revision: Option<String>,
    /// Count of normalized modules.
    pub module_count: usize,
    /// Count of normalized symbols.
    pub symbol_count: usize,
    /// Count of normalized examples.
    pub example_count: usize,
    /// Count of normalized docs.
    pub doc_count: usize,
    /// Optional repository-level hierarchical URI for path mapping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hierarchical_uri: Option<String>,
    /// Optional repository hierarchy segments for breadcrumbs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hierarchy: Option<Vec<String>>,
}

/// Query for module lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ModuleSearchQuery {
    /// Repository identifier to search within.
    pub repo_id: String,
    /// User-provided search string.
    pub query: String,
    /// Maximum number of rows to return.
    pub limit: usize,
}

/// Structured backlink metadata derived from relation records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoBacklinkItem {
    /// Stable backlink identifier (typically a doc id).
    pub id: String,
    /// Optional display title of the backlink source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional repository-relative path of the backlink source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Optional relation kind label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// One enriched module-search hit with ranking and projection metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ModuleSearchHit {
    /// The normalized module record.
    pub module: ModuleRecord,
    /// Optional normalized relevance score (0-1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    /// Optional stable rank in the returned hit set (1-based).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<usize>,
    /// Optional saliency score (0-1) for mixed-source ordering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saliency_score: Option<f64>,
    /// Optional hierarchical URI for path mapping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hierarchical_uri: Option<String>,
    /// Optional hierarchy segments for breadcrumbs and drawers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hierarchy: Option<Vec<String>>,
    /// Optional implicit backlinks derived from `documents` relations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implicit_backlinks: Option<Vec<String>>,
    /// Optional structured backlink metadata derived from `documents` relations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implicit_backlink_items: Option<Vec<RepoBacklinkItem>>,
    /// Optional projected-page identifiers that reference this module.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_page_ids: Option<Vec<String>>,
}

/// Result set for module lookup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ModuleSearchResult {
    /// Repository identifier searched.
    pub repo_id: String,
    /// Matching module rows.
    pub modules: Vec<ModuleRecord>,
    /// Enriched module hits with ranking/backlink/projection context.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub module_hits: Vec<ModuleSearchHit>,
}

/// Query for symbol lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SymbolSearchQuery {
    /// Repository identifier to search within.
    pub repo_id: String,
    /// User-provided search string.
    pub query: String,
    /// Maximum number of rows to return.
    pub limit: usize,
}

/// One enriched symbol-search hit with ranking and projection metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SymbolSearchHit {
    /// The normalized symbol record.
    pub symbol: SymbolRecord,
    /// Optional normalized relevance score (0-1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    /// Optional stable rank in the returned hit set (1-based).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<usize>,
    /// Optional saliency score (0-1) for mixed-source ordering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saliency_score: Option<f64>,
    /// Optional hierarchical URI for path mapping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hierarchical_uri: Option<String>,
    /// Optional hierarchy segments for breadcrumbs and drawers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hierarchy: Option<Vec<String>>,
    /// Optional implicit backlinks derived from `documents` relations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implicit_backlinks: Option<Vec<String>>,
    /// Optional structured backlink metadata derived from `documents` relations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implicit_backlink_items: Option<Vec<RepoBacklinkItem>>,
    /// Optional projected-page identifiers that reference this symbol.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_page_ids: Option<Vec<String>>,
    /// Optional audit status echoed from source records.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_status: Option<String>,
    /// Optional verification state derived from audit status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_state: Option<String>,
}

/// Result set for symbol lookup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SymbolSearchResult {
    /// Repository identifier searched.
    pub repo_id: String,
    /// Matching symbol rows.
    pub symbols: Vec<SymbolRecord>,
    /// Enriched symbol hits with ranking/backlink/projection context.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub symbol_hits: Vec<SymbolSearchHit>,
}

/// Query for example lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExampleSearchQuery {
    /// Repository identifier to search within.
    pub repo_id: String,
    /// User-provided search string.
    pub query: String,
    /// Maximum number of rows to return.
    pub limit: usize,
}

/// One enriched example-search hit with ranking and projection metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ExampleSearchHit {
    /// The normalized example record.
    pub example: ExampleRecord,
    /// Optional normalized relevance score (0-1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score: Option<f64>,
    /// Optional stable rank in the returned hit set (1-based).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<usize>,
    /// Optional saliency score (0-1) for mixed-source ordering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saliency_score: Option<f64>,
    /// Optional hierarchical URI for path mapping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hierarchical_uri: Option<String>,
    /// Optional hierarchy segments for breadcrumbs and drawers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hierarchy: Option<Vec<String>>,
    /// Optional implicit backlinks derived from `documents` relations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implicit_backlinks: Option<Vec<String>>,
    /// Optional structured backlink metadata derived from `documents` relations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implicit_backlink_items: Option<Vec<RepoBacklinkItem>>,
    /// Optional projected-page identifiers that reference this example.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_page_ids: Option<Vec<String>>,
}

/// Result set for example lookup.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ExampleSearchResult {
    /// Repository identifier searched.
    pub repo_id: String,
    /// Matching example rows.
    pub examples: Vec<ExampleRecord>,
    /// Enriched example hits with ranking/backlink/projection context.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub example_hits: Vec<ExampleSearchHit>,
}

/// Query for documentation coverage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocCoverageQuery {
    /// Repository identifier to search within.
    pub repo_id: String,
    /// Optional module identifier scope.
    pub module_id: Option<String>,
}

/// Minimal documentation coverage response for the MVP surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DocCoverageResult {
    /// Repository identifier searched.
    pub repo_id: String,
    /// Optional module identifier scope.
    pub module_id: Option<String>,
    /// Documentation rows relevant to the requested scope.
    pub docs: Vec<DocRecord>,
    /// Count of covered symbols in scope.
    pub covered_symbols: usize,
    /// Count of uncovered symbols in scope.
    pub uncovered_symbols: usize,
    /// Optional repository-level hierarchical URI for path mapping.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hierarchical_uri: Option<String>,
    /// Optional repository hierarchy segments for breadcrumbs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hierarchy: Option<Vec<String>>,
}

/// Query for deterministic projected pages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPagesQuery {
    /// Repository identifier to project.
    pub repo_id: String,
}

/// Deterministic projected-page result set for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPagesResult {
    /// Repository identifier projected.
    pub repo_id: String,
    /// Deterministic projected pages derived from repository truth.
    pub pages: Vec<ProjectedPageRecord>,
}

/// Query for deterministic projected-page lookup by stable page identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageQuery {
    /// Repository identifier to project.
    pub repo_id: String,
    /// Stable projected page identifier.
    pub page_id: String,
}

/// Deterministic projected-page lookup result for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageResult {
    /// Repository identifier projected.
    pub repo_id: String,
    /// The requested deterministic projected page.
    pub page: ProjectedPageRecord,
}

/// Query for deterministic projected-page retrieval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageSearchQuery {
    /// Repository identifier to project.
    pub repo_id: String,
    /// User-provided projected-page search string.
    pub query: String,
    /// Optional projected-page family filter.
    pub kind: Option<ProjectionPageKind>,
    /// Maximum number of projected pages to return.
    pub limit: usize,
}

/// Deterministic projected-page search result set for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageSearchResult {
    /// Repository identifier projected.
    pub repo_id: String,
    /// Matching deterministic projected pages.
    pub pages: Vec<ProjectedPageRecord>,
}

/// Retrieval hit family emitted by deterministic Stage-2 mixed retrieval.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ProjectedRetrievalHitKind {
    /// A projected-page level hit.
    Page,
    /// A builder-native projected page-index node hit.
    PageIndexNode,
}

/// One deterministic Stage-2 mixed retrieval hit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectedRetrievalHit {
    /// Retrieval hit family.
    pub kind: ProjectedRetrievalHitKind,
    /// Owning projected page record.
    pub page: ProjectedPageRecord,
    /// Optional builder-native projected page-index node hit.
    pub node: Option<ProjectedPageIndexNodeHit>,
}

/// Query for deterministic Stage-2 mixed retrieval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedRetrievalQuery {
    /// Repository identifier to project.
    pub repo_id: String,
    /// User-provided retrieval search string.
    pub query: String,
    /// Optional projected-page family filter.
    pub kind: Option<ProjectionPageKind>,
    /// Maximum number of mixed retrieval hits to return.
    pub limit: usize,
}

/// Deterministic Stage-2 mixed retrieval result set for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedRetrievalResult {
    /// Repository identifier projected.
    pub repo_id: String,
    /// Matching deterministic projected-page and page-index-node hits.
    pub hits: Vec<ProjectedRetrievalHit>,
}

/// Query for deterministic Stage-2 mixed retrieval hit lookup by stable identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedRetrievalHitQuery {
    /// Repository identifier to project.
    pub repo_id: String,
    /// Stable projected page identifier.
    pub page_id: String,
    /// Optional stable page-index node identifier.
    pub node_id: Option<String>,
}

/// Deterministic Stage-2 mixed retrieval hit lookup result for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedRetrievalHitResult {
    /// Repository identifier projected.
    pub repo_id: String,
    /// The requested deterministic mixed retrieval hit.
    pub hit: ProjectedRetrievalHit,
}

/// Deterministic local context around one projected page-index node hit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectedPageIndexNodeContext {
    /// Ancestor nodes ordered from root to immediate parent.
    pub ancestors: Vec<ProjectedPageIndexNodeHit>,
    /// Previous sibling node within the same parent scope.
    pub previous_sibling: Option<ProjectedPageIndexNodeHit>,
    /// Next sibling node within the same parent scope.
    pub next_sibling: Option<ProjectedPageIndexNodeHit>,
    /// Direct child nodes under the requested node.
    pub children: Vec<ProjectedPageIndexNodeHit>,
}

/// Query for deterministic Stage-2 retrieval context around one stable hit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedRetrievalContextQuery {
    /// Repository identifier to project.
    pub repo_id: String,
    /// Stable projected page identifier.
    pub page_id: String,
    /// Optional stable page-index node identifier.
    pub node_id: Option<String>,
    /// Maximum number of related projected pages to return.
    pub related_limit: usize,
}

/// Deterministic Stage-2 retrieval context result for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedRetrievalContextResult {
    /// Repository identifier projected.
    pub repo_id: String,
    /// The requested center hit.
    pub center: ProjectedRetrievalHit,
    /// Related projected pages sharing stable anchors with the center page.
    pub related_pages: Vec<ProjectedPageRecord>,
    /// Optional builder-native node neighborhood when `node_id` is present.
    pub node_context: Option<ProjectedPageIndexNodeContext>,
}

/// One deterministic page-family context entry ranked by shared stable anchors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectedPageFamilyContextEntry {
    /// Shared-anchor score between the center page and this related page.
    pub shared_anchor_score: usize,
    /// Deterministic projected page related to the center page.
    pub page: ProjectedPageRecord,
}

/// One deterministic page-family cluster around a projected page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectedPageFamilyCluster {
    /// Diataxis-aligned projected page family for this cluster.
    pub kind: ProjectionPageKind,
    /// Related projected pages in this family ordered by deterministic evidence.
    pub pages: Vec<ProjectedPageFamilyContextEntry>,
}

/// Query for deterministic Stage-2 page-family context around one stable projected page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageFamilyContextQuery {
    /// Repository identifier to project.
    pub repo_id: String,
    /// Stable projected page identifier.
    pub page_id: String,
    /// Maximum number of related projected pages to return for each page family.
    pub per_kind_limit: usize,
}

/// Deterministic Stage-2 page-family context result for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageFamilyContextResult {
    /// Repository identifier projected.
    pub repo_id: String,
    /// The requested center page.
    pub center_page: ProjectedPageRecord,
    /// Related projected pages grouped by projected page family.
    pub families: Vec<ProjectedPageFamilyCluster>,
}

/// One deterministic projected page-family search hit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectedPageFamilySearchHit {
    /// The matched projected center page.
    pub center_page: ProjectedPageRecord,
    /// Related projected pages grouped by page family around the matched page.
    pub families: Vec<ProjectedPageFamilyCluster>,
}

/// Query for deterministic Stage-2 page-family cluster search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageFamilySearchQuery {
    /// Repository identifier to project.
    pub repo_id: String,
    /// User-provided projected-page search string for center pages.
    pub query: String,
    /// Optional projected-page family filter applied to center pages.
    pub kind: Option<ProjectionPageKind>,
    /// Maximum number of center-page hits to return.
    pub limit: usize,
    /// Maximum number of related projected pages to return for each page family.
    pub per_kind_limit: usize,
}

/// Deterministic Stage-2 page-family cluster search result for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageFamilySearchResult {
    /// Repository identifier projected.
    pub repo_id: String,
    /// Matching center pages with grouped deterministic family clusters.
    pub hits: Vec<ProjectedPageFamilySearchHit>,
}

/// Query for deterministic Stage-2 singular page-family cluster lookup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageFamilyClusterQuery {
    /// Repository identifier to project.
    pub repo_id: String,
    /// Stable projected page identifier.
    pub page_id: String,
    /// Requested projected-page family for the returned cluster.
    pub kind: ProjectionPageKind,
    /// Maximum number of related projected pages to return in the requested family cluster.
    pub limit: usize,
}

/// Deterministic Stage-2 singular page-family cluster lookup result for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageFamilyClusterResult {
    /// Repository identifier projected.
    pub repo_id: String,
    /// The requested center page.
    pub center_page: ProjectedPageRecord,
    /// The requested related projected page family.
    pub family: ProjectedPageFamilyCluster,
}

/// Query for deterministic Stage-2 page-centric navigation around one stable projected page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageNavigationQuery {
    /// Repository identifier to project.
    pub repo_id: String,
    /// Stable projected page identifier.
    pub page_id: String,
    /// Optional stable page-index node identifier.
    pub node_id: Option<String>,
    /// Optional projected-page family to include as a deterministic cluster.
    pub family_kind: Option<ProjectionPageKind>,
    /// Maximum number of related projected pages to return.
    pub related_limit: usize,
    /// Maximum number of related projected pages to return in the requested family cluster.
    pub family_limit: usize,
}

/// Deterministic Stage-2 page-centric navigation bundle for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageNavigationResult {
    /// Repository identifier projected.
    pub repo_id: String,
    /// The requested center hit.
    pub center: ProjectedRetrievalHit,
    /// Related projected pages sharing stable anchors with the center page.
    pub related_pages: Vec<ProjectedPageRecord>,
    /// Optional builder-native node neighborhood when `node_id` is present.
    pub node_context: Option<ProjectedPageIndexNodeContext>,
    /// Builder-native projected page-index tree for the requested page.
    pub tree: ProjectedPageIndexTree,
    /// Optional deterministic related projected page family for the requested page.
    pub family_cluster: Option<ProjectedPageFamilyCluster>,
}

/// One deterministic projected page-navigation search hit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectedPageNavigationSearchHit {
    /// Stable ordering score derived from the projected page match.
    pub search_score: u8,
    /// Deterministic page-centric navigation bundle for the matched projected page.
    pub navigation: RepoProjectedPageNavigationResult,
}

/// Query for deterministic projected page-navigation search.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageNavigationSearchQuery {
    /// Repository identifier to project.
    pub repo_id: String,
    /// User-provided projected-page search string for center pages.
    pub query: String,
    /// Optional projected-page family filter applied to center pages.
    pub kind: Option<ProjectionPageKind>,
    /// Optional projected-page family to include as a deterministic cluster for each hit.
    pub family_kind: Option<ProjectionPageKind>,
    /// Maximum number of center-page hits to return.
    pub limit: usize,
    /// Maximum number of related projected pages to return for each matched center page.
    pub related_limit: usize,
    /// Maximum number of related projected pages to return in the requested family cluster.
    pub family_limit: usize,
}

/// Deterministic projected page-navigation search result for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageNavigationSearchResult {
    /// Repository identifier projected.
    pub repo_id: String,
    /// Matching center pages with deterministic navigation bundles.
    pub hits: Vec<ProjectedPageNavigationSearchHit>,
}

/// Query for deterministic projected page-index trees.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageIndexTreesQuery {
    /// Repository identifier to project.
    pub repo_id: String,
}

/// Query for deterministic projected page-index tree lookup by stable page identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageIndexTreeQuery {
    /// Repository identifier to project.
    pub repo_id: String,
    /// Stable projected page identifier.
    pub page_id: String,
}

/// Deterministic projected page-index tree lookup result for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageIndexTreeResult {
    /// Repository identifier projected.
    pub repo_id: String,
    /// The requested deterministic projected page-index tree.
    pub tree: ProjectedPageIndexTree,
}

/// Query for deterministic projected page-index node lookup by stable identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageIndexNodeQuery {
    /// Repository identifier to project.
    pub repo_id: String,
    /// Stable projected page identifier.
    pub page_id: String,
    /// Stable page-index node identifier.
    pub node_id: String,
}

/// One deterministic section-level retrieval hit inside a projected page-index tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectedPageIndexNodeHit {
    /// Owning repository identifier.
    pub repo_id: String,
    /// Stable projected page identifier.
    pub page_id: String,
    /// Human-readable projected page title.
    pub page_title: String,
    /// Diataxis-aligned projected page family.
    pub page_kind: ProjectionPageKind,
    /// Virtual markdown path used for parsing the projected page.
    pub path: String,
    /// Parsed document identifier as seen by the markdown parser.
    pub doc_id: String,
    /// Stable page-index node identifier.
    pub node_id: String,
    /// Human-readable node title.
    pub node_title: String,
    /// Structural path carried by the page-index builder.
    pub structural_path: Vec<String>,
    /// Inclusive 1-based source line range.
    pub line_range: (usize, usize),
    /// Node text payload after optional thinning.
    pub text: String,
}

/// Deterministic projected page-index node lookup result for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageIndexNodeResult {
    /// Repository identifier projected.
    pub repo_id: String,
    /// The requested deterministic projected page-index node hit.
    pub hit: ProjectedPageIndexNodeHit,
}

/// Query for deterministic projected page-index tree retrieval.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageIndexTreeSearchQuery {
    /// Repository identifier to project.
    pub repo_id: String,
    /// User-provided search string.
    pub query: String,
    /// Optional projected-page family filter.
    pub kind: Option<ProjectionPageKind>,
    /// Maximum number of projected page-index node hits to return.
    pub limit: usize,
}

/// Deterministic projected page-index tree retrieval result set for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageIndexTreeSearchResult {
    /// Repository identifier projected.
    pub repo_id: String,
    /// Matching deterministic section-level hits.
    pub hits: Vec<ProjectedPageIndexNodeHit>,
}

/// Deterministic projected page-index tree result set for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoProjectedPageIndexTreesResult {
    /// Repository identifier projected.
    pub repo_id: String,
    /// Deterministic projected page-index trees derived from repository truth.
    pub trees: Vec<ProjectedPageIndexTree>,
}
