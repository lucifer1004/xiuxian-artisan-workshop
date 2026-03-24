use serde::{Deserialize, Serialize};
use specta::Type;

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
