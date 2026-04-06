use std::fs::read_to_string;
use std::path::Path;

use super::regex::{RE_DEP, RE_EXACT_DEP, RE_SIMPLE};
use super::types::PyprojectDependency;

/// Parse dependencies from a `pyproject.toml` file.
///
/// # Errors
///
/// Returns I/O errors when the pyproject file cannot be read.
pub fn parse_pyproject_dependencies(
    path: &Path,
) -> Result<Vec<PyprojectDependency>, std::io::Error> {
    let content = read_to_string(path)?;

    let mut deps = Vec::new();

    if let Ok(toml) = content.parse::<toml::Value>() {
        if let Some(dependencies) = toml
            .get("project")
            .and_then(|project| project.get("dependencies"))
            && let Some(dep_array) = dependencies.as_array()
        {
            for dep in dep_array {
                if let Some(dep_str) = dep.as_str()
                    && let Some((name, version)) = parse_pyproject_dep(dep_str)
                {
                    deps.push(PyprojectDependency::new(name, Some(version)));
                }
            }
        }
    } else {
        for cap in RE_DEP.captures_iter(&content) {
            let name = cap[1].to_string();
            let version = cap[2].trim().to_string();
            deps.push(PyprojectDependency::new(name, Some(version)));
        }
    }

    Ok(deps)
}

fn parse_pyproject_dep(dep: &str) -> Option<(String, String)> {
    RE_EXACT_DEP
        .captures(dep)
        .map(|cap| (cap[1].to_string(), cap[2].to_string()))
        .or_else(|| {
            RE_SIMPLE
                .captures(dep)
                .map(|cap| (cap[1].to_string(), "latest".to_string()))
        })
}
