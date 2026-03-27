use serde::{Deserialize, Serialize};
use specta::Type;

use crate::{LinkGraphJuliaAnalyzerLaunchManifest, LinkGraphJuliaDeploymentArtifact};

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
}

/// Studio-visible Julia analyzer launch manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct UiJuliaAnalyzerLaunchManifest {
    /// Launcher path relative to the repository root.
    pub launcher_path: String,
    /// Ordered analyzer-owned CLI args.
    pub args: Vec<String>,
}

impl From<LinkGraphJuliaAnalyzerLaunchManifest> for UiJuliaAnalyzerLaunchManifest {
    fn from(value: LinkGraphJuliaAnalyzerLaunchManifest) -> Self {
        Self {
            launcher_path: value.launcher_path,
            args: value.args,
        }
    }
}

/// Studio-visible Julia deployment artifact inspection payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct UiJuliaDeploymentArtifact {
    /// Artifact-level schema version for deployment inspection surfaces.
    pub artifact_schema_version: String,
    /// RFC3339 timestamp recording when the deployment artifact was rendered.
    pub generated_at: String,
    /// Resolved Julia service base URL.
    pub base_url: Option<String>,
    /// Arrow IPC route expected by the service.
    pub route: Option<String>,
    /// Health-check route expected by the service.
    pub health_route: Option<String>,
    /// WendaoArrow schema version expected by Rust.
    pub schema_version: Option<String>,
    /// Optional request timeout in seconds.
    pub timeout_secs: Option<u64>,
    /// Resolved analyzer launch manifest.
    pub launch: UiJuliaAnalyzerLaunchManifest,
}

impl From<LinkGraphJuliaDeploymentArtifact> for UiJuliaDeploymentArtifact {
    fn from(value: LinkGraphJuliaDeploymentArtifact) -> Self {
        Self {
            artifact_schema_version: value.artifact_schema_version,
            generated_at: value.generated_at,
            base_url: value.base_url,
            route: value.route,
            health_route: value.health_route,
            schema_version: value.schema_version,
            timeout_secs: value.timeout_secs,
            launch: value.launch.into(),
        }
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
