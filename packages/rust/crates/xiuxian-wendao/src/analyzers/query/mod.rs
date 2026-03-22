//! Query request and response contracts for repository intelligence.

mod docs;
mod example;
mod family;
mod imports;
mod index_tree;
mod module;
mod navigation;
mod overview;
mod projected_pages;
mod refine;
mod retrieval;
mod symbol;
mod sync;

pub use docs::{DocCoverageQuery, DocCoverageResult};
pub use example::{ExampleSearchHit, ExampleSearchQuery, ExampleSearchResult};
pub use family::{
    ProjectedPageFamilyCluster, ProjectedPageFamilyContextEntry, ProjectedPageFamilySearchHit,
    RepoProjectedPageFamilyClusterQuery, RepoProjectedPageFamilyClusterResult,
    RepoProjectedPageFamilyContextQuery, RepoProjectedPageFamilyContextResult,
    RepoProjectedPageFamilySearchQuery, RepoProjectedPageFamilySearchResult,
};
pub use imports::{ImportSearchHit, ImportSearchQuery, ImportSearchResult};
pub use index_tree::{
    ProjectedPageIndexNodeContext, ProjectedPageIndexNodeHit, RepoProjectedPageIndexNodeQuery,
    RepoProjectedPageIndexNodeResult, RepoProjectedPageIndexTreeQuery,
    RepoProjectedPageIndexTreeResult, RepoProjectedPageIndexTreeSearchQuery,
    RepoProjectedPageIndexTreeSearchResult, RepoProjectedPageIndexTreesQuery,
    RepoProjectedPageIndexTreesResult,
};
pub use module::{ModuleSearchHit, ModuleSearchQuery, ModuleSearchResult, RepoBacklinkItem};
pub use navigation::{
    ProjectedPageNavigationSearchHit, RepoProjectedPageNavigationQuery,
    RepoProjectedPageNavigationResult, RepoProjectedPageNavigationSearchQuery,
    RepoProjectedPageNavigationSearchResult,
};
pub use overview::{RepoOverviewQuery, RepoOverviewResult};
pub use projected_pages::{
    RepoProjectedPageQuery, RepoProjectedPageResult, RepoProjectedPageSearchQuery,
    RepoProjectedPageSearchResult, RepoProjectedPagesQuery, RepoProjectedPagesResult,
};
pub use refine::{RefineEntityDocRequest, RefineEntityDocResponse};
pub use retrieval::{
    ProjectedRetrievalHit, ProjectedRetrievalHitKind, RepoProjectedRetrievalContextQuery,
    RepoProjectedRetrievalContextResult, RepoProjectedRetrievalHitQuery,
    RepoProjectedRetrievalHitResult, RepoProjectedRetrievalQuery, RepoProjectedRetrievalResult,
};
pub use symbol::{SymbolSearchHit, SymbolSearchQuery, SymbolSearchResult};
pub use sync::{
    RepoSourceKind, RepoSyncDriftState, RepoSyncFreshnessSummary, RepoSyncHealthState,
    RepoSyncLifecycleSummary, RepoSyncMode, RepoSyncQuery, RepoSyncResult, RepoSyncRevisionSummary,
    RepoSyncStalenessState, RepoSyncState, RepoSyncStatusSummary,
};
