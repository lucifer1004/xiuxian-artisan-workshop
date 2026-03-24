use crate::{ConfigCascadeSpec, ConfigCoreError};
use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};

mod discover;
mod imports;
mod merge;

use self::discover::{existing_config_files, global_candidates, orphan_candidates};
use self::merge::merge_values;

pub use imports::load_toml_value_with_imports;

/// Resolve layered files and return merged TOML value.
///
/// Merge order:
/// 1. Embedded defaults (`spec.embedded_toml`) as base.
/// 2. If any `xiuxian.toml` exists in `PRJ_CONFIG_HOME`, merge `[spec.namespace]`
///    from each candidate after resolving recursive `imports`.
/// 3. If no `xiuxian.toml` exists, merge standalone orphan file(s) as fallback.
///
/// # Errors
///
/// Returns [`ConfigCoreError`] on parse/read failure or `SSoT` conflict.
pub fn resolve_and_merge_toml(spec: ConfigCascadeSpec<'_>) -> Result<toml::Value, ConfigCoreError> {
    let project_root = resolve_project_root();
    let config_home = resolve_config_home(project_root.as_deref());
    resolve_and_merge_toml_with_paths(spec, project_root.as_deref(), config_home.as_deref())
}

/// Resolve layered files and return merged TOML value with explicit paths.
///
/// This is intended for deterministic testing and runtime call sites that already
/// resolved `project_root` and `config_home`.
///
/// # Errors
///
/// Returns [`ConfigCoreError`] on parse/read failure or `SSoT` conflict.
pub fn resolve_and_merge_toml_with_paths(
    spec: ConfigCascadeSpec<'_>,
    project_root: Option<&Path>,
    config_home: Option<&Path>,
) -> Result<toml::Value, ConfigCoreError> {
    let resolved_config_home = normalize_config_home(project_root, config_home);
    let mut global_paths =
        existing_config_files(global_candidates(resolved_config_home.as_deref()));
    let mut orphan_paths = existing_config_files(orphan_candidates(
        resolved_config_home.as_deref(),
        spec.orphan_file,
    ));
    global_paths.sort();
    orphan_paths.sort();
    global_paths.dedup();
    orphan_paths.dedup();

    if !global_paths.is_empty() && !orphan_paths.is_empty() {
        let orphans = orphan_paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(ConfigCoreError::RedundantOrphan {
            namespace: spec.namespace.to_string(),
            orphans,
        });
    }

    let embedded_source_path = spec.embedded_source_path.map(Path::new);
    let mut merged = imports::load_embedded_with_imports(
        spec.namespace,
        spec.embedded_toml,
        embedded_source_path,
        spec.array_merge_strategy,
    )?;

    if global_paths.is_empty() {
        for orphan_path in orphan_paths {
            let orphan_value =
                imports::load_file_with_imports(orphan_path.as_path(), spec.array_merge_strategy)?;
            merge_values(&mut merged, orphan_value, spec.array_merge_strategy);
        }
    } else {
        for path in global_paths {
            let global_root =
                imports::load_file_with_imports(path.as_path(), spec.array_merge_strategy)?;
            if let Some(namespace_value) = get_nested_value(&global_root, spec.namespace) {
                merge_values(&mut merged, namespace_value, spec.array_merge_strategy);
            }
        }
    }
    Ok(merged)
}

/// Resolve layered files and deserialize merged config into target type.
///
/// # Errors
///
/// Returns [`ConfigCoreError`] on resolve/merge failure or deserialize failure.
pub fn resolve_and_load<T>(spec: ConfigCascadeSpec<'_>) -> Result<T, ConfigCoreError>
where
    T: DeserializeOwned,
{
    let merged = resolve_and_merge_toml(spec)?;
    merged
        .try_into()
        .map_err(|source| ConfigCoreError::DeserializeMerged {
            namespace: spec.namespace.to_string(),
            source,
        })
}

/// Resolve layered files and deserialize merged config using explicit paths.
///
/// # Errors
///
/// Returns [`ConfigCoreError`] on resolve/merge failure or deserialize failure.
pub fn resolve_and_load_with_paths<T>(
    spec: ConfigCascadeSpec<'_>,
    project_root: Option<&Path>,
    config_home: Option<&Path>,
) -> Result<T, ConfigCoreError>
where
    T: DeserializeOwned,
{
    let merged = resolve_and_merge_toml_with_paths(spec, project_root, config_home)?;
    merged
        .try_into()
        .map_err(|source| ConfigCoreError::DeserializeMerged {
            namespace: spec.namespace.to_string(),
            source,
        })
}

/// Traverse a dotted path in a TOML value to get a nested value.
///
/// For example, `get_nested_value(&value, "llm.vision.deepseek")` will traverse
/// `value["llm"]["vision"]["deepseek"]`.
fn get_nested_value(value: &toml::Value, dotted_path: &str) -> Option<toml::Value> {
    if dotted_path.trim().is_empty() {
        return Some(value.clone());
    }

    let mut current = value;
    for key in dotted_path.split('.') {
        match current {
            toml::Value::Table(table) => {
                current = table.get(key)?;
            }
            _ => return None,
        }
    }
    Some(current.clone())
}

fn resolve_project_root() -> Option<PathBuf> {
    if let Some(path) = std::env::var("PRJ_ROOT")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        let candidate = PathBuf::from(path);
        if candidate.is_absolute() {
            return Some(candidate);
        }
        if let Ok(current_dir) = std::env::current_dir() {
            return Some(current_dir.join(candidate));
        }
        return None;
    }

    let mut cursor = std::env::current_dir().ok()?;
    loop {
        if cursor.join(".git").exists() {
            return Some(cursor);
        }
        if !cursor.pop() {
            break;
        }
    }
    None
}

fn resolve_config_home(project_root: Option<&Path>) -> Option<PathBuf> {
    std::env::var("PRJ_CONFIG_HOME")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .map(|path| {
            if path.is_absolute() {
                path
            } else if let Some(root) = project_root {
                root.join(path)
            } else {
                path
            }
        })
        .or_else(|| project_root.map(|root| root.join(".config")))
}

fn normalize_config_home(
    project_root: Option<&Path>,
    config_home: Option<&Path>,
) -> Option<PathBuf> {
    match config_home {
        Some(path) if path.is_absolute() => Some(path.to_path_buf()),
        Some(path) => project_root.map(|root| root.join(path)),
        None => project_root.map(|root| root.join(".config")),
    }
}
