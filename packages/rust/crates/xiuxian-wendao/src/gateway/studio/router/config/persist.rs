use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use crate::gateway::studio::types::UiConfig;

use super::load::load_wendao_toml_config;
use super::paths::{
    studio_effective_wendao_toml_path, studio_wendao_overlay_toml_path, studio_wendao_toml_path,
};
use super::sanitize::merge_repo_plugins;
use super::types::{WendaoTomlConfig, WendaoTomlProjectConfig};

/// Persists UI config into the base Wendao TOML.
///
/// # Errors
///
/// Returns an error string if reading, parsing, or writing fails.
pub fn persist_ui_config_to_wendao_toml(
    config_root: &Path,
    config: &UiConfig,
) -> Result<(), String> {
    let effective_path = studio_effective_wendao_toml_path(config_root);
    persist_ui_config_to_wendao_toml_path(effective_path.as_path(), config)
}

/// Persists UI config using one explicit effective Wendao TOML path.
///
/// # Errors
///
/// Returns an error string if reading, parsing, or writing fails.
pub fn persist_ui_config_to_wendao_toml_path(
    config_path: &Path,
    config: &UiConfig,
) -> Result<(), String> {
    let paths = resolve_persist_paths(config_path)?;
    let mut effective = load_or_default_wendao_config(paths.effective.as_path())?;
    let mut base = load_raw_or_default_wendao_config(paths.base.as_path())?;
    let projects = merge_ui_projects(&mut effective, &base, config);
    base.link_graph.projects = projects;
    write_base_config(paths.base.as_path(), &base)?;
    remove_legacy_overlay(paths.legacy_overlay.as_path(), paths.base.as_path())
}

struct PersistPaths {
    base: std::path::PathBuf,
    effective: std::path::PathBuf,
    legacy_overlay: std::path::PathBuf,
}

fn resolve_persist_paths(config_path: &Path) -> Result<PersistPaths, String> {
    let Some(config_root) = config_path.parent() else {
        return Err(format!(
            "failed to resolve config dir for `{}`",
            config_path.display()
        ));
    };
    let legacy_overlay = studio_wendao_overlay_toml_path(config_root);
    let base = if config_path == legacy_overlay.as_path() {
        studio_wendao_toml_path(config_root)
    } else {
        config_path.to_path_buf()
    };
    let effective = if legacy_overlay.is_file() {
        legacy_overlay.clone()
    } else if base.is_file() {
        base.clone()
    } else {
        config_path.to_path_buf()
    };

    Ok(PersistPaths {
        base,
        effective,
        legacy_overlay,
    })
}

fn load_or_default_wendao_config(effective_path: &Path) -> Result<WendaoTomlConfig, String> {
    if effective_path.is_file() {
        load_wendao_toml_config(effective_path).map_err(|details| {
            format!(
                "failed to read `{}` before persisting UI config: {details}",
                effective_path.display()
            )
        })
    } else {
        Ok(WendaoTomlConfig::default())
    }
}

fn load_raw_or_default_wendao_config(config_path: &Path) -> Result<WendaoTomlConfig, String> {
    if !config_path.is_file() {
        return Ok(WendaoTomlConfig::default());
    }

    let raw = fs::read_to_string(config_path).map_err(|error| {
        format!(
            "failed to read `{}` before persisting UI config: {error}",
            config_path.display()
        )
    })?;
    toml::from_str(&raw).map_err(|error| {
        format!(
            "failed to parse raw TOML `{}` before persisting UI config: {error}",
            config_path.display()
        )
    })
}

fn merge_ui_projects(
    parsed: &mut WendaoTomlConfig,
    base: &WendaoTomlConfig,
    config: &UiConfig,
) -> BTreeMap<String, WendaoTomlProjectConfig> {
    let local_project_ids = config
        .projects
        .iter()
        .map(|project| project.name.clone())
        .collect::<BTreeSet<_>>();
    let repo_project_ids = config
        .repo_projects
        .iter()
        .map(|project| project.id.clone())
        .collect::<BTreeSet<_>>();
    let local_and_repo_ids = local_project_ids
        .union(&repo_project_ids)
        .cloned()
        .collect::<BTreeSet<_>>();

    let mut existing_projects = std::mem::take(&mut parsed.link_graph.projects);
    let base_project_ids = base
        .link_graph
        .projects
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut projects = BTreeMap::<String, WendaoTomlProjectConfig>::new();
    for project in &config.projects {
        let mut entry = existing_projects.remove(&project.name).unwrap_or_default();
        entry.root = Some(project.root.clone());
        entry.dirs.clone_from(&project.dirs);
        if !repo_project_ids.contains(&project.name) {
            entry.plugins.clear();
        }
        projects.insert(project.name.clone(), entry);
    }
    for repo in &config.repo_projects {
        let mut entry = projects
            .remove(&repo.id)
            .or_else(|| existing_projects.remove(&repo.id))
            .unwrap_or_default();
        if let Some(root) = repo.root.clone() {
            entry.root = Some(root);
        }
        entry.url.clone_from(&repo.url);
        entry.git_ref.clone_from(&repo.git_ref);
        entry.refresh.clone_from(&repo.refresh);
        entry.plugins = merge_repo_plugins(entry.plugins, &repo.plugins);
        if !local_project_ids.contains(&repo.id) {
            entry.dirs.clear();
        }
        projects.insert(repo.id.clone(), entry);
    }

    for (id, mut entry) in existing_projects {
        if local_and_repo_ids.contains(&id) {
            continue;
        }
        if !base_project_ids.contains(&id) {
            continue;
        }
        entry.dirs.clear();
        entry.plugins.clear();
        projects.insert(id, entry);
    }

    projects
}

fn write_base_config(config_path: &Path, config: &WendaoTomlConfig) -> Result<(), String> {
    let serialized = toml::to_string_pretty(config).map_err(|error| {
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
    fs::write(config_path, serialized).map_err(|error| {
        format!(
            "failed to write persisted UI config `{}`: {error}",
            config_path.display()
        )
    })
}

fn remove_legacy_overlay(overlay_path: &Path, base_path: &Path) -> Result<(), String> {
    if overlay_path == base_path || !overlay_path.is_file() {
        return Ok(());
    }

    fs::remove_file(overlay_path).map_err(|error| {
        format!(
            "failed to remove legacy Studio overlay `{}` after persisting base config: {error}",
            overlay_path.display()
        )
    })
}
