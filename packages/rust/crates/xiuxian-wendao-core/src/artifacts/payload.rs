use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::artifacts::PluginLaunchSpec;
use crate::capabilities::ContractVersion;
use crate::ids::{ArtifactId, PluginId};
use crate::transport::{PluginTransportEndpoint, PluginTransportKind};

/// Generic artifact payload returned by plugin-artifact resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginArtifactPayload {
    /// Owner plugin id.
    pub plugin_id: PluginId,
    /// Artifact kind id.
    pub artifact_id: ArtifactId,
    /// Artifact payload schema version.
    pub artifact_schema_version: ContractVersion,
    /// RFC3339 generation timestamp.
    pub generated_at: String,
    /// Optional runtime endpoint carried by the artifact payload.
    pub endpoint: Option<PluginTransportEndpoint>,
    /// Optional provider schema version carried by the artifact payload.
    pub schema_version: Option<String>,
    /// Optional launch metadata carried by the artifact payload.
    pub launch: Option<PluginLaunchSpec>,
    /// Runtime-selected transport surfaced through outward inspection payloads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_transport: Option<PluginTransportKind>,
    /// Higher-preference transport that was skipped before selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_from: Option<PluginTransportKind>,
    /// Reason the runtime fell back from a higher-preference transport.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
}

impl PluginArtifactPayload {
    /// Render the plugin artifact as pretty TOML.
    ///
    /// # Errors
    ///
    /// Returns an error when the artifact cannot be serialized into TOML.
    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    /// Render the plugin artifact as pretty JSON.
    ///
    /// # Errors
    ///
    /// Returns an error when the artifact cannot be serialized into JSON.
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Persist the plugin artifact to a TOML file.
    ///
    /// # Errors
    ///
    /// Returns an error when serialization fails, parent directories cannot be
    /// created, or the file cannot be written.
    pub fn write_toml_file<P>(&self, path: P) -> std::io::Result<()>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }

        let encoded = self.to_toml_string().map_err(std::io::Error::other)?;
        std::fs::write(path, encoded)
    }

    /// Persist the plugin artifact to a JSON file.
    ///
    /// # Errors
    ///
    /// Returns an error when serialization fails, parent directories cannot be
    /// created, or the file cannot be written.
    pub fn write_json_file<P>(&self, path: P) -> std::io::Result<()>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }

        let encoded = self.to_json_string().map_err(std::io::Error::other)?;
        std::fs::write(path, encoded)
    }
}

#[cfg(test)]
mod tests {
    use super::PluginArtifactPayload;
    use crate::artifacts::PluginLaunchSpec;
    use crate::capabilities::ContractVersion;
    use crate::ids::{ArtifactId, PluginId};
    use crate::transport::{PluginTransportEndpoint, PluginTransportKind};

    #[test]
    fn artifact_payload_serializes_to_toml_and_json() {
        let payload = PluginArtifactPayload {
            plugin_id: PluginId("wendao-julia".to_string()),
            artifact_id: ArtifactId("deployment".to_string()),
            artifact_schema_version: ContractVersion("v1".to_string()),
            generated_at: "2026-03-28T12:00:00Z".to_string(),
            endpoint: Some(PluginTransportEndpoint {
                base_url: Some("http://127.0.0.1:8080".to_string()),
                route: Some("/arrow-ipc".to_string()),
                health_route: Some("/health".to_string()),
                timeout_secs: Some(30),
            }),
            schema_version: Some("v1".to_string()),
            launch: Some(PluginLaunchSpec {
                launcher_path: ".data/WendaoAnalyzer/scripts/run.sh".to_string(),
                args: vec!["--stdio".to_string()],
            }),
            selected_transport: Some(PluginTransportKind::ArrowIpcHttp),
            fallback_from: Some(PluginTransportKind::ArrowFlight),
            fallback_reason: Some(
                "preferred transport ArrowFlight is unavailable because the binding has no base_url"
                    .to_string(),
            ),
        };

        let toml = payload
            .to_toml_string()
            .expect("payload should serialize to TOML");
        let json = payload
            .to_json_string()
            .expect("payload should serialize to JSON");

        assert!(toml.contains("wendao-julia"));
        assert!(toml.contains("selected_transport = \"arrow_ipc_http\""));
        assert!(json.contains("\"deployment\""));
        assert!(json.contains("\"fallback_reason\""));
    }
}
