use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::errors::RepoIntelligenceError;

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct WendaoTomlConfig {
    #[serde(default)]
    link_graph: WendaoTomlLinkGraphConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct WendaoTomlLinkGraphConfig {
    #[serde(default)]
    projects: BTreeMap<String, WendaoTomlProjectConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct WendaoTomlProjectConfig {
    #[serde(default)]
    root: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(rename = "ref", default)]
    git_ref: Option<String>,
    #[serde(default)]
    refresh: Option<String>,
    #[serde(default)]
    plugins: Vec<String>,
}

impl RepositoryPluginConfig {
    /// Returns the stable plugin identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Id(id) => id.as_str(),
            Self::Config { id, .. } => id.as_str(),
        }
    }
}

/// Load the repo intelligence configuration from the project.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when configuration cannot be loaded.
pub fn load_repo_intelligence_config(
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoIntelligenceConfig, RepoIntelligenceError> {
    let config_path = config_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| cwd.join("wendao.toml"));
    let contents =
        fs::read_to_string(&config_path).map_err(|error| RepoIntelligenceError::ConfigLoad {
            message: format!("failed to read `{}`: {error}", config_path.display()),
        })?;
    let parsed: WendaoTomlConfig =
        toml::from_str(&contents).map_err(|error| RepoIntelligenceError::ConfigLoad {
            message: format!("failed to parse `{}`: {error}", config_path.display()),
        })?;

    let config_root = config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| cwd.to_path_buf());

    let repos = parsed
        .link_graph
        .projects
        .into_iter()
        .filter_map(|(id, project)| {
            let plugins = project
                .plugins
                .into_iter()
                .map(|plugin| plugin.trim().to_string())
                .filter(|plugin| !plugin.is_empty())
                .map(RepositoryPluginConfig::Id)
                .collect::<Vec<_>>();
            if plugins.is_empty() {
                return None;
            }

            let path = project
                .root
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .map(|path| {
                    if path.is_absolute() {
                        normalize_path(path)
                    } else {
                        normalize_path(config_root.join(path))
                    }
                });
            let url = project
                .url
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string);
            if path.is_none() && url.is_none() {
                return None;
            }

            Some(RegisteredRepository {
                id,
                path,
                url,
                git_ref: project.git_ref.as_deref().and_then(parse_repository_ref),
                refresh: parse_refresh_policy(project.refresh.as_deref()),
                plugins,
            })
        })
        .collect();

    Ok(RepoIntelligenceConfig { repos })
}

fn parse_repository_ref(value: &str) -> Option<RepositoryRef> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(RepositoryRef::Branch(trimmed.to_string()))
}

fn parse_refresh_policy(value: Option<&str>) -> RepositoryRefreshPolicy {
    match value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("fetch")
    {
        "manual" => RepositoryRefreshPolicy::Manual,
        _ => RepositoryRefreshPolicy::Fetch,
    }
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                let popped = normalized.pop();
                if !popped {
                    normalized.push(component.as_os_str());
                }
            }
            Component::RootDir | Component::Prefix(_) | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }

    normalized
}
