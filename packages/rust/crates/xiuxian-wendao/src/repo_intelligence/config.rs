use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::errors::RepoIntelligenceError;

/// Top-level Repo Intelligence runtime configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoIntelligenceConfig {
    /// Repositories registered for indexing and querying.
    #[serde(default)]
    pub repos: Vec<RegisteredRepository>,
}

/// One repository registered with the Repo Intelligence runtime.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
    /// Refresh behavior for managed checkouts.
    #[serde(default)]
    pub refresh: RepositoryRefreshPolicy,
    /// Enabled plugin identifiers for this repository.
    #[serde(default)]
    pub plugins: Vec<RepositoryPluginConfig>,
}

/// Git reference selection for a registered repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum RepositoryRef {
    /// Track a named branch, such as `main`.
    Branch(String),
}

impl RepositoryRef {
    /// Return the raw git reference string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Branch(value) => value,
        }
    }
}

/// Refresh policy for one registered repository.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RepositoryRefreshPolicy {
    /// Refresh managed checkouts on analysis.
    #[default]
    Fetch,
    /// Do not refresh an existing managed checkout automatically.
    Manual,
}

/// Plugin enablement entry for one repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum RepositoryPluginConfig {
    /// Short string form, for example `plugins = ["julia"]`.
    Id(String),
    /// Expanded object form for future plugin-specific settings.
    Config {
        /// Stable plugin identifier, for example `julia` or `modelica`.
        id: String,
    },
}

impl RepositoryPluginConfig {
    /// Return the stable plugin identifier.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Id(id) | Self::Config { id } => id,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct WendaoConfigFile {
    #[serde(default)]
    link_graph: RawLinkGraphConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct RawLinkGraphConfig {
    #[serde(default)]
    projects: BTreeMap<String, RawLinkGraphProjectConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
struct RawLinkGraphProjectConfig {
    #[serde(default)]
    root: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(rename = "ref", default)]
    git_ref: Option<RepositoryRef>,
    #[serde(default)]
    refresh: RepositoryRefreshPolicy,
    #[serde(default)]
    plugins: Vec<RepositoryPluginConfig>,
}

/// Load Repo Intelligence configuration from `wendao.toml`.
///
/// If `config_path` is provided, that file is used directly. Otherwise the
/// loader checks for `wendao.toml` under `cwd`. When no config file is found,
/// an empty config is returned.
///
/// Relative repository paths are resolved against the active config file
/// directory. When no explicit config file is used, relative paths resolve
/// against `cwd`.
///
/// # Errors
///
/// Returns [`RepoIntelligenceError`] when the configuration file cannot be read
/// or parsed.
pub fn load_repo_intelligence_config(
    config_path: Option<&Path>,
    cwd: &Path,
) -> Result<RepoIntelligenceConfig, RepoIntelligenceError> {
    let resolved_config_path = resolve_config_path(config_path, cwd);
    let Some(active_config_path) = resolved_config_path else {
        return Ok(RepoIntelligenceConfig::default());
    };

    let config_contents = fs::read_to_string(&active_config_path).map_err(|error| {
        RepoIntelligenceError::ConfigLoad {
            message: format!(
                "failed to read repo intelligence config `{}`: {error}",
                active_config_path.display()
            ),
        }
    })?;
    let parsed: WendaoConfigFile =
        toml::from_str(&config_contents).map_err(|error| RepoIntelligenceError::ConfigLoad {
            message: format!(
                "failed to parse repo intelligence config `{}`: {error}",
                active_config_path.display()
            ),
        })?;

    let base_dir = active_config_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| cwd.to_path_buf());
    Ok(resolve_runtime_config(parsed, &base_dir))
}

fn resolve_config_path(config_path: Option<&Path>, cwd: &Path) -> Option<PathBuf> {
    match config_path {
        Some(path) if path.is_absolute() => Some(path.to_path_buf()),
        Some(path) => Some(cwd.join(path)),
        None => {
            let local = cwd.join("wendao.toml");
            local.is_file().then_some(local)
        }
    }
}

