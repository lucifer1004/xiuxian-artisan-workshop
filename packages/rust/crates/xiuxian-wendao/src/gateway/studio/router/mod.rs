//! Studio API router for Qianji frontend.
//!
//! Provides HTTP endpoints for VFS operations, graph queries, and UI configuration.

/// Code-AST response builders and repository/path resolution helpers.
pub mod code_ast;
pub mod config;
mod error;
pub mod handlers;
mod repository;
mod routes;
pub mod sanitization;
mod state;

pub use code_ast::build_code_ast_analysis_response;
pub use config::{
    load_ui_config_from_wendao_toml, persist_ui_config_to_wendao_toml, resolve_studio_config_root,
    studio_wendao_toml_path,
};
pub use error::{StudioApiError, map_repo_intelligence_error};
pub use handlers::{
    code_ast, doc_coverage, example_search, get_ui_config, graph_neighbors, markdown,
    module_search, node_neighbors, overview, projected_page, projected_page_family_cluster,
    projected_page_family_context, projected_page_family_search, projected_page_index_node,
    projected_page_index_tree, projected_page_index_tree_search, projected_page_index_trees,
    projected_page_navigation, projected_page_navigation_search, projected_page_search,
    projected_pages, projected_retrieval, projected_retrieval_context, projected_retrieval_hit,
    refine_entity_doc, set_ui_config, symbol_search, sync, topology_3d, vfs_cat, vfs_entry,
    vfs_resolve, vfs_root_entries, vfs_scan,
};
pub use repository::{configured_repositories, configured_repository};
pub use routes::{studio_router, studio_routes};
pub use sanitization::{
    sanitize_path_like, sanitize_path_list, sanitize_projects, sanitize_repo_projects,
};
pub use state::{GatewayState, StudioState};

#[cfg(test)]
mod tests;
