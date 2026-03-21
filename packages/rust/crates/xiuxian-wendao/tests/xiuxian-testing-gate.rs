//! Test-structure policy gate for xiuxian-wendao.

use std::path::Path;

use xiuxian_testing::assert_crate_tests_structure_with_workspace_config;

#[path = "integration/coactivation_multihop_diffusion.rs"]
mod coactivation_multihop_diffusion;

#[path = "integration/coactivation_weighted_propagation.rs"]
mod coactivation_weighted_propagation;

#[path = "integration/planned_search_semantic_ignition.rs"]
mod planned_search_semantic_ignition;

#[path = "integration/ppr_weight_precision.rs"]
mod ppr_weight_precision;

#[path = "integration/quantum_fusion_openai_ignition.rs"]
mod quantum_fusion_openai_ignition;

#[path = "integration/quantum_fusion_saliency_budget.rs"]
mod quantum_fusion_saliency_budget;

#[path = "integration/quantum_fusion_saliency_window.rs"]
mod quantum_fusion_saliency_window;

#[path = "integration/repo_doc_coverage.rs"]
mod repo_doc_coverage;

#[path = "integration/repo_example_search.rs"]
mod repo_example_search;

#[path = "integration/repo_intelligence_registry.rs"]
mod repo_intelligence_registry;

#[path = "integration/repo_module_search.rs"]
mod repo_module_search;

#[path = "integration/repo_overview.rs"]
mod repo_overview;

#[path = "integration/repo_projected_page.rs"]
mod repo_projected_page;

#[path = "integration/repo_projected_page_index_documents.rs"]
mod repo_projected_page_index_documents;

#[path = "integration/repo_projected_page_index_node.rs"]
mod repo_projected_page_index_node;

#[path = "integration/repo_projected_page_index_tree.rs"]
mod repo_projected_page_index_tree;

#[path = "integration/repo_projected_page_index_tree_search.rs"]
mod repo_projected_page_index_tree_search;

#[path = "integration/repo_projected_page_index_trees.rs"]
mod repo_projected_page_index_trees;

#[path = "integration/repo_projected_page_search.rs"]
mod repo_projected_page_search;

#[path = "integration/repo_projected_pages.rs"]
mod repo_projected_pages;

#[path = "integration/repo_projected_retrieval.rs"]
mod repo_projected_retrieval;

#[path = "integration/repo_projected_retrieval_context.rs"]
mod repo_projected_retrieval_context;

#[path = "integration/repo_projected_retrieval_hit.rs"]
mod repo_projected_retrieval_hit;

#[path = "integration/repo_projection_inputs.rs"]
mod repo_projection_inputs;

#[path = "integration/repo_relations.rs"]
mod repo_relations;

#[path = "integration/repo_symbol_search.rs"]
mod repo_symbol_search;

#[path = "integration/repo_sync.rs"]
mod repo_sync;

#[test]
fn enforce_tests_structure_gate() {
    assert_crate_tests_structure_with_workspace_config(Path::new(env!("CARGO_MANIFEST_DIR")));
}
