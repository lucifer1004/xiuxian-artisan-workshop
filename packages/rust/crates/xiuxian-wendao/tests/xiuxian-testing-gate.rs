//! Test-structure policy gate for xiuxian-wendao.

use std::path::Path;

use xiuxian_testing::assert_crate_tests_structure_with_workspace_config;

#[cfg(not(feature = "performance"))]
#[path = "integration/coactivation_multihop_diffusion.rs"]
mod coactivation_multihop_diffusion;

#[cfg(not(feature = "performance"))]
#[path = "integration/coactivation_weighted_propagation.rs"]
mod coactivation_weighted_propagation;

#[cfg(not(feature = "performance"))]
#[path = "integration/planned_search_semantic_ignition.rs"]
mod planned_search_semantic_ignition;

#[cfg(not(feature = "performance"))]
#[path = "integration/ppr_weight_precision.rs"]
mod ppr_weight_precision;

#[cfg(not(feature = "performance"))]
#[path = "integration/quantum_fusion_openai_ignition.rs"]
mod quantum_fusion_openai_ignition;

#[cfg(not(feature = "performance"))]
#[path = "integration/quantum_fusion_saliency_budget.rs"]
mod quantum_fusion_saliency_budget;

#[cfg(not(feature = "performance"))]
#[path = "integration/quantum_fusion_saliency_window.rs"]
mod quantum_fusion_saliency_window;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_doc_coverage.rs"]
mod repo_doc_coverage;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_example_search.rs"]
mod repo_example_search;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_intelligence_registry.rs"]
mod repo_intelligence_registry;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_module_search.rs"]
mod repo_module_search;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_overview.rs"]
mod repo_overview;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_projected_page.rs"]
mod repo_projected_page;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_projected_page_index_documents.rs"]
mod repo_projected_page_index_documents;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_projected_page_index_node.rs"]
mod repo_projected_page_index_node;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_projected_page_index_tree.rs"]
mod repo_projected_page_index_tree;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_projected_page_index_tree_search.rs"]
mod repo_projected_page_index_tree_search;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_projected_page_index_trees.rs"]
mod repo_projected_page_index_trees;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_projected_page_search.rs"]
mod repo_projected_page_search;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_projected_pages.rs"]
mod repo_projected_pages;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_projected_retrieval.rs"]
mod repo_projected_retrieval;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_projected_retrieval_context.rs"]
mod repo_projected_retrieval_context;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_projected_retrieval_hit.rs"]
mod repo_projected_retrieval_hit;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_projection_inputs.rs"]
mod repo_projection_inputs;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_relations.rs"]
mod repo_relations;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_symbol_search.rs"]
mod repo_symbol_search;

#[cfg(not(feature = "performance"))]
#[path = "integration/repo_sync.rs"]
mod repo_sync;

#[cfg(feature = "performance")]
#[path = "performance/mod.rs"]
mod performance;

#[cfg(feature = "performance-stress")]
#[path = "performance/stress/mod.rs"]
mod performance_stress;

#[test]
fn enforce_tests_structure_gate() {
    assert_crate_tests_structure_with_workspace_config(Path::new(env!("CARGO_MANIFEST_DIR")));
}
