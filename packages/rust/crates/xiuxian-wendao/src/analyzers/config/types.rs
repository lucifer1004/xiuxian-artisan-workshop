use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Full configuration for the Repo Intelligence runtime.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RepoIntelligenceConfig {
    /// List of registered repositories available for analysis.
    pub repos: Vec<RegisteredRepository>,
}

/// One repository registered with the Repo Intelligence runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
pub struct RegisteredRepository {
    /// Stable repository identifier used by CLI, gateway, and query APIs.
    pub id: String,
    /// Local repository checkout path used for the current MVP slice.
    #[serde(default)]
    pub path: Option<PathBuf>,
    /// Upstream git URL.
    #[serde(default)]
    pub url: Option<String>,
    /// Revision policy to materialize locally.
    #[serde(rename = "ref", default)]
    pub git_ref: Option<RepositoryRef>,
    /// Refresh policy for upstream updates.
    #[serde(default)]
    pub refresh: RepositoryRefreshPolicy,
    /// Analysis plugins associated with the repository.
    #[serde(default)]
    pub plugins: Vec<RepositoryPluginConfig>,
}

/// Specific git reference to materialize.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum RepositoryRef {
    /// Track a specific branch.
    Branch(String),
    /// Pin to a specific tag.
    Tag(String),
    /// Pin to a specific commit SHA.
    Commit(String),
}

impl RepositoryRef {
    /// Returns the string representation of the reference.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Branch(s) | Self::Tag(s) | Self::Commit(s) => s.as_str(),
        }
    }
}

/// Policy for repository source refreshing.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RepositoryRefreshPolicy {
    /// Always attempt to fetch upstream updates on every analysis.
    #[default]
    Fetch,
    /// Only perform manual source refreshes.
    Manual,
}

/// Configuration for a repository analysis plugin.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum RepositoryPluginConfig {
    /// Plugin identified by its stable ID.
    Id(String),
    /// Inline plugin configuration (not implemented).
    Config {
        /// Plugin identifier.
        id: String,
        /// Plugin-specific options.
        options: serde_json::Value,
    },
}

impl RepositoryPluginConfig {
    /// Returns the stable plugin identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Id(id) | Self::Config { id, .. } => id.as_str(),
        }
    }
}
