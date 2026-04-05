use crate::compatibility::link_graph::{
    DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH, DEFAULT_JULIA_RERANK_FLIGHT_ROUTE,
    JULIA_DEPLOYMENT_ARTIFACT_ID, JULIA_PLUGIN_ID,
};
use serde_json::{Value, json};
use xiuxian_wendao_core::artifacts::PluginLaunchSpec;
use xiuxian_wendao_core::{
    artifacts::PluginArtifactPayload,
    capabilities::ContractVersion,
    ids::{ArtifactId, PluginId},
    transport::{PluginTransportEndpoint, PluginTransportKind},
};

const JULIA_GATEWAY_ARTIFACT_BASE_URL: &str = "http://127.0.0.1:18080";
const JULIA_GATEWAY_ARTIFACT_SCHEMA_VERSION: &str = "v1";
const JULIA_GATEWAY_ARTIFACT_SERVICE_MODE: &str = "stream";
const JULIA_GATEWAY_ARTIFACT_SELECTED_TRANSPORT: &str = "arrow_flight";
const JULIA_GATEWAY_ARTIFACT_DEFAULT_STRATEGY: &str = "similarity_only";
const JULIA_UI_ARTIFACT_BASE_URL: &str = "http://127.0.0.1:8088";
const JULIA_UI_ARTIFACT_GENERATED_AT: &str = "2026-03-27T12:00:00Z";
const JULIA_UI_ARTIFACT_HEALTH_ROUTE: &str = "/healthz";
const JULIA_UI_ARTIFACT_TIMEOUT_SECS: u64 = 15;

/// Return the stable base URL used by Studio gateway artifact fixtures.
#[must_use]
pub fn julia_gateway_artifact_base_url() -> &'static str {
    JULIA_GATEWAY_ARTIFACT_BASE_URL
}

/// Return the stable schema version used by Studio gateway artifact fixtures.
#[must_use]
pub fn julia_gateway_artifact_schema_version() -> &'static str {
    JULIA_GATEWAY_ARTIFACT_SCHEMA_VERSION
}

/// Return the stable selected transport used by Studio gateway artifact
/// fixtures.
#[must_use]
pub fn julia_gateway_artifact_selected_transport() -> &'static str {
    JULIA_GATEWAY_ARTIFACT_SELECTED_TRANSPORT
}

/// Return the stable analyzer strategy used by generic plugin-artifact fixture
/// tests.
#[must_use]
pub fn julia_gateway_artifact_default_strategy() -> &'static str {
    JULIA_GATEWAY_ARTIFACT_DEFAULT_STRATEGY
}

/// Return the stable path identifiers for the Julia deployment artifact.
#[must_use]
pub fn julia_gateway_artifact_path() -> (String, String) {
    (
        JULIA_PLUGIN_ID.to_string(),
        JULIA_DEPLOYMENT_ARTIFACT_ID.to_string(),
    )
}

/// Build the JSON-RPC params fixture used by generic Julia plugin-artifact
/// export tests.
#[must_use]
pub fn julia_gateway_artifact_rpc_params_fixture(
    output_format: Option<&str>,
    output_path: Option<&str>,
) -> Value {
    let (plugin_id, artifact_id) = julia_gateway_artifact_path();
    let mut request = json!({
        "plugin_id": plugin_id,
        "artifact_id": artifact_id,
    });

    if let Some(format) = output_format {
        request["output_format"] = Value::String(format.to_string());
    }

    if let Some(path) = output_path {
        request["output_path"] = Value::String(path.to_string());
    }

    request
}

/// Render the runtime config TOML fixture used by Studio gateway artifact
/// handler tests.
#[must_use]
pub fn julia_gateway_artifact_runtime_config_toml(analyzer_strategy: Option<&str>) -> String {
    let mut rendered = format!(
        "[link_graph.retrieval.julia_rerank]\nbase_url = \"{JULIA_GATEWAY_ARTIFACT_BASE_URL}\"\nroute = \"{DEFAULT_JULIA_RERANK_FLIGHT_ROUTE}\"\nschema_version = \"{JULIA_GATEWAY_ARTIFACT_SCHEMA_VERSION}\"\nservice_mode = \"{JULIA_GATEWAY_ARTIFACT_SERVICE_MODE}\"\n"
    );

    if let Some(strategy) = analyzer_strategy {
        rendered.push_str(format!("analyzer_strategy = \"{strategy}\"\n").as_str());
    }

    rendered
}

/// Return the stable TOML fragments that Studio gateway artifact handler tests
/// should observe in the rendered generic plugin artifact output.
#[must_use]
pub fn julia_gateway_artifact_expected_toml_fragments() -> Vec<String> {
    vec![
        format!("artifact_schema_version = \"{JULIA_GATEWAY_ARTIFACT_SCHEMA_VERSION}\""),
        format!("base_url = \"{JULIA_GATEWAY_ARTIFACT_BASE_URL}\""),
        format!("route = \"{DEFAULT_JULIA_RERANK_FLIGHT_ROUTE}\""),
        format!("launcher_path = \"{DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH}\""),
        format!("selected_transport = \"{JULIA_GATEWAY_ARTIFACT_SELECTED_TRANSPORT}\""),
    ]
}

/// Return the stable JSON fragments that generic plugin-artifact tests should
/// observe in rendered Julia deployment artifact output.
#[must_use]
pub fn julia_gateway_artifact_expected_json_fragments() -> Vec<String> {
    vec![
        format!("\"artifact_schema_version\": \"{JULIA_GATEWAY_ARTIFACT_SCHEMA_VERSION}\""),
        format!("\"base_url\": \"{JULIA_GATEWAY_ARTIFACT_BASE_URL}\""),
        format!("\"route\": \"{DEFAULT_JULIA_RERANK_FLIGHT_ROUTE}\""),
        format!("\"launcher_path\": \"{DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH}\""),
    ]
}

