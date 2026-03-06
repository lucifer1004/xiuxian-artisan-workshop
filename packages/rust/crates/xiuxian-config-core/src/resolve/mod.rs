mod discover;
mod io;
mod merge;
mod namespace;

use crate::cache::{build_file_stamps, cache_key, store_cached_merged, try_get_cached_merged};
use crate::paths::{normalize_config_home, resolve_config_home, resolve_project_root};
use crate::{ConfigCascadeSpec, ConfigCoreError};
use discover::{existing_config_files, global_candidates, orphan_candidates, tracked_files};
use io::read_toml;
use merge::merge_values;
use namespace::extract_namespace_value;
use serde::de::DeserializeOwned;
use std::path::Path;

/// Resolve layered files and return merged TOML value.
///
/// Merge order:
/// 1. Embedded defaults (`spec.embedded_toml`) as base.
/// 2. If any `xiuxian.toml` exists in `PRJ_CONFIG_HOME`, merge `[spec.namespace]` from each candidate.
/// 3. If no `xiuxian.toml` exists, merge standalone orphan file(s) as fallback.
///
/// This resolver uses an internal read-through cache keyed by namespace/spec/path
/// and invalidated by file metadata stamps.
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

    let tracked_files = tracked_files(&global_paths, &orphan_paths);
    let file_stamps = build_file_stamps(tracked_files.as_slice());
    let key = cache_key(spec, resolved_config_home.as_deref());
    if let Some(cached) = try_get_cached_merged(&key, file_stamps.as_slice()) {
        return Ok(cached);
    }

    let mut merged: toml::Value =
        toml::from_str(spec.embedded_toml).map_err(|source| ConfigCoreError::ParseEmbedded {
            namespace: spec.namespace.to_string(),
            source,
        })?;

    if global_paths.is_empty() {
        for orphan_path in orphan_paths {
            let orphan_value = read_toml(orphan_path.as_path())?;
            merge_values(&mut merged, orphan_value, spec.array_merge_strategy);
        }
    } else {
        for path in global_paths {
            let global_root = read_toml(path.as_path())?;
            if let Some(namespace_value) = extract_namespace_value(&global_root, spec.namespace) {
                merge_values(&mut merged, namespace_value, spec.array_merge_strategy);
            }
        }
    }

    store_cached_merged(key, file_stamps, &merged);
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
    deserialize_merged(merged, spec.namespace)
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
    deserialize_merged(merged, spec.namespace)
}

fn deserialize_merged<T>(merged: toml::Value, namespace: &str) -> Result<T, ConfigCoreError>
where
    T: DeserializeOwned,
{
    merged
        .try_into()
        .map_err(|source| ConfigCoreError::DeserializeMerged {
            namespace: namespace.to_string(),
            source,
        })
}
