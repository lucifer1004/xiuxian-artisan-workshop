//! Scenario-based Modelica parser snapshots for `xiuxian-ast`.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use xiuxian_ast::{ModelicaFileSummary, TreeSitterModelicaParser};
use xiuxian_testing::{Scenario, ScenarioFramework, ScenarioRunner, ScenarioSnapshotPolicy};

struct ModelicaParserRunner;

impl ScenarioRunner for ModelicaParserRunner {
    fn category(&self) -> &str {
        "modelica_parser"
    }

    fn run(&self, scenario: &Scenario, temp_dir: &Path) -> Result<Value, Box<dyn Error>> {
        let source_path = temp_dir.join("source.mo");
        let source = fs::read_to_string(&source_path)?;

        let mut parser = TreeSitterModelicaParser::new()?;
        match scenario.config.input.input_type.as_str() {
            "modelica_file_source" => {
                let summary = parser.parse_file_summary(&source)?;
                Ok(summary_to_json(summary))
            }
            other => Err(format!("unsupported Modelica scenario input type `{other}`").into()),
        }
    }
}

fn summary_to_json(summary: ModelicaFileSummary) -> Value {
    json!({
        "class_name": summary.class_name,
        "imports": summary.imports.into_iter().map(import_json).collect::<Vec<_>>(),
        "symbols": summary.symbols.into_iter().map(symbol_json).collect::<Vec<_>>(),
        "documentation": summary.documentation,
    })
}

fn import_json(import: xiuxian_ast::ModelicaImport) -> Value {
    json!({
        "name": import.name,
        "alias": import.alias,
    })
}

fn symbol_json(symbol: xiuxian_ast::ModelicaSymbol) -> Value {
    json!({
        "name": symbol.name,
        "kind": format!("{:?}", symbol.kind).to_ascii_lowercase(),
        "signature": symbol.signature,
    })
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn test_modelica_parser_scenarios() {
    // Skip gracefully if tree-sitter-modelica library is not available
    if TreeSitterModelicaParser::new().is_err() {
        eprintln!("Skipping test: tree-sitter-modelica library not found");
        return;
    }

    let manifest = manifest_dir();
    let scenarios_root = manifest.join("tests").join("fixtures").join("scenarios");
    let snapshot_path = manifest.join("tests").join("snapshots");

    let mut framework = ScenarioFramework::with_snapshot_path(&snapshot_path)
        .with_snapshot_policy(ScenarioSnapshotPolicy::portable_ci());
    framework.register(Box::new(ModelicaParserRunner));

    // Run only modelica_parser category scenarios
    let count = framework
        .run_category_at("modelica_parser", &scenarios_root)
        .unwrap_or_else(|error| panic!("modelica parser scenarios should pass: {error}"));
    assert!(
        count >= 2,
        "should run at least 2 Modelica parser scenarios"
    );
}
