//! TOML configuration loading and persistence for Studio API.

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use xiuxian_io::PrjDirs;

use crate::gateway::studio::pathing;
use crate::gateway::studio::types::{UiConfig, UiProjectConfig, UiRepoProjectConfig};

/// Root configuration structure for `wendao.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WendaoTomlConfig {
    #[serde(default)]
    gateway: WendaoTomlGatewayConfig,
    #[serde(default)]
    link_graph: WendaoTomlLinkGraphConfig,
    #[serde(default, flatten)]
    extra: BTreeMap<String, toml::Value>,
}

/// Gateway-specific configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct WendaoTomlGatewayConfig {
    #[serde(default)]
    bind: Option<String>,
    #[serde(default, flatten)]
    extra: BTreeMap<String, toml::Value>,
}

/// Link graph configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct WendaoTomlLinkGraphConfig {
    #[serde(default)]
    projects: BTreeMap<String, WendaoTomlProjectConfig>,
    #[serde(default, flatten)]
    extra: BTreeMap<String, toml::Value>,
}

/// Per-project configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct WendaoTomlProjectConfig {
    #[serde(default)]
    root: Option<String>,
    #[serde(default)]
    dirs: Vec<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(rename = "ref", default)]
    git_ref: Option<String>,
    #[serde(default)]
    refresh: Option<String>,
    #[serde(default)]
    plugins: Vec<String>,
    #[serde(default, flatten)]
    extra: BTreeMap<String, toml::Value>,
}

/// Returns the path to `wendao.toml` for the given config root.
pub fn studio_wendao_toml_path(config_root: &Path) -> PathBuf {
    config_root.join("wendao.toml")
}

/// Loads UI config from `wendao.toml` if it exists.
pub fn load_ui_config_from_wendao_toml(config_root: &Path) -> Option<UiConfig> {
    let config_path = studio_wendao_toml_path(config_root);
    let contents = fs::read_to_string(config_path).ok()?;
    let parsed: WendaoTomlConfig = toml::from_str(&contents).ok()?;
    Some(ui_config_from_wendao_toml(parsed))
}

/// Converts parsed TOML config to `UiConfig`.
fn ui_config_from_wendao_toml(parsed: WendaoTomlConfig) -> UiConfig {
    let mut projects = Vec::new();
    let mut repo_projects = Vec::new();

    for (id, project) in parsed.link_graph.projects {
        let dirs = sanitize_path_list(&project.dirs);
        let root = project
            .root
            .as_deref()
            .and_then(sanitize_path_like)
            .unwrap_or_else(|| ".".to_string());
        if !dirs.is_empty() {
            projects.push(UiProjectConfig {
                name: id.clone(),
                root,
                dirs,
            });
        }

        let mut plugin_seen = HashSet::<String>::new();
        let plugins = project
            .plugins
            .into_iter()
            .map(|plugin| plugin.trim().to_string())
            .filter(|plugin| !plugin.is_empty())
            .filter(|plugin| plugin_seen.insert(plugin.clone()))
            .collect::<Vec<_>>();
        if plugins.is_empty() {
            continue;
        }

        let repo_root = project.root.as_deref().and_then(sanitize_path_like);
        let url = project
            .url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if repo_root.is_none() && url.is_none() {
            continue;
        }
        let git_ref = project
            .git_ref
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let refresh = project
            .refresh
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        repo_projects.push(UiRepoProjectConfig {
            id,
            root: repo_root,
            url,
            git_ref,
            refresh,
            plugins,
        });
    }

    UiConfig {
        projects: sanitize_projects(projects),
        repo_projects: sanitize_repo_projects(repo_projects),
    }
}

