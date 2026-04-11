type TestResult = Result<(), Box<dyn std::error::Error>>;

use std::fs;
use std::path::Path;

use serde_json::json;
use tempfile::TempDir;
use xiuxian_wendao_core::repo_intelligence::RepositoryPluginConfig;
use xiuxian_wendao_core::repo_intelligence::{AnalysisContext, RegisteredRepository};

use super::{analyze_repository, preflight_repository};
use crate::julia_plugin_test_support::common::ensure_linked_modelica_parser_summary_service;

#[test]
fn analyze_repository_keeps_top_level_package_paths() -> TestResult {
    ensure_linked_modelica_parser_summary_service()?;
    let tempdir = TempDir::new()?;
    write_modelica_file(
        tempdir.path().join("package.mo").as_path(),
        "within ;\npackage DemoLib\nend DemoLib;\n",
    )?;
    write_modelica_file(
        tempdir.path().join("Example.mo").as_path(),
        "within DemoLib;\nmodel Example\nend Example;\n",
    )?;

    let output = analyze_repository(&analysis_context("demo", tempdir.path()), tempdir.path())?;

    assert!(
        output
            .modules
            .iter()
            .any(|module| module.path == "package.mo" && module.qualified_name == "DemoLib")
    );
    assert!(
        output
            .symbols
            .iter()
            .any(|symbol| symbol.path == "Example.mo" && symbol.qualified_name == "DemoLib.Example")
    );
    assert_eq!(output.diagnostics[0].path, "package.mo");
    Ok(())
}

#[test]
fn analyze_repository_supports_dominant_nested_root_package() -> TestResult {
    ensure_linked_modelica_parser_summary_service()?;
    let tempdir = TempDir::new()?;
    write_modelica_file(
        tempdir.path().join("Modelica/package.mo").as_path(),
        "within ;\npackage Modelica\nend Modelica;\n",
    )?;
    write_modelica_file(
        tempdir.path().join("Modelica/Blocks.mo").as_path(),
        "within Modelica;\nmodel Blocks\nend Blocks;\n",
    )?;
    write_modelica_file(
        tempdir.path().join("ModelicaServices/package.mo").as_path(),
        "within ;\npackage ModelicaServices\nend ModelicaServices;\n",
    )?;

    preflight_repository(&analysis_context("mcl", tempdir.path()), tempdir.path())?;
    let output = analyze_repository(&analysis_context("mcl", tempdir.path()), tempdir.path())?;

    assert!(
        output
            .modules
            .iter()
            .any(|module| module.path == "Modelica/package.mo"
                && module.qualified_name == "Modelica")
    );
    assert!(
        output
            .symbols
            .iter()
            .any(|symbol| symbol.path == "Modelica/Blocks.mo"
                && symbol.qualified_name == "Modelica.Blocks")
    );
    assert_eq!(output.diagnostics[0].path, "Modelica/package.mo");
    Ok(())
}

#[test]
fn analyze_repository_preserves_import_backed_package_attributes() -> TestResult {
    ensure_linked_modelica_parser_summary_service()?;
    let tempdir = TempDir::new()?;
    write_modelica_file(
        tempdir.path().join("Modelica/package.mo").as_path(),
        "within ;\npackage Modelica\nend Modelica;\n",
    )?;
    write_modelica_file(
        tempdir.path().join("Modelica/Blocks/package.mo").as_path(),
        "within Modelica;\npackage Blocks\n  import SI = Modelica.Units.SI;\n  import Modelica.Math;\n  import Modelica.Math.*;\nend Blocks;\n",
    )?;

    let output = analyze_repository(&analysis_context("mcl", tempdir.path()), tempdir.path())?;
    let payload = output
        .imports
        .iter()
        .map(|import| {
            json!({
                "module_id": import.module_id,
                "import_name": import.import_name,
                "target_package": import.target_package,
                "source_module": import.source_module,
                "kind": format!("{:?}", import.kind),
                "line_start": import.line_start,
                "resolved_id": import.resolved_id,
                "attributes": import.attributes,
            })
        })
        .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "analyze_repository_preserves_import_backed_package_attributes",
        payload
    );
    Ok(())
}

fn analysis_context(repo_id: &str, repository_root: &Path) -> AnalysisContext {
    AnalysisContext {
        repository: RegisteredRepository {
            id: repo_id.to_string(),
            plugins: vec![RepositoryPluginConfig::Id("modelica".to_string())],
            ..RegisteredRepository::default()
        },
        repository_root: repository_root.to_path_buf(),
    }
}

fn write_modelica_file(path: &Path, contents: &str) -> TestResult {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}
