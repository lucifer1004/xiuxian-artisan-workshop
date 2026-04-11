type TestResult = Result<(), Box<dyn std::error::Error>>;

use std::fs;
use std::path::Path;

use tempfile::TempDir;
use xiuxian_wendao_core::repo_intelligence::{
    AnalysisContext, RegisteredRepository, RepoIntelligencePlugin, RepoSourceFile,
    RepositoryPluginConfig,
};

use super::ModelicaRepoIntelligencePlugin;
use crate::julia_plugin_test_support::common::{
    ensure_linked_modelica_parser_summary_service, repo_root,
};

#[test]
fn analyze_file_emits_modelica_module_and_symbols() -> TestResult {
    ensure_linked_modelica_parser_summary_service()?;
    let tempdir = TempDir::new()?;
    let plugin = ModelicaRepoIntelligencePlugin;
    let output = plugin.analyze_file(
        &analysis_context("demo", tempdir.path()),
        &RepoSourceFile {
            path: "Controllers/PI.mo".to_string(),
            contents: "within Demo.Controllers;\nmodel PI\n  parameter Real k = 1;\n  parameter Real Ti = 0.1;\n  Real y;\nequation\n  y = k;\nend PI;\n".to_string(),
        },
    )?;

    assert!(
        output
            .modules
            .iter()
            .any(|module| module.path == "Controllers/PI.mo" && module.qualified_name == "PI")
    );
    assert!(
        output.symbols.iter().any(|symbol| {
            symbol.path == "Controllers/PI.mo"
                && symbol.qualified_name == "PI"
                && symbol.name == "PI"
                && symbol.module_id.as_deref() == Some("repo:demo:module:PI")
        }),
        "symbols: {:?}",
        output.symbols
    );
    let model = output
        .symbols
        .iter()
        .find(|symbol| symbol.name == "PI")
        .unwrap_or_else(|| panic!("symbols: {:?}", output.symbols));
    assert!(
        model
            .attributes
            .get("class_name")
            .is_some_and(|value| value == "PI"),
        "model attrs: {:?}",
        model.attributes
    );
    assert!(
        model
            .attributes
            .get("restriction")
            .is_some_and(|value| value == "model"),
        "model attrs: {:?}",
        model.attributes
    );
    assert!(
        model
            .attributes
            .get("top_level")
            .is_some_and(|value| value == "true"),
        "model attrs: {:?}",
        model.attributes
    );
    Ok(())
}

#[test]
fn analyze_file_supports_modelica_standard_library_package_via_process_managed_parser_summary()
-> TestResult {
    if std::env::var_os("RUN_PROCESS_MANAGED_WENDAOSEARCH_TEST").is_none() {
        eprintln!("skipping process-managed Modelica analyze_file proof");
        return Ok(());
    }

    let source_path = repo_root().join(
        ".data/xiuxian-wendao/repo-intelligence/repos/github.com/modelica/ModelicaStandardLibrary/Modelica/Blocks/package.mo",
    );
    if !source_path.is_file() {
        eprintln!(
            "skipping process-managed Modelica analyze_file proof; missing {}",
            source_path.display()
        );
        return Ok(());
    }

    let tempdir = TempDir::new()?;
    let plugin = ModelicaRepoIntelligencePlugin;
    let output = plugin.analyze_file(
        &analysis_context("mcl-live", tempdir.path()),
        &RepoSourceFile {
            path: "Modelica/Blocks/package.mo".to_string(),
            contents: fs::read_to_string(&source_path)?,
        },
    )?;

    assert!(
        output
            .modules
            .iter()
            .any(|module| module.path == "Modelica/Blocks/package.mo"
                && module.qualified_name == "Blocks"),
        "modules: {:?}",
        output.modules
    );
    assert!(
        output
            .symbols
            .iter()
            .any(|symbol| symbol.path == "Modelica/Blocks/package.mo" && symbol.name == "Init"),
        "symbols: {:?}",
        output.symbols
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
