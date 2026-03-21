//! Repo Intelligence common-core contracts and plugin registry.
//!
//! This module defines the initial Wendao-native contracts for repository
//! intelligence. The first landing focuses on:
//!
//! - repository registration metadata
//! - normalized records for repository understanding
//! - query request/response contracts
//! - plugin registration and dispatch boundaries

mod checkout;
mod config;
mod errors;
mod julia;
mod modelica;
mod plugin;
mod projection;
mod query;
mod records;
mod registry;
mod service;

pub use config::{
    RegisteredRepository, RepoIntelligenceConfig, RepositoryPluginConfig, RepositoryRef,
    RepositoryRefreshPolicy, load_repo_intelligence_config,
};
pub use errors::RepoIntelligenceError;
pub use plugin::{
    AnalysisContext, PluginAnalysisOutput, PluginLinkContext, RepoIntelligencePlugin,
    RepoSourceFile, RepositoryAnalysisOutput,
};
pub use projection::{
    ProjectedMarkdownDocument, ProjectedPageIndexDocument, ProjectedPageIndexNode,
    ProjectedPageIndexSection, ProjectedPageIndexTree, ProjectedPageRecord, ProjectedPageSection,
    ProjectionInputBundle, ProjectionPageKind, ProjectionPageSeed, build_projected_page,
    build_projected_page_family_cluster, build_projected_page_family_context,
    build_projected_page_family_search, build_projected_page_index_documents,
    build_projected_page_index_node, build_projected_page_index_tree,
    build_projected_page_index_tree_search, build_projected_page_index_trees,
    build_projected_page_navigation, build_projected_page_navigation_search,
    build_projected_page_search, build_projected_pages, build_projected_retrieval,
    build_projected_retrieval_context, build_projected_retrieval_hit, build_projection_inputs,
    render_projected_markdown_documents,
};
pub use modelica::ModelicaRepoIntelligencePlugin;
pub use query::{
    DocCoverageQuery, DocCoverageResult, ExampleSearchHit, ExampleSearchQuery, ExampleSearchResult,
    ModuleSearchHit, ModuleSearchQuery, ModuleSearchResult, ProjectedPageFamilyCluster,
    ProjectedPageFamilyContextEntry, ProjectedPageFamilySearchHit, ProjectedPageIndexNodeContext,
    ProjectedPageIndexNodeHit, ProjectedPageNavigationSearchHit, ProjectedRetrievalHit,
    ProjectedRetrievalHitKind, RepoOverviewQuery, RepoOverviewResult,
    RepoBacklinkItem,
    RepoProjectedPageFamilyClusterQuery, RepoProjectedPageFamilyClusterResult,
    RepoProjectedPageFamilyContextQuery, RepoProjectedPageFamilyContextResult,
    RepoProjectedPageFamilySearchQuery, RepoProjectedPageFamilySearchResult,
    RepoProjectedPageIndexNodeQuery, RepoProjectedPageIndexNodeResult,
    RepoProjectedPageIndexTreeQuery, RepoProjectedPageIndexTreeResult,
    RepoProjectedPageIndexTreeSearchQuery, RepoProjectedPageIndexTreeSearchResult,
    RepoProjectedPageIndexTreesQuery, RepoProjectedPageIndexTreesResult,
    RepoProjectedPageNavigationQuery, RepoProjectedPageNavigationResult,
    RepoProjectedPageNavigationSearchQuery, RepoProjectedPageNavigationSearchResult,
    RepoProjectedPageQuery, RepoProjectedPageResult, RepoProjectedPageSearchQuery,
    RepoProjectedPageSearchResult, RepoProjectedPagesQuery, RepoProjectedPagesResult,
    RepoProjectedRetrievalContextQuery, RepoProjectedRetrievalContextResult,
    RepoProjectedRetrievalHitQuery, RepoProjectedRetrievalHitResult, RepoProjectedRetrievalQuery,
    RepoProjectedRetrievalResult, RepoSourceKind, RepoSyncDriftState, RepoSyncFreshnessSummary,
    RepoSyncHealthState, RepoSyncLifecycleSummary, RepoSyncMode, RepoSyncQuery, RepoSyncResult,
    RepoSyncRevisionSummary, RepoSyncStalenessState, RepoSyncState, RepoSyncStatusSummary,
    SymbolSearchHit, SymbolSearchQuery, SymbolSearchResult,
};
pub use records::{
    DiagnosticRecord, DocRecord, ExampleRecord, ModuleRecord, RelationKind, RelationRecord,
    RepoSymbolKind, RepositoryRecord, SymbolRecord,
};
pub use registry::PluginRegistry;
pub use service::{
    analyze_registered_repository, analyze_registered_repository_with_registry,
    analyze_repository_from_config, analyze_repository_from_config_with_registry,
    bootstrap_builtin_registry, build_doc_coverage, build_example_search, build_module_search,
    build_repo_overview, build_repo_projected_page, build_repo_projected_page_family_cluster,
    build_repo_projected_page_family_context, build_repo_projected_page_family_search,
    build_repo_projected_page_index_node, build_repo_projected_page_index_tree,
    build_repo_projected_page_index_tree_search, build_repo_projected_page_index_trees,
    build_repo_projected_page_navigation, build_repo_projected_page_navigation_search,
    build_repo_projected_page_search, build_repo_projected_pages, build_repo_projected_retrieval,
    build_repo_projected_retrieval_context, build_repo_projected_retrieval_hit,
    build_symbol_search, doc_coverage_from_config, doc_coverage_from_config_with_registry,
    example_search_from_config, example_search_from_config_with_registry,
    load_registered_repository, module_search_from_config, module_search_from_config_with_registry,
    repo_overview_from_config, repo_overview_from_config_with_registry,
    repo_projected_page_family_cluster_from_config,
    repo_projected_page_family_cluster_from_config_with_registry,
    repo_projected_page_family_context_from_config,
    repo_projected_page_family_context_from_config_with_registry,
    repo_projected_page_family_search_from_config,
    repo_projected_page_family_search_from_config_with_registry, repo_projected_page_from_config,
    repo_projected_page_from_config_with_registry, repo_projected_page_index_node_from_config,
    repo_projected_page_index_node_from_config_with_registry,
    repo_projected_page_index_tree_from_config,
    repo_projected_page_index_tree_from_config_with_registry,
    repo_projected_page_index_tree_search_from_config,
    repo_projected_page_index_tree_search_from_config_with_registry,
    repo_projected_page_index_trees_from_config,
    repo_projected_page_index_trees_from_config_with_registry,
    repo_projected_page_navigation_from_config,
    repo_projected_page_navigation_from_config_with_registry,
    repo_projected_page_navigation_search_from_config,
    repo_projected_page_navigation_search_from_config_with_registry,
    repo_projected_page_search_from_config, repo_projected_page_search_from_config_with_registry,
    repo_projected_pages_from_config, repo_projected_pages_from_config_with_registry,
    repo_projected_retrieval_context_from_config,
    repo_projected_retrieval_context_from_config_with_registry,
    repo_projected_retrieval_from_config, repo_projected_retrieval_from_config_with_registry,
    repo_projected_retrieval_hit_from_config,
    repo_projected_retrieval_hit_from_config_with_registry,
    repo_sync_for_registered_repository, repo_sync_from_config,
    symbol_search_from_config, symbol_search_from_config_with_registry,
};
