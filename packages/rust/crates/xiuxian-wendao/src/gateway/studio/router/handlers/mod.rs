//! Studio API endpoint handlers.

pub mod analysis;
pub mod graph;
pub mod repo;
pub mod ui_config;
pub mod vfs;

pub use analysis::{code_ast, markdown};
pub use graph::{graph_neighbors, node_neighbors, topology_3d};
pub use repo::{
    doc_coverage, example_search, module_search, overview, projected_page,
    projected_page_family_cluster, projected_page_family_context, projected_page_family_search,
    projected_page_index_node, projected_page_index_tree, projected_page_index_tree_search,
    projected_page_index_trees, projected_page_navigation, projected_page_navigation_search,
    projected_page_search, projected_pages, projected_retrieval, projected_retrieval_context,
    projected_retrieval_hit, refine_entity_doc, repo_index, repo_index_status, symbol_search, sync,
};
pub use ui_config::{get as get_ui_config, set as set_ui_config};
pub use vfs::{
    cat as vfs_cat, entry as vfs_entry, resolve as vfs_resolve, root_entries as vfs_root_entries,
    scan as vfs_scan,
};
