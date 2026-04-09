use std::fs;

use crate::gateway::studio::router::config::types::{WendaoTomlConfig, WendaoTomlPluginEntry};
use crate::gateway::studio::router::config::{
    load_ui_config_from_wendao_toml, persist_ui_config_to_wendao_toml,
    studio_wendao_overlay_toml_path, studio_wendao_toml_path,
};
use crate::gateway::studio::types::{UiConfig, UiRepoProjectConfig};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn load_ui_config_from_wendao_toml_accepts_inline_repo_plugin_config() -> TestResult {
    let temp = tempfile::tempdir()?;
    fs::write(
        temp.path().join("wendao.toml"),
        r#"[link_graph.projects.sample]
root = "."
plugins = [
  "julia",
  { id = "julia", flight_transport = { base_url = "http://127.0.0.1:8815" } }
]
"#,
    )?;

    let Some(config) = load_ui_config_from_wendao_toml(temp.path()) else {
        panic!("ui config should load");
    };
    assert_eq!(config.repo_projects.len(), 1);
    assert_eq!(config.repo_projects[0].id, "sample");
    assert_eq!(config.repo_projects[0].plugins, vec!["julia".to_string()]);
    Ok(())
}

#[test]
fn persist_ui_config_to_wendao_toml_preserves_inline_repo_plugin_config() -> TestResult {
    let temp = tempfile::tempdir()?;
    let config_path = studio_wendao_toml_path(temp.path());
    let overlay_path = studio_wendao_overlay_toml_path(temp.path());
    fs::write(
        &config_path,
        r#"[link_graph.projects.sample]
root = "."
plugins = [
  "julia",
  { id = "julia", flight_transport = { base_url = "http://127.0.0.1:8815", route = "/rerank" } }
]
"#,
    )?;

    persist_ui_config_to_wendao_toml(
        temp.path(),
        &UiConfig {
            projects: Vec::new(),
            repo_projects: vec![UiRepoProjectConfig {
                id: "sample".to_string(),
                root: Some(".".to_string()),
                url: None,
                git_ref: None,
                refresh: None,
                plugins: vec!["julia".to_string()],
            }],
        },
    )?;

    let persisted_base: WendaoTomlConfig = toml::from_str(&fs::read_to_string(&config_path)?)?;
    assert!(persisted_base.imports.is_empty());
    let Some(base_project) = persisted_base.link_graph.projects.get("sample") else {
        panic!("sample project should remain intact in base config");
    };
    assert_eq!(base_project.plugins.len(), 2);

    let persisted_overlay: WendaoTomlConfig = toml::from_str(&fs::read_to_string(&overlay_path)?)?;
    assert_eq!(persisted_overlay.imports, vec!["wendao.toml".to_string()]);
    let Some(project) = persisted_overlay.link_graph.projects.get("sample") else {
        panic!("sample project should persist in overlay");
    };
    assert_eq!(project.plugins.len(), 2);
    assert!(matches!(
        &project.plugins[0],
        WendaoTomlPluginEntry::Id(id) if id == "julia"
    ));
    assert!(matches!(
        &project.plugins[1],
        WendaoTomlPluginEntry::Config(config)
            if config.id == "julia" && config.extra.contains_key("flight_transport")
    ));

    let Some(config) = load_ui_config_from_wendao_toml(temp.path()) else {
        panic!("ui config should load through overlay");
    };
    assert_eq!(config.repo_projects.len(), 1);
    assert_eq!(config.repo_projects[0].id, "sample");
    assert_eq!(config.repo_projects[0].plugins, vec!["julia".to_string()]);
    Ok(())
}

#[test]
fn load_ui_config_from_wendao_toml_prefers_overlay_importing_base() -> TestResult {
    let temp = tempfile::tempdir()?;
    fs::write(
        studio_wendao_toml_path(temp.path()),
        r#"[link_graph.projects.kernel]
root = "."
dirs = ["docs"]
"#,
    )?;
    fs::write(
        studio_wendao_overlay_toml_path(temp.path()),
        r#"imports = ["wendao.toml"]

[link_graph.projects.kernel]
root = "."
dirs = ["docs", "src"]
"#,
    )?;

    let Some(config) = load_ui_config_from_wendao_toml(temp.path()) else {
        panic!("ui config should load through overlay");
    };
    assert_eq!(config.projects.len(), 1);
    assert_eq!(config.projects[0].name, "kernel");
    assert_eq!(
        config.projects[0].dirs,
        vec!["docs".to_string(), "src".to_string()]
    );
    Ok(())
}

#[test]
fn persist_ui_config_to_wendao_toml_tombstones_base_projects_not_in_ui_state() -> TestResult {
    let temp = tempfile::tempdir()?;
    fs::write(
        studio_wendao_toml_path(temp.path()),
        r#"[link_graph.projects.kernel]
root = "."
dirs = ["docs"]

[link_graph.projects.sample]
root = "."
plugins = ["julia"]
"#,
    )?;

    persist_ui_config_to_wendao_toml(
        temp.path(),
        &UiConfig {
            projects: Vec::new(),
            repo_projects: vec![UiRepoProjectConfig {
                id: "main".to_string(),
                root: Some(".".to_string()),
                url: None,
                git_ref: None,
                refresh: None,
                plugins: vec!["julia".to_string()],
            }],
        },
    )?;

    let Some(config) = load_ui_config_from_wendao_toml(temp.path()) else {
        panic!("ui config should load through overlay");
    };
    assert!(config.projects.is_empty());
    assert_eq!(config.repo_projects.len(), 1);
    assert_eq!(config.repo_projects[0].id, "main");
    Ok(())
}
