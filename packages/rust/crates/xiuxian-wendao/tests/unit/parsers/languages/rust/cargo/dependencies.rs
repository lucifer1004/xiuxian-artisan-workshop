use crate::parsers::languages::rust::cargo::dependencies::parse_cargo_dependencies;
use std::io::Write as StdWrite;
use tempfile::NamedTempFile;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn parse_cargo_dependencies_prefers_workspace_dependencies() -> TestResult {
    let content = r#"
[workspace]
members = ["crates/*"]

[workspace.dependencies]
tokio = { version = "1.49.0", features = ["full"] }
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.149"
anyhow = "1.0.100"
thiserror = "2.0.17"
"#;

    let mut file = NamedTempFile::new()?;
    file.write_all(content.as_bytes())?;

    let deps = parse_cargo_dependencies(file.path())?;

    assert!(deps.iter().any(|dep| dep.name == "tokio"));
    assert!(deps.iter().any(|dep| dep.name == "serde"));
    assert!(deps.iter().any(|dep| dep.name == "anyhow"));
    assert_eq!(
        deps.iter()
            .find(|dep| dep.name == "serde")
            .map(|dep| dep.version.as_str()),
        Some("1.0.228")
    );
    Ok(())
}

#[test]
fn parse_cargo_dependencies_reads_regular_dependencies() -> TestResult {
    let content = r#"
[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = "1.0"
anyhow = "1.0"
thiserror = "1.0"
"#;

    let mut file = NamedTempFile::new()?;
    file.write_all(content.as_bytes())?;

    let deps = parse_cargo_dependencies(file.path())?;

    assert!(deps.iter().any(|dep| dep.name == "serde"));
    assert!(deps.iter().any(|dep| dep.name == "anyhow"));
    Ok(())
}

#[test]
fn parse_cargo_dependencies_falls_back_to_regular_dependencies_for_workspace_root() -> TestResult {
    let content = r#"
[workspace]
members = ["crates/*"]

[dependencies]
serde = "1.0"
anyhow = "1.0"
"#;

    let mut file = NamedTempFile::new()?;
    file.write_all(content.as_bytes())?;

    let deps = parse_cargo_dependencies(file.path())?;

    assert_eq!(deps.len(), 2);
    assert_eq!(deps[0].name, "serde");
    assert_eq!(deps[1].name, "anyhow");
    Ok(())
}
