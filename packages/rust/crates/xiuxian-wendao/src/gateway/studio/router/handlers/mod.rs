//! Studio API endpoint handlers.

pub mod analysis;
mod analysis_exports;
pub mod capabilities;
mod capabilities_exports;
/// Docs-facing deep-wiki planning handlers.
#[path = "docs/mod.rs"]
pub mod docs;
mod docs_exports;
pub mod graph;
mod graph_exports;
pub mod repo;
mod repo_exports;
pub mod ui_config;
mod ui_config_exports;
pub mod vfs;
mod vfs_exports;

pub use analysis_exports::{
    code_ast, code_ast_retrieval_arrow, markdown, markdown_retrieval_arrow,
};
pub use capabilities_exports::{
    get_compat_deployment_artifact, get_plugin_artifact, get_ui_capabilities,
};
pub use docs_exports::{
    docs_family_cluster, docs_family_context, docs_family_search, docs_navigation,
    docs_navigation_search, docs_page, docs_planner_item, docs_planner_queue, docs_planner_rank,
    docs_planner_search, docs_planner_workset, docs_projected_gap_report, docs_retrieval,
    docs_retrieval_context, docs_retrieval_hit, docs_search,
};
pub use graph_exports::{graph_neighbors, node_neighbors, topology_3d};
pub use repo_exports::{
    doc_coverage, example_search, import_search, module_search, overview, projected_page,
    projected_page_family_cluster, projected_page_family_context, projected_page_family_search,
    projected_page_index_node, projected_page_index_tree, projected_page_index_tree_search,
    projected_page_index_trees, projected_page_navigation, projected_page_navigation_search,
    projected_page_search, projected_pages, projected_retrieval, projected_retrieval_context,
    projected_retrieval_hit, refine_entity_doc, repo_index, repo_index_status, symbol_search, sync,
};
pub use ui_config_exports::{get_ui_config, set_ui_config};
pub use vfs_exports::{vfs_cat, vfs_entry, vfs_resolve, vfs_root_entries, vfs_scan};
