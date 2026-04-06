use super::{
    resolve_link_graph_agentic_runtime, resolve_link_graph_coactivation_runtime,
    resolve_link_graph_retrieval_policy_runtime,
};
#[cfg(feature = "julia")]
use super::{
    resolve_link_graph_rerank_binding, resolve_link_graph_rerank_flight_runtime_settings,
    resolve_link_graph_rerank_schema_version, resolve_link_graph_rerank_score_weights,
};
use crate::link_graph::runtime_config::models::LinkGraphSemanticIgnitionBackend;
use crate::link_graph::set_link_graph_wendao_config_override;
use serial_test::serial;
use std::fs;
#[cfg(feature = "julia")]
use xiuxian_wendao_builtin::{
    linked_builtin_julia_analyzer_example_config_path, linked_builtin_julia_analyzer_launcher_path,
    linked_builtin_julia_deployment_artifact_selector,
    linked_builtin_julia_rerank_provider_selector,
};
use xiuxian_wendao_runtime::config::{
    DEFAULT_LINK_GRAPH_COACTIVATION_HOP_DECAY_SCALE, DEFAULT_LINK_GRAPH_COACTIVATION_MAX_HOPS,
    DEFAULT_LINK_GRAPH_COACTIVATION_MAX_NEIGHBORS_PER_DIRECTION,
    DEFAULT_LINK_GRAPH_COACTIVATION_TOUCH_QUEUE_DEPTH,
};
use xiuxian_wendao_runtime::transport::CANONICAL_PLUGIN_TRANSPORT_PREFERENCE_ORDER;
#[cfg(feature = "julia")]
use xiuxian_wendao_runtime::transport::RerankScoreWeights;

#[cfg(feature = "julia")]
fn configure_julia_rerank_runtime_fixture() -> Result<tempfile::TempDir, Box<dyn std::error::Error>>
{
    let temp = tempfile::tempdir()?;
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        format!(
            r#"[link_graph.retrieval]
mode = "hybrid"

[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:8088"
route = "/rerank"
health_route = "/healthz"
schema_version = "v1"
timeout_secs = 15
service_mode = "stream"
analyzer_config_path = "{config_path}"
analyzer_strategy = "similarity_only"
vector_weight = 0.2
similarity_weight = 0.8
"#,
            config_path = linked_builtin_julia_analyzer_example_config_path()
        ),
    )?;
    set_link_graph_wendao_config_override(&config_path.to_string_lossy());
    Ok(temp)
}

#[test]
fn test_coactivation_touch_queue_depth_default() {
    let runtime = resolve_link_graph_coactivation_runtime();
    assert_eq!(runtime.max_hops, DEFAULT_LINK_GRAPH_COACTIVATION_MAX_HOPS);
    assert_eq!(
        runtime.max_total_propagations,
        DEFAULT_LINK_GRAPH_COACTIVATION_MAX_NEIGHBORS_PER_DIRECTION.saturating_mul(2)
    );
    assert!(
        (runtime.hop_decay_scale - DEFAULT_LINK_GRAPH_COACTIVATION_HOP_DECAY_SCALE).abs()
            <= f64::EPSILON,
        "unexpected hop_decay_scale: {}",
        runtime.hop_decay_scale
    );
    assert_eq!(
        runtime.touch_queue_depth,
        DEFAULT_LINK_GRAPH_COACTIVATION_TOUCH_QUEUE_DEPTH
    );
}

#[test]
#[serial]
fn test_agentic_runtime_resolves_override_values() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.agentic.suggested_link]
max_entries = 111
ttl_seconds = 600

[link_graph.agentic.search]
include_provisional_default = true
provisional_limit = 17

[link_graph.agentic.expansion]
max_workers = 3
max_candidates = 90
max_pairs_per_worker = 11
time_budget_ms = 44.0

[link_graph.agentic.execution]
worker_time_budget_ms = 33.0
persist_suggestions_default = true
persist_retry_attempts = 4
idempotency_scan_limit = 77
relation = "supports"
agent_id = "runtime-agent"
evidence_prefix = "runtime-prefix"
"#,
    )?;
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let runtime = resolve_link_graph_agentic_runtime();
    assert_eq!(runtime.suggested_link_max_entries, 111);
    assert_eq!(runtime.suggested_link_ttl_seconds, Some(600));
    assert!(runtime.search_include_provisional_default);
    assert_eq!(runtime.search_provisional_limit, 17);
    assert_eq!(runtime.expansion_max_workers, 3);
    assert_eq!(runtime.expansion_max_candidates, 90);
    assert_eq!(runtime.expansion_max_pairs_per_worker, 11);
    assert!((runtime.expansion_time_budget_ms - 44.0).abs() <= f64::EPSILON);
    assert!((runtime.execution_worker_time_budget_ms - 33.0).abs() <= f64::EPSILON);
    assert!(runtime.execution_persist_suggestions_default);
    assert_eq!(runtime.execution_persist_retry_attempts, 4);
    assert_eq!(runtime.execution_idempotency_scan_limit, 77);
    assert_eq!(runtime.execution_relation, "supports");
    assert_eq!(runtime.execution_agent_id, "runtime-agent");
    assert_eq!(runtime.execution_evidence_prefix, "runtime-prefix");

    Ok(())
}

