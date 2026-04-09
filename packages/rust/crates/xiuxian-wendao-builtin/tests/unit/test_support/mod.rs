use xiuxian_wendao_julia::compatibility::link_graph::{
    DEFAULT_JULIA_ANALYZER_EXAMPLE_CONFIG_PATH, DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH,
    build_rerank_provider_binding, julia_deployment_artifact_selector,
    julia_rerank_provider_selector,
};

use crate::test_support::{
    linked_builtin_julia_analyzer_example_config_path, linked_builtin_julia_analyzer_launcher_path,
    linked_builtin_julia_deployment_artifact_selector,
    linked_builtin_julia_rerank_provider_binding_with_endpoint,
    linked_builtin_julia_rerank_provider_selector,
};

#[test]
fn linked_builtin_host_test_helpers_match_julia_compatibility_helpers() {
    assert_eq!(
        linked_builtin_julia_analyzer_example_config_path(),
        DEFAULT_JULIA_ANALYZER_EXAMPLE_CONFIG_PATH
    );
    assert_eq!(
        linked_builtin_julia_analyzer_launcher_path(),
        DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH
    );
    assert_eq!(
        linked_builtin_julia_rerank_provider_selector(),
        julia_rerank_provider_selector()
    );
    assert_eq!(
        linked_builtin_julia_deployment_artifact_selector(),
        julia_deployment_artifact_selector()
    );
    assert_eq!(
        linked_builtin_julia_rerank_provider_binding_with_endpoint(
            "http://127.0.0.1:8090",
            "/custom-rerank",
            "/healthz",
            "v1",
            15,
        ),
        build_rerank_provider_binding(
            &xiuxian_wendao_julia::compatibility::link_graph::LinkGraphJuliaRerankRuntimeConfig {
                base_url: Some("http://127.0.0.1:8090".to_string()),
                route: Some("/custom-rerank".to_string()),
                health_route: Some("/healthz".to_string()),
                schema_version: Some("v1".to_string()),
                timeout_secs: Some(15),
                service_mode: None,
                analyzer_config_path: None,
                analyzer_strategy: None,
                vector_weight: None,
                similarity_weight: None,
            }
        )
    );
}
