use super::merge::merge_values;
use crate::{ArrayMergeStrategy, ConfigCoreError};
use std::path::{Path, PathBuf};

pub(super) fn load_file_with_imports(
    path: &Path,
    array_merge_strategy: ArrayMergeStrategy,
) -> Result<toml::Value, ConfigCoreError> {
    let content = std::fs::read_to_string(path).map_err(|source| ConfigCoreError::ReadFile {
        path: path.display().to_string(),
        source,
    })?;
    let value =
        toml::from_str::<toml::Value>(&content).map_err(|source| ConfigCoreError::ParseFile {
            path: path.display().to_string(),
            source,
        })?;
    let mut stack = Vec::new();
    load_value_with_imports(value, Some(path), array_merge_strategy, &mut stack)
}

pub(super) fn load_embedded_with_imports(
    namespace: &str,
    embedded_toml: &str,
    embedded_source_path: Option<&Path>,
    array_merge_strategy: ArrayMergeStrategy,
) -> Result<toml::Value, ConfigCoreError> {
    let value = toml::from_str::<toml::Value>(embedded_toml).map_err(|source| {
        ConfigCoreError::ParseEmbedded {
            namespace: namespace.to_string(),
            source,
        }
    })?;
    let mut stack = Vec::new();
    load_value_with_imports(
        value,
        embedded_source_path,
        array_merge_strategy,
        &mut stack,
    )
}

/// Load one TOML file and recursively resolve any nested `imports`.
///
/// # Errors
///
/// Returns [`ConfigCoreError`] when the file cannot be read, parsed, or when
/// import resolution fails.
pub fn load_toml_value_with_imports(path: &Path) -> Result<toml::Value, ConfigCoreError> {
    load_file_with_imports(path, ArrayMergeStrategy::Overwrite)
}

fn load_value_with_imports(
    value: toml::Value,
    source_path: Option<&Path>,
    array_merge_strategy: ArrayMergeStrategy,
    stack: &mut Vec<PathBuf>,
) -> Result<toml::Value, ConfigCoreError> {
    match value {
        toml::Value::Table(table) => {
            load_table_with_imports(table, source_path, array_merge_strategy, stack)
        }
        toml::Value::Array(values) => {
            let mut resolved = Vec::with_capacity(values.len());
            for item in values {
                resolved.push(load_value_with_imports(
                    item,
                    source_path,
                    array_merge_strategy,
                    stack,
                )?);
            }
            Ok(toml::Value::Array(resolved))
        }
        other => Ok(other),
    }
}

fn load_table_with_imports(
    mut table: toml::map::Map<String, toml::Value>,
    source_path: Option<&Path>,
    array_merge_strategy: ArrayMergeStrategy,
    stack: &mut Vec<PathBuf>,
) -> Result<toml::Value, ConfigCoreError> {
    let import_paths = extract_import_paths(&mut table, source_path)?;
    let mut merged = toml::Value::Table(toml::map::Map::new());

    for import_path in import_paths {
        let imported = load_imported_value(import_path.as_path(), array_merge_strategy, stack)?;
        merge_values(&mut merged, imported, array_merge_strategy);
    }

    let mut resolved_table = toml::map::Map::new();
    for (key, value) in table {
        let resolved = load_value_with_imports(value, source_path, array_merge_strategy, stack)?;
        resolved_table.insert(key, resolved);
    }
    merge_values(
        &mut merged,
        toml::Value::Table(resolved_table),
        array_merge_strategy,
    );

    Ok(merged)
}

fn extract_import_paths(
    table: &mut toml::map::Map<String, toml::Value>,
    source_path: Option<&Path>,
) -> Result<Vec<PathBuf>, ConfigCoreError> {
    let Some(imports_value) = table.remove("imports") else {
        return Ok(Vec::new());
    };

    if source_path.is_none() {
        return Err(ConfigCoreError::InvalidImports {
            path: "<embedded>".to_string(),
            message: "embedded TOML with imports requires a source path".to_string(),
        });
    }

    let Some(imports_array) = imports_value.as_array() else {
        return Err(ConfigCoreError::InvalidImports {
            path: source_path.map_or_else(
                || "<embedded>".to_string(),
                |path| path.display().to_string(),
            ),
            message: "`imports` must be an array of relative or absolute TOML file paths"
                .to_string(),
        });
    };

    let mut import_paths = Vec::with_capacity(imports_array.len());
    for entry in imports_array {
        let Some(raw_path) = entry.as_str() else {
            return Err(ConfigCoreError::InvalidImports {
                path: source_path.map_or_else(
                    || "<embedded>".to_string(),
                    |path| path.display().to_string(),
                ),
                message: "`imports` entries must be strings".to_string(),
            });
        };

        let trimmed = raw_path.trim();
        if trimmed.is_empty() {
            continue;
        }

        let resolved = resolve_import_path(source_path, trimmed);
        import_paths.push(resolved);
    }

    Ok(import_paths)
}

fn resolve_import_path(source_path: Option<&Path>, import_path: &str) -> PathBuf {
    let candidate = Path::new(import_path);
    if candidate.is_absolute() {
        return candidate.to_path_buf();
    }

    source_path
        .and_then(Path::parent)
        .map_or_else(|| candidate.to_path_buf(), |base| base.join(candidate))
}

fn load_imported_value(
    path: &Path,
    array_merge_strategy: ArrayMergeStrategy,
    stack: &mut Vec<PathBuf>,
) -> Result<toml::Value, ConfigCoreError> {
    let normalized_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if stack.contains(&normalized_path) {
        let mut chain = stack
            .iter()
            .map(|entry| entry.display().to_string())
            .collect::<Vec<_>>();
        chain.push(normalized_path.display().to_string());
        return Err(ConfigCoreError::ImportCycle {
            chain: chain.join(" -> "),
        });
    }

    stack.push(normalized_path);
    let content = std::fs::read_to_string(path).map_err(|source| ConfigCoreError::ReadFile {
        path: path.display().to_string(),
        source,
    })?;
    let value =
        toml::from_str::<toml::Value>(&content).map_err(|source| ConfigCoreError::ParseFile {
            path: path.display().to_string(),
            source,
        })?;
    let resolved = load_value_with_imports(value, Some(path), array_merge_strategy, stack);
    stack.pop();
    resolved
}