#[test]
#[serial]
fn test_retrieval_runtime_resolves_semantic_ignition_config()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config_path = temp.path().join("wendao.toml");
    let shared_path = temp.path().join("wendao.shared.toml");
    fs::write(
        &shared_path,
        r#"[semantic_ignition]
backend = "openai-compatible"
vector_store_path = ".cache/glm-anchor-store"
table_name = "glm_anchor_index"
embedding_base_url = "http://127.0.0.1:11434"
embedding_model = "glm-5"
"#,
    )?;
    fs::write(
        &config_path,
        r#"[link_graph.retrieval]
imports = ["wendao.shared.toml"]
mode = "hybrid"
candidate_multiplier = 3
max_sources = 5
graph_rows_per_source = 4
"#,
    )?;
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let runtime = resolve_link_graph_retrieval_policy_runtime();
    assert_eq!(
        runtime.semantic_ignition.backend,
        LinkGraphSemanticIgnitionBackend::OpenAiCompatible
    );
    assert_eq!(runtime.candidate_multiplier, 3);
    assert_eq!(runtime.max_sources, 5);
    assert_eq!(runtime.graph_rows_per_source, 4);
    assert_eq!(
        runtime.semantic_ignition.vector_store_path.as_deref(),
        Some(".cache/glm-anchor-store")
    );
    assert_eq!(
        runtime.semantic_ignition.table_name.as_deref(),
        Some("glm_anchor_index")
    );
    assert_eq!(
        runtime.semantic_ignition.embedding_base_url.as_deref(),
        Some("http://127.0.0.1:11434")
    );
    assert_eq!(
        runtime.semantic_ignition.embedding_model.as_deref(),
        Some("glm-5")
    );
    assert!(runtime.rerank_binding().is_none());
    assert!(runtime.rerank_schema_version().is_none());
    assert!(runtime.rerank_score_weights().is_none());

    Ok(())
}

#[cfg(feature = "julia")]
#[test]
#[serial]
fn test_retrieval_runtime_resolves_julia_rerank_config() -> Result<(), Box<dyn std::error::Error>> {
    let _temp = configure_julia_rerank_runtime_fixture()?;

    let runtime = resolve_link_graph_retrieval_policy_runtime();
    let Some(binding) = runtime.rerank_binding() else {
        panic!("generic rerank binding");
    };

    assert_eq!(
        binding.selector,
        linked_builtin_julia_rerank_provider_selector()
    );
    assert_eq!(
        binding.endpoint.base_url.as_deref(),
        Some("http://127.0.0.1:8088")
    );
    assert_eq!(binding.endpoint.route.as_deref(), Some("/rerank"));
    assert_eq!(binding.endpoint.health_route.as_deref(), Some("/healthz"));
    assert_eq!(binding.endpoint.timeout_secs, Some(15));
    assert_eq!(
        binding
            .launch
            .as_ref()
            .map(|launch| launch.launcher_path.as_str()),
        Some(linked_builtin_julia_analyzer_launcher_path())
    );
    assert_eq!(runtime.rerank_schema_version().as_deref(), Some("v1"));
    let score_weights = match RerankScoreWeights::new(0.2, 0.8) {
        Ok(weights) => weights,
        Err(error) => panic!("valid weight fixture should construct: {error}"),
    };
    assert_eq!(runtime.rerank_score_weights(), Some(score_weights));

    Ok(())
}