/// Build the Julia `PluginArtifactPayload` fixture used by Studio UI artifact
/// mapping tests.
#[must_use]
pub fn julia_ui_artifact_payload_fixture() -> PluginArtifactPayload {
    let (plugin_id, artifact_id) = julia_gateway_artifact_path();

    PluginArtifactPayload {
        plugin_id: PluginId(plugin_id),
        artifact_id: ArtifactId(artifact_id),
        artifact_schema_version: ContractVersion(JULIA_GATEWAY_ARTIFACT_SCHEMA_VERSION.to_string()),
        generated_at: JULIA_UI_ARTIFACT_GENERATED_AT.to_string(),
        endpoint: Some(PluginTransportEndpoint {
            base_url: Some(JULIA_UI_ARTIFACT_BASE_URL.to_string()),
            route: Some(DEFAULT_JULIA_RERANK_FLIGHT_ROUTE.to_string()),
            health_route: Some(JULIA_UI_ARTIFACT_HEALTH_ROUTE.to_string()),
            timeout_secs: Some(JULIA_UI_ARTIFACT_TIMEOUT_SECS),
        }),
        schema_version: Some(JULIA_GATEWAY_ARTIFACT_SCHEMA_VERSION.to_string()),
        launch: Some(PluginLaunchSpec {
            launcher_path: DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH.to_string(),
            args: vec![
                "--service-mode".to_string(),
                JULIA_GATEWAY_ARTIFACT_SERVICE_MODE.to_string(),
            ],
        }),
        selected_transport: Some(PluginTransportKind::ArrowFlight),
        fallback_from: None,
        fallback_reason: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH, julia_gateway_artifact_default_strategy,
        julia_gateway_artifact_expected_json_fragments,
        julia_gateway_artifact_expected_toml_fragments, julia_gateway_artifact_path,
        julia_gateway_artifact_rpc_params_fixture, julia_gateway_artifact_runtime_config_toml,
        julia_gateway_artifact_selected_transport, julia_ui_artifact_payload_fixture,
    };
    use xiuxian_wendao_core::transport::PluginTransportKind;

    #[test]
    fn gateway_artifact_runtime_config_toml_renders_default_fixture() {
        let rendered = julia_gateway_artifact_runtime_config_toml(None);

        assert!(rendered.contains("[link_graph.retrieval.julia_rerank]"));
        assert!(rendered.contains("base_url = \"http://127.0.0.1:18080\""));
        assert!(!rendered.contains("analyzer_strategy ="));
    }

    #[test]
    fn gateway_artifact_runtime_config_toml_renders_optional_strategy() {
        let rendered = julia_gateway_artifact_runtime_config_toml(Some("similarity_only"));

        assert!(rendered.contains("analyzer_strategy = \"similarity_only\""));
    }

    #[test]
    fn gateway_artifact_helpers_keep_stable_selector_and_toml_fragments() {
        let (plugin_id, artifact_id) = julia_gateway_artifact_path();
        let fragments = julia_gateway_artifact_expected_toml_fragments();

        assert_eq!(plugin_id, "xiuxian-wendao-julia");
        assert_eq!(artifact_id, "deployment");
        assert!(
            fragments
                .iter()
                .any(|fragment| fragment.contains(julia_gateway_artifact_selected_transport()))
        );
    }

    #[test]
    fn gateway_artifact_helpers_keep_stable_json_fragments_and_strategy() {
        let fragments = julia_gateway_artifact_expected_json_fragments();

        assert_eq!(julia_gateway_artifact_default_strategy(), "similarity_only");
        assert!(
            fragments
                .iter()
                .any(|fragment| fragment.contains("\"artifact_schema_version\": \"v1\""))
        );
        assert!(
            fragments
                .iter()
                .any(|fragment| fragment.contains("\"route\": \"/rerank\""))
        );
    }

    #[test]
    fn gateway_artifact_rpc_params_fixture_keeps_stable_shape() {
        let request = julia_gateway_artifact_rpc_params_fixture(
            Some("json"),
            Some(".run/julia/plugin-artifact.json"),
        );

        assert_eq!(request["plugin_id"].as_str(), Some("xiuxian-wendao-julia"));
        assert_eq!(request["artifact_id"].as_str(), Some("deployment"));
        assert_eq!(request["output_format"].as_str(), Some("json"));
        assert_eq!(
            request["output_path"].as_str(),
            Some(".run/julia/plugin-artifact.json")
        );
    }

    #[test]
    fn ui_artifact_payload_fixture_keeps_stable_julia_payload_shape() {
        let payload = julia_ui_artifact_payload_fixture();
        let endpoint = payload
            .endpoint
            .as_ref()
            .unwrap_or_else(|| panic!("fixture should include endpoint"));
        let launch = payload
            .launch
            .as_ref()
            .unwrap_or_else(|| panic!("fixture should include launch spec"));

        assert_eq!(payload.plugin_id.0, "xiuxian-wendao-julia");
        assert_eq!(payload.artifact_id.0, "deployment");
        assert_eq!(payload.generated_at, "2026-03-27T12:00:00Z");
        assert_eq!(endpoint.base_url.as_deref(), Some("http://127.0.0.1:8088"));
        assert_eq!(endpoint.health_route.as_deref(), Some("/healthz"));
        assert_eq!(endpoint.timeout_secs, Some(15));
        assert_eq!(
            payload.selected_transport,
            Some(PluginTransportKind::ArrowFlight)
        );
        assert_eq!(
            launch.launcher_path,
            DEFAULT_JULIA_ANALYZER_LAUNCHER_PATH.to_string()
        );
    }
}