fn resolve_runtime_config(parsed: WendaoConfigFile, base_dir: &Path) -> RepoIntelligenceConfig {
    let mut repositories = BTreeMap::new();

    for (project_id, project) in parsed.link_graph.projects {
        let Some(repository) =
            registered_repository_from_project(project_id.clone(), project, base_dir)
        else {
            continue;
        };
        repositories.insert(project_id, repository);
    }

    RepoIntelligenceConfig {
        repos: repositories.into_values().collect(),
    }
}

fn registered_repository_from_project(
    id: String,
    project: RawLinkGraphProjectConfig,
    base_dir: &Path,
) -> Option<RegisteredRepository> {
    if project.plugins.is_empty() {
        return None;
    }

    let path = project
        .root
        .as_deref()
        .and_then(|raw| resolve_project_root(base_dir, raw));
    let url = normalize_nonempty_string(project.url);
    if path.is_none() && url.is_none() {
        return None;
    }

    Some(RegisteredRepository {
        id,
        path,
        url,
        git_ref: project.git_ref,
        refresh: project.refresh,
        plugins: project.plugins,
    })
}

fn resolve_project_root(base_dir: &Path, raw: &str) -> Option<PathBuf> {
    let normalized = raw.trim();
    if normalized.is_empty() {
        return None;
    }

    let resolved = if normalized == "." {
        base_dir.to_path_buf()
    } else if normalized == "~" {
        dirs::home_dir()?
    } else if let Some(rest) = normalized.strip_prefix("~/") {
        dirs::home_dir()?.join(rest)
    } else {
        let candidate = Path::new(normalized);
        if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            base_dir.join(candidate)
        }
    };

    Some(normalize_path(resolved))
}

fn normalize_nonempty_string(value: Option<String>) -> Option<String> {
    value.and_then(|entry| {
        let trimmed = entry.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_string())
    })
}