#[cfg(feature = "julia")]
#[test]
#[serial]
fn test_retrieval_runtime_projects_julia_rerank_host_helpers()
-> Result<(), Box<dyn std::error::Error>> {
    let _temp = configure_julia_rerank_runtime_fixture()?;

    let runtime = resolve_link_graph_retrieval_policy_runtime();
    let Some(score_weights) = resolve_link_graph_rerank_score_weights() else {
        panic!("score weights should resolve");
    };
    assert!((score_weights.vector_weight - 0.2).abs() < f64::EPSILON);
    assert!((score_weights.semantic_weight - 0.8).abs() < f64::EPSILON);
    assert_eq!(
        resolve_link_graph_rerank_schema_version().as_deref(),
        Some("v1")
    );
    let flight_settings = resolve_link_graph_rerank_flight_runtime_settings();
    assert_eq!(flight_settings.schema_version.as_deref(), Some("v1"));
    let Some(flight_weights) = flight_settings.score_weights else {
        panic!("flight score weights should resolve");
    };
    assert!((flight_weights.vector_weight - 0.2).abs() < f64::EPSILON);
    assert!((flight_weights.semantic_weight - 0.8).abs() < f64::EPSILON);
    let Some(binding) = runtime.rerank_binding() else {
        panic!("generic rerank binding");
    };
    assert_eq!(
        binding.selector,
        linked_builtin_julia_rerank_provider_selector()
    );
    assert_eq!(
        binding.endpoint.base_url.as_deref(),
        Some("http://127.0.0.1:8088")
    );
    assert_eq!(binding.endpoint.route.as_deref(), Some("/rerank"));
    assert_eq!(
        binding.transport,
        xiuxian_wendao_core::transport::PluginTransportKind::ArrowFlight
    );
    assert_eq!(binding.endpoint.health_route.as_deref(), Some("/healthz"));
    assert_eq!(binding.endpoint.timeout_secs, Some(15));
    assert_eq!(binding.contract_version.0, "v1");
    assert_eq!(
        binding
            .launch
            .as_ref()
            .map(|launch| launch.launcher_path.as_str()),
        Some(linked_builtin_julia_analyzer_launcher_path())
    );

    let Some(resolved_binding) = resolve_link_graph_rerank_binding() else {
        panic!("resolved generic rerank binding");
    };
    assert_eq!(
        resolved_binding.selector,
        linked_builtin_julia_rerank_provider_selector()
    );
    assert_eq!(
        resolved_binding.endpoint.base_url.as_deref(),
        Some("http://127.0.0.1:8088")
    );

    Ok(())
}

#[test]
fn canonical_transport_preference_order_is_flight_first() {
    assert_eq!(
        CANONICAL_PLUGIN_TRANSPORT_PREFERENCE_ORDER,
        [xiuxian_wendao_core::transport::PluginTransportKind::ArrowFlight]
    );
}

#[cfg(feature = "julia")]
#[test]
#[serial]
fn resolve_plugin_artifact_resolves_julia_deployment_payload()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:8088"
route = "/rerank"
health_route = "/healthz"
schema_version = "v1"
timeout_secs = 15
service_mode = "stream"
"#,
    )?;
    set_link_graph_wendao_config_override(&config_path.to_string_lossy());

    let selector = linked_builtin_julia_deployment_artifact_selector();
    let Some(artifact) = super::resolve_link_graph_plugin_artifact_for_selector(&selector) else {
        panic!("artifact");
    };
    assert_eq!(artifact.plugin_id, selector.plugin_id);
    assert_eq!(artifact.artifact_id, selector.artifact_id);
    assert_eq!(artifact.artifact_schema_version.0, "v1");
    assert_eq!(
        artifact
            .endpoint
            .as_ref()
            .and_then(|endpoint| endpoint.base_url.as_deref()),
        Some("http://127.0.0.1:8088")
    );
    assert_eq!(
        artifact.selected_transport,
        Some(xiuxian_wendao_core::transport::PluginTransportKind::ArrowFlight)
    );
    assert_eq!(artifact.fallback_from, None);
    assert_eq!(artifact.fallback_reason, None);

    Ok(())
}

#[cfg(feature = "julia")]
#[test]
#[serial]
fn render_plugin_artifact_toml_renders_julia_deployment_payload()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:8088"
route = "/rerank"
schema_version = "v1"
"#,
    )?;
    set_link_graph_wendao_config_override(&config_path.to_string_lossy());

    let Some(rendered) = super::render_link_graph_plugin_artifact_toml_for_selector(
        &linked_builtin_julia_deployment_artifact_selector(),
    )?
    else {
        panic!("rendered artifact");
    };
    assert!(rendered.contains("plugin_id = \"xiuxian-wendao-julia\""));
    assert!(rendered.contains("artifact_id = \"deployment\""));
    assert!(rendered.contains("route = \"/rerank\""));
    assert!(rendered.contains("selected_transport = \"arrow_flight\""));

    Ok(())
}
