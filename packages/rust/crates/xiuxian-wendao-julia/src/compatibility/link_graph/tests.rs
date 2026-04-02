use xiuxian_wendao_core::{
    artifacts::{PluginArtifactPayload, PluginLaunchSpec},
    transport::PluginTransportEndpoint,
};

use super::{
    DEFAULT_JULIA_ANALYZER_EXAMPLE_CONFIG_PATH, DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH,
    DEFAULT_JULIA_DEPLOYMENT_ARTIFACT_SCHEMA_VERSION, DEFAULT_JULIA_RERANK_FLIGHT_ROUTE,
    JULIA_DEPLOYMENT_ARTIFACT_ID, JULIA_GRAPH_STRUCTURAL_CAPABILITY_ID, JULIA_PLUGIN_ID,
    JULIA_RERANK_CAPABILITY_ID,
    LinkGraphJuliaAnalyzerLaunchManifest, LinkGraphJuliaAnalyzerServiceDescriptor,
    LinkGraphJuliaDeploymentArtifact, LinkGraphJuliaRerankRuntimeConfig,
    build_rerank_provider_binding, julia_deployment_artifact_selector,
    julia_graph_structural_provider_selector, julia_rerank_provider_selector,
};

#[test]
fn selectors_keep_stable_julia_ids() {
    let provider = julia_rerank_provider_selector();
    let graph_structural = julia_graph_structural_provider_selector();
    let artifact = julia_deployment_artifact_selector();

    assert_eq!(provider.provider.0, JULIA_PLUGIN_ID);
    assert_eq!(provider.capability_id.0, JULIA_RERANK_CAPABILITY_ID);
    assert_eq!(graph_structural.provider.0, JULIA_PLUGIN_ID);
    assert_eq!(
        graph_structural.capability_id.0,
        JULIA_GRAPH_STRUCTURAL_CAPABILITY_ID
    );
    assert_eq!(artifact.plugin_id.0, JULIA_PLUGIN_ID);
    assert_eq!(artifact.artifact_id.0, JULIA_DEPLOYMENT_ARTIFACT_ID);
}

#[test]
fn launch_manifest_round_trips_plugin_launch_spec() {
    let launch = LinkGraphJuliaAnalyzerLaunchManifest {
        launcher_path: DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH.to_string(),
        args: vec!["--service-mode".to_string(), "stream".to_string()],
    };

    let spec: PluginLaunchSpec = launch.clone().into();
    let roundtrip = LinkGraphJuliaAnalyzerLaunchManifest::from(spec);

    assert_eq!(roundtrip, launch);
}

#[test]
fn service_descriptor_builds_plugin_launch_spec() {
    let descriptor = LinkGraphJuliaAnalyzerServiceDescriptor {
        service_mode: Some("stream".to_string()),
        analyzer_config_path: Some(DEFAULT_JULIA_ANALYZER_EXAMPLE_CONFIG_PATH.to_string()),
        analyzer_strategy: Some("similarity_only".to_string()),
        vector_weight: Some(0.2),
        similarity_weight: Some(0.8),
    };

    let spec = descriptor.plugin_launch_spec(DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH);

    assert_eq!(spec.launcher_path, DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH);
    assert_eq!(
        spec.args,
        vec![
            "--service-mode",
            "stream",
            "--analyzer-config",
            DEFAULT_JULIA_ANALYZER_EXAMPLE_CONFIG_PATH,
            "--analyzer-strategy",
            "similarity_only",
            "--vector-weight",
            "0.2",
            "--similarity-weight",
            "0.8",
        ]
    );
}

#[test]
fn deployment_artifact_round_trips_plugin_artifact_payload() {
    let artifact = LinkGraphJuliaDeploymentArtifact {
        artifact_schema_version: "v1".to_string(),
        generated_at: "2026-03-28T12:34:56Z".to_string(),
        base_url: Some("http://127.0.0.1:8080".to_string()),
        route: Some(DEFAULT_JULIA_RERANK_FLIGHT_ROUTE.to_string()),
        health_route: Some("/health".to_string()),
        schema_version: Some("v1".to_string()),
        timeout_secs: Some(15),
        launch: LinkGraphJuliaAnalyzerLaunchManifest {
            launcher_path: DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH.to_string(),
            args: vec!["--service-mode".to_string(), "stream".to_string()],
        },
    };

    let payload: PluginArtifactPayload = artifact.clone().into();

    assert_eq!(payload.plugin_id.0, JULIA_PLUGIN_ID);
    assert_eq!(payload.artifact_id.0, JULIA_DEPLOYMENT_ARTIFACT_ID);
    assert_eq!(
        payload.launch.unwrap().launcher_path,
        artifact.launch.launcher_path
    );

    let roundtrip = LinkGraphJuliaDeploymentArtifact::from(PluginArtifactPayload {
        plugin_id: payload.plugin_id,
        artifact_id: payload.artifact_id,
        artifact_schema_version: payload.artifact_schema_version,
        generated_at: payload.generated_at,
        endpoint: Some(PluginTransportEndpoint {
            base_url: Some("http://127.0.0.1:8080".to_string()),
            route: Some(DEFAULT_JULIA_RERANK_FLIGHT_ROUTE.to_string()),
            health_route: Some("/health".to_string()),
            timeout_secs: Some(15),
        }),
        schema_version: Some("v1".to_string()),
        launch: Some(PluginLaunchSpec {
            launcher_path: DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH.to_string(),
            args: vec!["--service-mode".to_string(), "stream".to_string()],
        }),
        selected_transport: None,
        fallback_from: None,
        fallback_reason: None,
    });

    assert_eq!(roundtrip.artifact_schema_version, "v1");
    assert_eq!(roundtrip.schema_version.as_deref(), Some("v1"));
    assert_eq!(roundtrip.timeout_secs, Some(15));
}

#[test]
fn runtime_config_builds_provider_binding_and_artifact_payload() {
    let runtime = LinkGraphJuliaRerankRuntimeConfig {
        base_url: Some("http://127.0.0.1:8088".to_string()),
        route: Some(DEFAULT_JULIA_RERANK_FLIGHT_ROUTE.to_string()),
        health_route: Some("/healthz".to_string()),
        schema_version: Some("v1".to_string()),
        timeout_secs: Some(15),
        service_mode: Some("stream".to_string()),
        analyzer_config_path: Some(DEFAULT_JULIA_ANALYZER_EXAMPLE_CONFIG_PATH.to_string()),
        analyzer_strategy: Some("similarity_only".to_string()),
        vector_weight: Some(0.2),
        similarity_weight: Some(0.8),
    };

    let binding = build_rerank_provider_binding(&runtime);
    assert_eq!(binding.selector, julia_rerank_provider_selector());
    assert_eq!(
        binding.endpoint.base_url.as_deref(),
        Some("http://127.0.0.1:8088")
    );
    assert_eq!(
        binding.transport,
        xiuxian_wendao_core::transport::PluginTransportKind::ArrowFlight
    );
    assert_eq!(
        binding.launch.expect("launch").launcher_path,
        DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH
    );

    let artifact = runtime.deployment_artifact();
    assert_eq!(
        artifact.artifact_schema_version,
        DEFAULT_JULIA_DEPLOYMENT_ARTIFACT_SCHEMA_VERSION
    );
    assert_eq!(artifact.base_url.as_deref(), Some("http://127.0.0.1:8088"));
    assert_eq!(
        artifact.route.as_deref(),
        Some(DEFAULT_JULIA_RERANK_FLIGHT_ROUTE)
    );
    assert_eq!(artifact.health_route.as_deref(), Some("/healthz"));
}
