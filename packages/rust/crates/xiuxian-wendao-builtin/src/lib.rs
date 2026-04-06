//! Builtin plugin bundle and registry bootstrap for Wendao hosts.

#[cfg(feature = "julia")]
mod artifacts;
mod link;
mod retrieval_policy;
#[cfg(feature = "julia")]
mod test_support;

use xiuxian_wendao_core::repo_intelligence::{
    PluginRegistry, RepoIntelligenceError, builtin_plugin_registrars,
};

#[cfg(feature = "julia")]
pub use artifacts::{
    linked_builtin_julia_deployment_artifact_openapi_json_example,
    linked_builtin_julia_deployment_artifact_openapi_toml_example,
    linked_builtin_julia_gateway_artifact_base_url,
    linked_builtin_julia_gateway_artifact_default_strategy,
    linked_builtin_julia_gateway_artifact_expected_json_fragments,
    linked_builtin_julia_gateway_artifact_expected_toml_fragments,
    linked_builtin_julia_gateway_artifact_path, linked_builtin_julia_gateway_artifact_route,
    linked_builtin_julia_gateway_artifact_rpc_params_fixture,
    linked_builtin_julia_gateway_artifact_runtime_config_toml,
    linked_builtin_julia_gateway_artifact_schema_version,
    linked_builtin_julia_gateway_artifact_selected_transport,
    linked_builtin_julia_gateway_artifact_ui_payload_fixture,
    linked_builtin_julia_gateway_launcher_path,
    linked_builtin_plugin_artifact_openapi_json_example,
    linked_builtin_plugin_artifact_openapi_toml_example,
    render_builtin_plugin_artifact_toml_for_selector,
    render_builtin_plugin_artifact_toml_for_selector_with_settings,
    resolve_builtin_plugin_artifact_for_selector,
    resolve_builtin_plugin_artifact_for_selector_with_settings,
};
pub use retrieval_policy::{
    BuiltinRerankRuntimeProjection, resolve_builtin_rerank_runtime_projection_with_settings,
};
#[cfg(feature = "julia")]
pub use test_support::{
    GRAPH_STRUCTURAL_ANCHOR_PLANES_COLUMN, GRAPH_STRUCTURAL_ANCHOR_VALUES_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_EDGE_DESTINATIONS_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_EDGE_KINDS_COLUMN, GRAPH_STRUCTURAL_CANDIDATE_EDGE_SOURCES_COLUMN,
    GRAPH_STRUCTURAL_CANDIDATE_ID_COLUMN, GRAPH_STRUCTURAL_CANDIDATE_NODE_IDS_COLUMN,
    GRAPH_STRUCTURAL_CONSTRAINT_KIND_COLUMN, GRAPH_STRUCTURAL_DEPENDENCY_SCORE_COLUMN,
    GRAPH_STRUCTURAL_EDGE_CONSTRAINT_KINDS_COLUMN, GRAPH_STRUCTURAL_KEYWORD_SCORE_COLUMN,
    GRAPH_STRUCTURAL_QUERY_ID_COLUMN, GRAPH_STRUCTURAL_QUERY_MAX_LAYERS_COLUMN,
    GRAPH_STRUCTURAL_REQUIRED_BOUNDARY_SIZE_COLUMN, GRAPH_STRUCTURAL_RETRIEVAL_LAYER_COLUMN,
    GRAPH_STRUCTURAL_SEMANTIC_SCORE_COLUMN, GRAPH_STRUCTURAL_TAG_SCORE_COLUMN,
    GraphStructuralFilterConstraint, GraphStructuralFilterRequestRow,
    GraphStructuralFilterScoreRow, GraphStructuralRawConnectedPairCollectionCandidateInputs,
    GraphStructuralRerankScoreRow, LinkedBuiltinWendaoArrowScoreRow,
    build_graph_structural_filter_request_batch,
    build_graph_structural_generic_topology_candidate_inputs,
    build_graph_structural_generic_topology_candidate_inputs_from_pair_collection,
    build_graph_structural_generic_topology_candidate_inputs_from_raw_connected_pairs,
    build_graph_structural_generic_topology_candidate_inputs_from_scored_pair_collection,
    build_graph_structural_generic_topology_candidate_metadata_inputs,
    build_graph_structural_generic_topology_candidate_metadata_inputs_from_pair_collection,
    build_graph_structural_generic_topology_filter_request_batch,
    build_graph_structural_generic_topology_filter_request_batch_from_raw_connected_pair_collections,
    build_graph_structural_generic_topology_rerank_request_batch_from_raw_connected_pair_collections,
    build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs,
    build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_raw_candidates,
    build_graph_structural_keyword_overlap_query_inputs,
    build_graph_structural_keyword_overlap_raw_candidate_inputs,
    build_graph_structural_keyword_tag_query_context, build_graph_structural_pair_candidate_inputs,
    build_graph_structural_raw_connected_pair_collection_candidate_inputs,
    build_graph_structural_raw_connected_pair_collection_candidate_inputs_from_raw_tuples,
    build_graph_structural_raw_connected_pair_inputs,
    build_graph_structural_scored_pair_candidate_inputs,
    fetch_graph_structural_filter_rows_for_repository,
    fetch_graph_structural_generic_topology_filter_rows_for_repository,
    fetch_graph_structural_generic_topology_filter_rows_for_repository_from_raw_connected_pair_collections,
    fetch_graph_structural_generic_topology_rerank_rows_for_repository,
    fetch_graph_structural_generic_topology_rerank_rows_for_repository_from_raw_connected_pair_collections,
    fetch_graph_structural_keyword_overlap_pair_rerank_rows_for_repository_from_raw_candidates,
    linked_builtin_julia_analyzer_example_config_path, linked_builtin_julia_analyzer_launcher_path,
    linked_builtin_julia_deployment_artifact_selector,
    linked_builtin_julia_planned_search_openai_runtime_config_toml,
    linked_builtin_julia_planned_search_similarity_only_runtime_config_toml,
    linked_builtin_julia_planned_search_vector_store_runtime_config_toml,
    linked_builtin_julia_rerank_provider_binding_with_endpoint,
    linked_builtin_julia_rerank_provider_selector,
    linked_builtin_spawn_wendaoanalyzer_similarity_only_service,
    linked_builtin_spawn_wendaoanalyzer_stream_linear_blend_service,
    linked_builtin_spawn_wendaoarrow_custom_scoring_service,
    linked_builtin_spawn_wendaoarrow_stream_metadata_service,
    linked_builtin_spawn_wendaoarrow_stream_scoring_service,
    linked_builtin_spawn_wendaosearch_solver_demo_multi_route_service,
    linked_builtin_spawn_wendaosearch_solver_demo_structural_rerank_service,
};