fn normalize_path(path: PathBuf) -> PathBuf {
    let mut normalized = PathBuf::new();

    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => {
                normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR))
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            }
            std::path::Component::Normal(part) => normalized.push(part),
        }
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn assert_config_json_snapshot(name: &str, value: serde_json::Value) {
        insta::with_settings!({
            sort_maps => true,
        }, {
            insta::assert_json_snapshot!(name, value);
        });
    }

    #[test]
    fn load_config_derives_repositories_from_link_graph_projects() -> TestResult {
        let temp = tempfile::tempdir()?;
        let config_dir = temp.path().join("config");
        let repo_dir = temp.path().join("repos").join("sample");
        fs::create_dir_all(&config_dir)?;
        fs::create_dir_all(&repo_dir)?;

        let config_path = config_dir.join("wendao.toml");
        fs::write(
            &config_path,
            r#"[link_graph.projects.kernel]
root = "."
dirs = ["docs"]

[link_graph.projects.sample]
root = "../repos/sample"
dirs = ["docs"]
plugins = ["julia"]
"#,
        )?;

        let config = load_repo_intelligence_config(Some(&config_path), temp.path())?;
        assert_eq!(config.repos.len(), 1);
        assert_eq!(config.repos[0].id, "sample");
        assert_eq!(config.repos[0].path.as_deref(), Some(repo_dir.as_path()));
        assert_eq!(config.repos[0].url, None);
        Ok(())
    }

    #[test]
    fn load_config_skips_projects_without_plugins_or_source() -> TestResult {
        let temp = tempfile::tempdir()?;
        let config_path = temp.path().join("wendao.toml");
        fs::write(
            &config_path,
            r#"[link_graph.projects.docs_only]
root = "."
dirs = ["docs"]

[link_graph.projects.missing_source]
plugins = ["julia"]

[link_graph.projects.remote_pkg]
url = "https://example.invalid/remote.git"
plugins = ["modelica"]
"#,
        )?;

        let config = load_repo_intelligence_config(Some(&config_path), temp.path())?;
        assert_eq!(config.repos.len(), 1);
        assert_eq!(config.repos[0].id, "remote_pkg");
        assert_eq!(
            config.repos[0].url.as_deref(),
            Some("https://example.invalid/remote.git")
        );
        assert!(config.repos[0].path.is_none());
        Ok(())
    }

    #[test]
    fn load_config_prefers_project_root_and_preserves_url_metadata() -> TestResult {
        let temp = tempfile::tempdir()?;
        let repo_dir = temp.path().join("repo");
        fs::create_dir_all(&repo_dir)?;
        let config_path = temp.path().join("wendao.toml");
        fs::write(
            &config_path,
            format!(
                r#"[link_graph.projects.sample]
root = "{}"
url = "https://example.invalid/sample.git"
plugins = ["julia"]
"#,
                repo_dir.display()
            ),
        )?;

        let config = load_repo_intelligence_config(Some(&config_path), temp.path())?;
        assert_eq!(config.repos.len(), 1);
        assert_eq!(config.repos[0].path.as_deref(), Some(repo_dir.as_path()));
        assert_eq!(
            config.repos[0].url.as_deref(),
            Some("https://example.invalid/sample.git")
        );
        Ok(())
    }

    #[test]
    fn load_config_expands_tilde_project_roots() -> TestResult {
        let Some(home_dir) = dirs::home_dir() else {
            return Ok(());
        };

        let temp = tempfile::tempdir()?;
        let config_path = temp.path().join("wendao.toml");
        fs::write(
            &config_path,
            r#"[link_graph.projects.sample]
root = "~/ghq/example-repo"
plugins = ["julia"]
"#,
        )?;

        let config = load_repo_intelligence_config(Some(&config_path), temp.path())?;
        assert_eq!(config.repos.len(), 1);
        assert_eq!(
            config.repos[0].path.as_deref(),
            Some(home_dir.join("ghq/example-repo").as_path())
        );
        Ok(())
    }

    #[test]
    fn load_config_normalizes_project_repo_contract_snapshot() -> TestResult {
        let temp = tempfile::tempdir()?;
        let config_dir = temp.path().join("config");
        let local_repo_dir = temp.path().join("repos").join("modelica");
        fs::create_dir_all(&config_dir)?;
        fs::create_dir_all(&local_repo_dir)?;

        let config_path = config_dir.join("wendao.toml");
        fs::write(
            &config_path,
            r#"[link_graph.projects.kernel]
root = "."
dirs = ["docs"]

[link_graph.projects.local_modelica]
root = "../repos/modelica"
url = "https://example.invalid/modelica.git"
plugins = ["modelica"]

[link_graph.projects.sciml]
url = "https://example.invalid/sciml.git"
ref = "main"
refresh = "manual"
plugins = ["julia"]
"#,
        )?;

        let config = load_repo_intelligence_config(Some(&config_path), temp.path())?;
        let payload = json!({
            "repos": config.repos.iter().map(|repository| {
                json!({
                    "id": repository.id,
                    "path": repository.path.as_ref().and_then(|path| {
                        path.strip_prefix(temp.path())
                            .ok()
                            .map(|relative| relative.display().to_string())
                    }),
                    "url": repository.url,
                    "ref": repository.git_ref.as_ref().map(RepositoryRef::as_str),
                    "refresh": repository.refresh,
                    "plugins": repository.plugins.iter().map(RepositoryPluginConfig::id).collect::<Vec<_>>(),
                })
            }).collect::<Vec<_>>(),
        });
        assert_config_json_snapshot("load_config_normalizes_project_repo_contract", payload);
        Ok(())
    }

    #[test]
    fn load_config_ignores_legacy_repo_intelligence_repos_table() -> TestResult {
        let temp = tempfile::tempdir()?;
        let config_path = temp.path().join("wendao.toml");
        fs::write(
            &config_path,
            r#"[[repo_intelligence.repos]]
id = "legacy"
path = "./legacy"
plugins = ["julia"]

[link_graph.projects.sciml]
url = "https://example.invalid/sciml.git"
plugins = ["julia"]
"#,
        )?;

        let config = load_repo_intelligence_config(Some(&config_path), temp.path())?;
        assert_eq!(config.repos.len(), 1);
        assert_eq!(config.repos[0].id, "sciml");
        assert_eq!(
            config.repos[0].url.as_deref(),
            Some("https://example.invalid/sciml.git")
        );
        Ok(())
    }
}
