use std::io::Write as StdWrite;

use tempfile::NamedTempFile;

use crate::parsers::languages::python::pyproject::dependencies::parse_pyproject_dependencies;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn parse_pyproject_dependencies_reads_project_dependency_array() -> TestResult {
    let content = r#"
[project]
name = "demo"
version = "0.1.0"
dependencies = [
    "requests>=2.0",
    "click>=8.0",
    "rich>=13.0",
]
"#;

    let mut file = NamedTempFile::new()?;
    file.write_all(content.as_bytes())?;

    let deps = parse_pyproject_dependencies(file.path())?;

    assert!(deps.iter().any(|dep| dep.name == "requests"));
    assert!(deps.iter().any(|dep| dep.name == "click"));
    assert!(deps.iter().any(|dep| dep.name == "rich"));
    Ok(())
}

#[test]
fn parse_pyproject_dependencies_falls_back_to_regex_for_invalid_toml() -> TestResult {
    let content = "package1==1.0.0\npackage2>=2.0.0\nanother_package[extra]==5.0.0\n";

    let mut file = NamedTempFile::new()?;
    file.write_all(content.as_bytes())?;

    let deps = parse_pyproject_dependencies(file.path())?;

    assert_eq!(deps.len(), 3);
    assert_eq!(deps[0].name, "package1");
    assert_eq!(deps[0].version.as_deref(), Some("1.0.0"));
    Ok(())
}

#[test]
fn parse_pyproject_dependencies_defaults_unpinned_packages_to_latest() -> TestResult {
    let content = r#"
[project]
name = "demo"
version = "0.1.0"
dependencies = [
    "uvicorn",
]
"#;

    let mut file = NamedTempFile::new()?;
    file.write_all(content.as_bytes())?;

    let deps = parse_pyproject_dependencies(file.path())?;

    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].name, "uvicorn");
    assert_eq!(deps[0].version.as_deref(), Some("latest"));
    Ok(())
}