/// Register built-in repo-intelligence plugins into a fresh registry.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] if a linked builtin plugin registrar
/// fails while registering into the fresh registry.
pub fn bootstrap_builtin_registry() -> Result<PluginRegistry, RepoIntelligenceError> {
    link::ensure_builtin_plugins_linked();
    let mut registry = PluginRegistry::new();
    let mut registrars = builtin_plugin_registrars();
    registrars.sort_by(|left, right| left.plugin_id().cmp(right.plugin_id()));
    for registrar in registrars {
        registrar.register(&mut registry)?;
    }

    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::bootstrap_builtin_registry;

    #[cfg(not(any(feature = "julia", feature = "modelica")))]
    #[test]
    fn bootstrap_builtin_registry_succeeds_without_feature_plugins() {
        let registry = bootstrap_builtin_registry()
            .unwrap_or_else(|error| panic!("builtin registry bootstrap should succeed: {error}"));

        assert!(
            registry.plugin_ids().is_empty(),
            "default bundle build should not link feature-gated builtin plugins"
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn bootstrap_builtin_registry_registers_julia_plugin() {
        let registry = bootstrap_builtin_registry()
            .unwrap_or_else(|error| panic!("builtin registry bootstrap should succeed: {error}"));

        assert!(
            registry.get("julia").is_some(),
            "builtin registry should include the external Julia plugin"
        );
    }

    #[cfg(feature = "modelica")]
    #[test]
    fn bootstrap_builtin_registry_registers_modelica_plugin() {
        let registry = bootstrap_builtin_registry()
            .unwrap_or_else(|error| panic!("builtin registry bootstrap should succeed: {error}"));

        assert!(
            registry.get("modelica").is_some(),
            "builtin registry should include the external Modelica plugin"
        );
    }
}
