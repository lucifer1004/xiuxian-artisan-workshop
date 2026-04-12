use std::collections::HashMap;
use std::path::{Path, PathBuf};
use xiuxian_qianhuan::{ManifestationManager, MemoryTemplateRecord};
use xiuxian_wendao::{WendaoResourceUri, embedded_resource_text_from_wendao_uri};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ZhixingSkillTemplateLoadSummary {
    pub(crate) linked_ids: usize,
    pub(crate) template_records: usize,
    pub(crate) loaded_template_names: usize,
}

pub(crate) fn resolve_project_root_with_prj_root(
    prj_root: Option<&str>,
    current_dir: &Path,
) -> PathBuf {
    resolve_project_root_from_override(prj_root.map(str::to_owned), current_dir)
}

fn resolve_project_root_from_override(env_override: Option<String>, current_dir: &Path) -> PathBuf {
    if let Some(root) = env_override
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    {
        return root;
    }

    let mut cursor = current_dir.to_path_buf();
    loop {
        let has_git = cursor.join(".git").exists();
        let has_system_config = cursor
            .join("packages")
            .join("conf")
            .join("xiuxian.toml")
            .is_file();
        if has_git || has_system_config {
            return cursor;
        }
        if !cursor.pop() {
            return current_dir.to_path_buf();
        }
    }
}

pub(crate) fn resolve_prj_data_home_with_env(
    project_root: &Path,
    prj_data_home: Option<&str>,
) -> PathBuf {
    resolve_prj_data_home_from_override(project_root, prj_data_home.map(str::to_owned))
}

fn resolve_prj_data_home_from_override(
    project_root: &Path,
    prj_data_home: Option<String>,
) -> PathBuf {
    prj_data_home
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map_or_else(|| project_root.join(".data"), PathBuf::from)
}

pub(crate) fn resolve_notebook_root(
    prj_data_home: &Path,
    env_notebook_path: Option<String>,
    config_notebook_path: Option<String>,
) -> PathBuf {
    env_notebook_path
        .map(PathBuf::from)
        .or_else(|| config_notebook_path.map(PathBuf::from))
        .unwrap_or_else(|| prj_data_home.join("xiuxian").join("notebook"))
}

pub(crate) fn resolve_template_globs_with_resource_root(
    project_root: &Path,
    config_template_paths: Option<Vec<String>>,
    resource_root_override: Option<&str>,
) -> Vec<String> {
    resolve_template_globs_with_runtime_overrides(
        project_root,
        config_template_paths,
        resource_root_override.map(str::to_owned),
        None,
    )
}

fn dedup_paths_in_order(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut unique = Vec::new();
    for path in paths {
        if !unique.contains(&path) {
            unique.push(path);
        }
    }
    unique
}

fn dedup_strings_in_order(values: Vec<String>) -> Vec<String> {
    let mut unique = Vec::new();
    for value in values {
        if !unique.contains(&value) {
            unique.push(value);
        }
    }
    unique
}

fn resolve_template_globs_with_runtime_overrides(
    project_root: &Path,
    config_template_paths: Option<Vec<String>>,
    resource_root_override: Option<String>,
    executable_dir: Option<&Path>,
) -> Vec<String> {
    let mut roots =
        resolve_runtime_template_candidates(project_root, resource_root_override, executable_dir)
            .into_iter()
            .filter(|path| path.is_dir())
            .collect::<Vec<_>>();
    if let Some(custom_paths) = config_template_paths.filter(|paths| !paths.is_empty()) {
        roots.extend(
            custom_paths
                .into_iter()
                .filter_map(|value| {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        return None;
                    }
                    let path = PathBuf::from(trimmed);
                    Some(if path.is_absolute() {
                        path
                    } else {
                        project_root.join(path)
                    })
                })
                .filter(|path| path.is_dir())
                .collect::<Vec<_>>(),
        );
    }
    dedup_paths_in_order(roots)
        .into_iter()
        .map(|path| path.join("*.md").to_string_lossy().into_owned())
        .collect::<Vec<_>>()
}

fn resolve_runtime_template_candidates(
    project_root: &Path,
    resource_root_override: Option<String>,
    executable_dir: Option<&Path>,
) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(resource_root) = resource_root_override
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else {
                project_root.join(path)
            }
        })
    {
        candidates.push(
            resource_root
                .join("xiuxian-daochang")
                .join("zhixing")
                .join("templates"),
        );
        candidates.push(resource_root.join("zhixing").join("templates"));
    }

    if let Some(executable_dir) = executable_dir {
        candidates.push(
            executable_dir
                .join("..")
                .join("resources")
                .join("zhixing")
                .join("templates"),
        );
        candidates.push(
            executable_dir
                .join("resources")
                .join("zhixing")
                .join("templates"),
        );
    }

    candidates
}

pub(crate) fn load_skill_templates_from_embedded_registry(
    manager: &ManifestationManager,
) -> Result<ZhixingSkillTemplateLoadSummary, String> {
    const TEMPLATE_CONFIG_TYPE: &str = "template";

    let registry = xiuxian_wendao::build_embedded_wendao_registry().map_err(|error| {
        format!("failed to build embedded zhixing wendao registry for skill bridge: {error}")
    })?;
    let mut links_by_template_id: HashMap<String, Vec<String>> = HashMap::new();
    for file in registry.files() {
        for (id, targets) in file.link_targets_by_id() {
            let entry = links_by_template_id.entry(id.clone()).or_default();
            for target in targets {
                if target.reference_type.as_deref() != Some(TEMPLATE_CONFIG_TYPE) {
                    continue;
                }
                if !entry
                    .iter()
                    .any(|existing| existing == target.target_path.as_str())
                {
                    entry.push(target.target_path.clone());
                }
            }
        }
    }

    let mut id_links = links_by_template_id.into_iter().collect::<Vec<_>>();
    id_links.sort_by(|(left_id, _), (right_id, _)| left_id.cmp(right_id));

    let mut records = Vec::new();
    let mut linked_ids = 0usize;

    for (id, links) in id_links {
        let deduped_links = dedup_strings_in_order(links);
        if deduped_links.is_empty() {
            continue;
        }
        linked_ids += 1;

        let alias_target = (deduped_links.len() == 1).then(|| id.clone());
        for link_uri in deduped_links {
            if WendaoResourceUri::parse(link_uri.as_str()).is_err() {
                return Err(format!(
                    "template link `{link_uri}` for id `{id}` must use semantic URI `wendao://skills/<name>/references/<entity>`"
                ));
            }
            let Some(content) = embedded_resource_text_from_wendao_uri(link_uri.as_str()) else {
                return Err(format!(
                    "linked template URI `{link_uri}` for id `{id}` not found in embedded zhixing resources"
                ));
            };
            records.push(MemoryTemplateRecord::new(
                link_uri,
                alias_target.clone(),
                content,
            ));
        }
    }

    let template_records = records.len();
    let loaded_template_names = manager
        .load_templates_from_memory(records)
        .map_err(|error| format!("failed to load linked templates into manifestation: {error}"))?;

    Ok(ZhixingSkillTemplateLoadSummary {
        linked_ids,
        template_records,
        loaded_template_names,
    })
}
