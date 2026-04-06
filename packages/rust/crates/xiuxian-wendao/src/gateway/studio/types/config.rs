use serde::{Deserialize, Serialize};
use specta::Type;
use xiuxian_wendao_core::{
    artifacts::{PluginArtifactPayload, PluginLaunchSpec},
    transport::PluginTransportKind,
};

/// Global UI configuration for Studio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct UiConfig {
    /// Local project roots to scan.
    pub projects: Vec<UiProjectConfig>,
    /// External repository projects.
    pub repo_projects: Vec<UiRepoProjectConfig>,
}

/// Gateway-reported studio capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct UiCapabilities {
    /// Supported language identifiers reported by the gateway plugin registry.
    #[serde(rename = "supportedLanguages")]
    pub languages: Vec<String>,
    /// Supported repository identifiers reported by the gateway UI config.
    #[serde(rename = "supportedRepositories")]
    pub repositories: Vec<String>,
    /// Supported code filter kinds reported by the gateway capability surface.
    #[serde(rename = "supportedKinds")]
    pub kinds: Vec<String>,
    /// Whether bootstrap-time background indexing is enabled during gateway startup.
    pub studio_bootstrap_background_indexing_enabled: bool,
    /// Stable mode label for bootstrap-time background indexing during gateway startup.
    pub studio_bootstrap_background_indexing_mode: String,
    /// Whether deferred bootstrap indexing has been lazily activated since process boot.
    pub studio_bootstrap_background_indexing_deferred_activation_observed: bool,
}

/// Studio-visible generic plugin launch manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct UiPluginLaunchSpec {
    /// Launcher path relative to the repository root.
    pub launcher_path: String,
    /// Ordered provider-owned CLI args.
    pub args: Vec<String>,
}

impl From<PluginLaunchSpec> for UiPluginLaunchSpec {
    fn from(value: PluginLaunchSpec) -> Self {
        Self {
            launcher_path: value.launcher_path,
            args: value.args,
        }
    }
}

/// Studio-visible generic plugin transport kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum UiPluginTransportKind {
    ArrowFlight,
}

impl From<PluginTransportKind> for UiPluginTransportKind {
    fn from(value: PluginTransportKind) -> Self {
        match value {
            PluginTransportKind::ArrowFlight => Self::ArrowFlight,
        }
    }
}

/// Studio-visible generic plugin artifact inspection payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct UiPluginArtifact {
    /// Owner plugin id.
    pub plugin_id: String,
    /// Artifact kind id.
    pub artifact_id: String,
    /// Artifact-level schema version for inspection surfaces.
    pub artifact_schema_version: String,
    /// RFC3339 timestamp recording when the artifact was rendered.
    pub generated_at: String,
    /// Resolved provider service base URL.
    pub base_url: Option<String>,
    /// Request route expected by the provider.
    pub route: Option<String>,
    /// Health-check route expected by the provider.
    pub health_route: Option<String>,
    /// Optional request timeout in seconds.
    pub timeout_secs: Option<u64>,
    /// Optional provider schema version.
    pub schema_version: Option<String>,
    /// Optional launch manifest for managed providers.
    pub launch: Option<UiPluginLaunchSpec>,
    /// Runtime-selected transport surfaced by the current negotiation seam.
    pub selected_transport: Option<UiPluginTransportKind>,
    /// Higher-preference transport skipped before selection.
    pub fallback_from: Option<UiPluginTransportKind>,
    /// Reason the runtime fell back from a higher-preference transport.
    pub fallback_reason: Option<String>,
}

impl From<PluginArtifactPayload> for UiPluginArtifact {
    fn from(value: PluginArtifactPayload) -> Self {
        let endpoint = value.endpoint;
        Self {
            plugin_id: value.plugin_id.0,
            artifact_id: value.artifact_id.0,
            artifact_schema_version: value.artifact_schema_version.0,
            generated_at: value.generated_at,
            base_url: endpoint
                .as_ref()
                .and_then(|endpoint| endpoint.base_url.clone()),
            route: endpoint
                .as_ref()
                .and_then(|endpoint| endpoint.route.clone()),
            health_route: endpoint
                .as_ref()
                .and_then(|endpoint| endpoint.health_route.clone()),
            timeout_secs: endpoint.as_ref().and_then(|endpoint| endpoint.timeout_secs),
            schema_version: value.schema_version,
            launch: value.launch.map(Into::into),
            selected_transport: value.selected_transport.map(Into::into),
            fallback_from: value.fallback_from.map(Into::into),
            fallback_reason: value.fallback_reason,
        }
    }
}

#[cfg(all(test, feature = "julia"))]
mod tests {
    use super::{UiPluginArtifact, UiPluginLaunchSpec};
    use xiuxian_wendao_builtin::linked_builtin_julia_gateway_artifact_ui_payload_fixture;

    #[test]
    fn generic_ui_artifact_builds_from_plugin_artifact_payload() {
        let payload = linked_builtin_julia_gateway_artifact_ui_payload_fixture();
        let expected_endpoint = payload
            .endpoint
            .clone()
            .unwrap_or_else(|| panic!("fixture should include endpoint"));
        let expected_launch = payload
            .launch
            .clone()
            .unwrap_or_else(|| panic!("fixture should include launch spec"));
        let expected_plugin_id = payload.plugin_id.0.clone();
        let expected_artifact_id = payload.artifact_id.0.clone();
        let expected_artifact_schema_version = payload.artifact_schema_version.0.clone();
        let expected_generated_at = payload.generated_at.clone();
        let expected_schema_version = payload.schema_version.clone();
        let expected_selected_transport = payload.selected_transport;
        let expected_fallback_from = payload.fallback_from;
        let expected_fallback_reason = payload.fallback_reason.clone();

        assert_eq!(
            UiPluginArtifact::from(payload),
            UiPluginArtifact {
                plugin_id: expected_plugin_id,
                artifact_id: expected_artifact_id,
                artifact_schema_version: expected_artifact_schema_version,
                generated_at: expected_generated_at,
                base_url: expected_endpoint.base_url,
                route: expected_endpoint.route,
                health_route: expected_endpoint.health_route,
                timeout_secs: expected_endpoint.timeout_secs,
                schema_version: expected_schema_version,
                launch: Some(UiPluginLaunchSpec {
                    launcher_path: expected_launch.launcher_path,
                    args: expected_launch.args,
                }),
                selected_transport: expected_selected_transport.map(Into::into),
                fallback_from: expected_fallback_from.map(Into::into),
                fallback_reason: expected_fallback_reason,
            }
        );
    }
}

/// Configuration for a local project root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UiProjectConfig {
    /// Unique name.
    pub name: String,
    /// Relative path to project root.
    pub root: String,
    /// Explicit subdirectories to index.
    pub dirs: Vec<String>,
}

/// Configuration for an external analyzed repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UiRepoProjectConfig {
    /// Unique identifier.
    pub id: String,
    /// Optional local path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// Optional upstream URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Optional git reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
    /// Refresh policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh: Option<String>,
    /// Enabled analysis plugins.
    pub plugins: Vec<String>,
}
