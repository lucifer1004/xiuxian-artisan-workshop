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

/// Persists UI config as a Studio overlay TOML.
///
/// # Errors
///
/// Returns an error string if reading, parsing, or writing fails.
pub fn persist_ui_config_to_wendao_toml(
    config_root: &Path,
    config: &UiConfig,
) -> Result<(), String> {
    let base_path = studio_wendao_toml_path(config_root);
    let overlay_path = studio_wendao_overlay_toml_path(config_root);
    let effective_path = studio_effective_wendao_toml_path(config_root);

    let mut parsed = if effective_path.is_file() {
        load_wendao_toml_config(effective_path.as_path()).map_err(|details| {
            format!(
                "failed to read `{}` before persisting UI config: {details}",
                effective_path.display()
            )
        })?
    } else {
        WendaoTomlConfig::default()
    };
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

        entry.dirs.clear();
        entry.plugins.clear();
        projects.insert(id, entry);
    }

    let mut overlay = WendaoTomlConfig::default();
    if base_path.is_file() {
        overlay.imports.push("wendao.toml".to_string());
    }
    overlay.link_graph.projects = projects;

    let serialized = toml::to_string_pretty(&overlay).map_err(|error| {
        format!(
            "failed to serialize UI config into TOML `{}`: {error}",
            overlay_path.display()
        )
    })?;
    if let Some(parent) = overlay_path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create config dir `{}`: {error}",
                parent.display()
            )
        })?;
    }
    fs::write(overlay_path.as_path(), serialized).map_err(|error| {
        format!(
            "failed to write persisted UI config `{}`: {error}",
            overlay_path.display()
        )
    })
}
