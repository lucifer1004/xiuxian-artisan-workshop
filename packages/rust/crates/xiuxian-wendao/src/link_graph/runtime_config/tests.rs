use super::{
    export_link_graph_julia_deployment_artifact_toml, resolve_link_graph_coactivation_runtime,
    resolve_link_graph_julia_deployment_artifact, resolve_link_graph_retrieval_policy_runtime,
};
use crate::link_graph::runtime_config::constants::{
    DEFAULT_LINK_GRAPH_COACTIVATION_HOP_DECAY_SCALE, DEFAULT_LINK_GRAPH_COACTIVATION_MAX_HOPS,
    DEFAULT_LINK_GRAPH_COACTIVATION_MAX_NEIGHBORS_PER_DIRECTION,
    DEFAULT_LINK_GRAPH_COACTIVATION_TOUCH_QUEUE_DEPTH,
    DEFAULT_LINK_GRAPH_JULIA_DEPLOYMENT_ARTIFACT_SCHEMA_VERSION,
};
use crate::link_graph::runtime_config::models::LinkGraphSemanticIgnitionBackend;
use crate::link_graph::set_link_graph_wendao_config_override;
use chrono::DateTime;
use std::fs;

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
    assert!(runtime.julia_rerank.base_url.is_none());

    Ok(())
}

#[test]
fn test_retrieval_runtime_resolves_julia_rerank_config() -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let config_path = temp.path().join("wendao.toml");
    fs::write(
        &config_path,
        r#"[link_graph.retrieval]
mode = "hybrid"

[link_graph.retrieval.julia_rerank]
base_url = "http://127.0.0.1:8088"
route = "/arrow-ipc"
health_route = "/healthz"
schema_version = "v1"
timeout_secs = 15
service_mode = "stream"
analyzer_config_path = ".data/WendaoAnalyzer/config/analyzer.example.toml"
analyzer_strategy = "similarity_only"
vector_weight = 0.2
similarity_weight = 0.8
"#,
    )?;
    let config_path_string = config_path.to_string_lossy().to_string();
    set_link_graph_wendao_config_override(&config_path_string);

    let runtime = resolve_link_graph_retrieval_policy_runtime();
    assert_eq!(
        runtime.julia_rerank.base_url.as_deref(),
        Some("http://127.0.0.1:8088")
    );
    assert_eq!(runtime.julia_rerank.route.as_deref(), Some("/arrow-ipc"));
    assert_eq!(
        runtime.julia_rerank.health_route.as_deref(),
        Some("/healthz")
    );
    assert_eq!(runtime.julia_rerank.schema_version.as_deref(), Some("v1"));
    assert_eq!(runtime.julia_rerank.timeout_secs, Some(15));
    assert_eq!(runtime.julia_rerank.service_mode.as_deref(), Some("stream"));
    assert_eq!(
        runtime.julia_rerank.analyzer_config_path.as_deref(),
        Some(".data/WendaoAnalyzer/config/analyzer.example.toml")
    );
    assert_eq!(
        runtime.julia_rerank.analyzer_strategy.as_deref(),
        Some("similarity_only")
    );
    assert_eq!(runtime.julia_rerank.vector_weight, Some(0.2));
    assert_eq!(runtime.julia_rerank.similarity_weight, Some(0.8));

    let descriptor = runtime.julia_rerank.analyzer_service_descriptor();
    assert_eq!(descriptor.service_mode.as_deref(), Some("stream"));
    assert_eq!(
        descriptor.analyzer_config_path.as_deref(),
        Some(".data/WendaoAnalyzer/config/analyzer.example.toml")
    );
    assert_eq!(
        descriptor.analyzer_strategy.as_deref(),
        Some("similarity_only")
    );
    assert_eq!(descriptor.vector_weight, Some(0.2));
    assert_eq!(descriptor.similarity_weight, Some(0.8));

    let manifest = runtime.julia_rerank.analyzer_launch_manifest();
    assert_eq!(
        manifest.launcher_path,
        ".data/WendaoAnalyzer/scripts/run_analyzer_service.sh"
    );
    assert_eq!(
        manifest.args,
        vec![
            "--service-mode",
            "stream",
            "--analyzer-config",
            ".data/WendaoAnalyzer/config/analyzer.example.toml",
            "--analyzer-strategy",
            "similarity_only",
            "--vector-weight",
            "0.2",
            "--similarity-weight",
            "0.8",
        ]
    );

    let artifact = runtime.julia_rerank.deployment_artifact();
    assert_eq!(
        artifact.artifact_schema_version,
        DEFAULT_LINK_GRAPH_JULIA_DEPLOYMENT_ARTIFACT_SCHEMA_VERSION
    );
    DateTime::parse_from_rfc3339(&artifact.generated_at)?;
    assert_eq!(artifact.base_url.as_deref(), Some("http://127.0.0.1:8088"));
    assert_eq!(artifact.route.as_deref(), Some("/arrow-ipc"));
    assert_eq!(artifact.health_route.as_deref(), Some("/healthz"));
    assert_eq!(artifact.schema_version.as_deref(), Some("v1"));
    assert_eq!(artifact.timeout_secs, Some(15));
    assert_eq!(artifact.launch, manifest);

    let encoded = toml::to_string_pretty(&artifact)?;
    assert!(encoded.contains("launcher_path"));
    assert!(encoded.contains("base_url = \"http://127.0.0.1:8088\""));
    assert_eq!(artifact.to_toml_string()?, encoded);
    let resolved_artifact = resolve_link_graph_julia_deployment_artifact();
    assert_eq!(
        resolved_artifact.artifact_schema_version,
        DEFAULT_LINK_GRAPH_JULIA_DEPLOYMENT_ARTIFACT_SCHEMA_VERSION
    );
    DateTime::parse_from_rfc3339(&resolved_artifact.generated_at)?;
    assert_eq!(resolved_artifact.base_url, artifact.base_url);
    assert_eq!(resolved_artifact.route, artifact.route);
    assert_eq!(resolved_artifact.health_route, artifact.health_route);
    assert_eq!(resolved_artifact.schema_version, artifact.schema_version);
    assert_eq!(resolved_artifact.timeout_secs, artifact.timeout_secs);
    assert_eq!(resolved_artifact.launch, artifact.launch);

    let exported = export_link_graph_julia_deployment_artifact_toml()?;
    assert!(exported.contains("artifact_schema_version = \"v1\""));
    assert!(exported.contains("generated_at = "));

    Ok(())
}