/// Persists UI config to `wendao.toml`.
///
/// # Errors
///
/// Returns an error string if reading, parsing, or writing fails.
pub fn persist_ui_config_to_wendao_toml(
    config_root: &Path,
    config: &UiConfig,
) -> Result<(), String> {
    let config_path = studio_wendao_toml_path(config_root);
    let mut parsed = if config_path.is_file() {
        let existing = fs::read_to_string(config_path.as_path()).map_err(|error| {
            format!(
                "failed to read `{}` before persisting UI config: {error}",
                config_path.display()
            )
        })?;
        toml::from_str::<WendaoTomlConfig>(&existing).unwrap_or_default()
    } else {
        WendaoTomlConfig::default()
    };

    let mut projects = BTreeMap::<String, WendaoTomlProjectConfig>::new();
    for project in &config.projects {
        projects.insert(
            project.name.clone(),
            WendaoTomlProjectConfig {
                root: Some(project.root.clone()),
                dirs: project.dirs.clone(),
                ..WendaoTomlProjectConfig::default()
            },
        );
    }
    for repo in &config.repo_projects {
        let entry = projects.entry(repo.id.clone()).or_default();
        if let Some(root) = repo.root.clone() {
            entry.root = Some(root);
        }
        entry.url = repo.url.clone();
        entry.git_ref = repo.git_ref.clone();
        entry.refresh = repo.refresh.clone();
        entry.plugins = repo.plugins.clone();
    }
    parsed.link_graph.projects = projects;

    let serialized = toml::to_string_pretty(&parsed).map_err(|error| {
        format!(
            "failed to serialize UI config into TOML `{}`: {error}",
            config_path.display()
        )
    })?;
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create config dir `{}`: {error}",
                parent.display()
            )
        })?;
    }
    fs::write(config_path.as_path(), serialized).map_err(|error| {
        format!(
            "failed to write persisted UI config `{}`: {error}",
            config_path.display()
        )
    })
}

/// Resolves the studio config root directory.
pub fn resolve_studio_config_root(project_root: &Path) -> PathBuf {
    let candidate = PrjDirs::data_home().join("wendao-frontend");
    if candidate.exists() {
        candidate
    } else {
        project_root.to_path_buf()
    }
}

// --- Internal sanitization helpers (duplicated to avoid circular dependency) ---

fn sanitize_projects(raw: Vec<UiProjectConfig>) -> Vec<UiProjectConfig> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::new();
    for project in raw {
        let name = project.name.trim();
        if name.is_empty() {
            continue;
        }
        if !seen.insert(name.to_string()) {
            continue;
        }

        let Some(root) = sanitize_path_like(project.root.as_str()) else {
            continue;
        };

        out.push(UiProjectConfig {
            name: name.to_string(),
            root,
            dirs: sanitize_path_list(&project.dirs),
        });
    }
    out
}

fn sanitize_path_list(raw: &[String]) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::new();
    for path in raw {
        let Some(normalized) = pathing::normalize_project_dir_root(path.as_str()) else {
            continue;
        };
        if seen.insert(normalized.clone()) {
            out.push(normalized);
        }
    }
    out
}

fn sanitize_repo_projects(raw: Vec<UiRepoProjectConfig>) -> Vec<UiRepoProjectConfig> {
    let mut seen = HashSet::<String>::new();
    let mut out = Vec::new();
    for project in raw {
        let id = project.id.trim();
        if id.is_empty() || !seen.insert(id.to_string()) {
            continue;
        }
        let root = project
            .root
            .as_deref()
            .and_then(sanitize_path_like)
            .filter(|value| !value.is_empty());
        let url = project
            .url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let git_ref = project
            .git_ref
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let refresh = project
            .refresh
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let mut plugin_seen = HashSet::<String>::new();
        let plugins = project
            .plugins
            .into_iter()
            .map(|plugin| plugin.trim().to_string())
            .filter(|plugin| !plugin.is_empty())
            .filter(|plugin| plugin_seen.insert(plugin.clone()))
            .collect::<Vec<_>>();
        if plugins.is_empty() {
            continue;
        }
        if root.is_none() && url.is_none() {
            continue;
        }
        out.push(UiRepoProjectConfig {
            id: id.to_string(),
            root,
            url,
            git_ref,
            refresh,
            plugins,
        });
    }
    out
}

fn sanitize_path_like(raw: &str) -> Option<String> {
    pathing::normalize_path_like(raw)
}