#[test]
fn test_julia_deployment_artifact_writes_toml_file() -> Result<(), Box<dyn std::error::Error>> {
    let artifact = super::models::LinkGraphJuliaDeploymentArtifact {
        artifact_schema_version: DEFAULT_LINK_GRAPH_JULIA_DEPLOYMENT_ARTIFACT_SCHEMA_VERSION
            .to_string(),
        generated_at: "2026-03-27T16:00:00+00:00".to_string(),
        base_url: Some("http://127.0.0.1:18080".to_string()),
        route: Some("/arrow-ipc".to_string()),
        health_route: Some("/health".to_string()),
        schema_version: Some("v1".to_string()),
        timeout_secs: Some(15),
        launch: super::models::LinkGraphJuliaAnalyzerLaunchManifest {
            launcher_path: ".data/WendaoAnalyzer/scripts/run_analyzer_service.sh".to_string(),
            args: vec![
                "--service-mode".to_string(),
                "stream".to_string(),
                "--analyzer-strategy".to_string(),
                "similarity_only".to_string(),
            ],
        },
    };

    let temp = tempfile::tempdir()?;
    let artifact_path = temp
        .path()
        .join("nested")
        .join("julia_deployment_artifact.toml");
    artifact.write_toml_file(&artifact_path)?;

    let written = fs::read_to_string(&artifact_path)?;
    assert!(written.contains("artifact_schema_version = \"v1\""));
    assert!(written.contains("generated_at = \"2026-03-27T16:00:00+00:00\""));
    assert!(written.contains("base_url = \"http://127.0.0.1:18080\""));
    assert!(
        written
            .contains("launcher_path = \".data/WendaoAnalyzer/scripts/run_analyzer_service.sh\"")
    );
    assert!(written.contains("\"similarity_only\""));
    assert_eq!(written, artifact.to_toml_string()?);

    Ok(())
}

#[test]
fn test_julia_deployment_artifact_writes_json_file() -> Result<(), Box<dyn std::error::Error>> {
    let artifact = super::models::LinkGraphJuliaDeploymentArtifact {
        artifact_schema_version: DEFAULT_LINK_GRAPH_JULIA_DEPLOYMENT_ARTIFACT_SCHEMA_VERSION
            .to_string(),
        generated_at: "2026-03-27T16:00:00+00:00".to_string(),
        base_url: Some("http://127.0.0.1:18080".to_string()),
        route: Some("/arrow-ipc".to_string()),
        health_route: Some("/health".to_string()),
        schema_version: Some("v1".to_string()),
        timeout_secs: Some(15),
        launch: super::models::LinkGraphJuliaAnalyzerLaunchManifest {
            launcher_path: ".data/WendaoAnalyzer/scripts/run_analyzer_service.sh".to_string(),
            args: vec![
                "--service-mode".to_string(),
                "stream".to_string(),
                "--analyzer-strategy".to_string(),
                "similarity_only".to_string(),
            ],
        },
    };

    let temp = tempfile::tempdir()?;
    let artifact_path = temp
        .path()
        .join("nested")
        .join("julia_deployment_artifact.json");
    artifact.write_json_file(&artifact_path)?;

    let written = fs::read_to_string(&artifact_path)?;
    assert!(written.contains("\"artifact_schema_version\": \"v1\""));
    assert!(written.contains("\"generated_at\": \"2026-03-27T16:00:00+00:00\""));
    assert!(written.contains("\"base_url\": \"http://127.0.0.1:18080\""));
    assert!(
        written.contains(
            "\"launcher_path\": \".data/WendaoAnalyzer/scripts/run_analyzer_service.sh\""
        )
    );
    assert_eq!(written, artifact.to_json_string()?);

    Ok(())
}
