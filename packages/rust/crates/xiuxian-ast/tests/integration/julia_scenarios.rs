//! Scenario-based Julia parser snapshots for `xiuxian-ast`.

use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use xiuxian_ast::TreeSitterJuliaParser;
use xiuxian_testing::{Scenario, ScenarioFramework, ScenarioRunner, ScenarioSnapshotPolicy};

struct JuliaParserRunner;

impl ScenarioRunner for JuliaParserRunner {
    fn category(&self) -> &'static str {
        "julia_parser"
    }

    fn run(&self, scenario: &Scenario, temp_dir: &Path) -> Result<Value, Box<dyn Error>> {
        let source_path = temp_dir.join("source.jl");
        let source = fs::read_to_string(&source_path)?;

        let mut parser = TreeSitterJuliaParser::new()?;
        match scenario.config.input.input_type.as_str() {
            "julia_root_source" => {
                let summary = parser.parse_summary(&source)?;
                Ok(json!({
                    "module_name": summary.module_name,
                    "exports": summary.exports,
                    "imports": summary.imports.iter().map(import_json).collect::<Vec<_>>(),
                    "symbols": summary.symbols.iter().map(symbol_json).collect::<Vec<_>>(),
                    "docstrings": summary.docstrings.iter().map(docstring_json).collect::<Vec<_>>(),
                    "includes": summary.includes,
                }))
            }
            "julia_file_source" => {
                let summary = parser.parse_file_summary(&source)?;
                Ok(json!({
                    "module_name": summary.module_name,
                    "exports": summary.exports,
                    "imports": summary.imports.iter().map(import_json).collect::<Vec<_>>(),
                    "symbols": summary.symbols.iter().map(symbol_json).collect::<Vec<_>>(),
                    "docstrings": summary.docstrings.iter().map(docstring_json).collect::<Vec<_>>(),
                    "includes": summary.includes,
                }))
            }
            other => Err(format!("unsupported Julia scenario input type `{other}`").into()),
        }
    }
}

fn import_json(import: &xiuxian_ast::JuliaImport) -> Value {
    json!({
        "module": import.module,
        "reexported": import.reexported,
    })
}

fn symbol_json(symbol: &xiuxian_ast::JuliaSymbol) -> Value {
    json!({
        "name": symbol.name,
        "kind": format!("{:?}", symbol.kind).to_ascii_lowercase(),
        "signature": symbol.signature,
    })
}

fn docstring_json(docstring: &xiuxian_ast::JuliaDocAttachment) -> Value {
    json!({
        "target_name": docstring.target_name,
        "target_kind": format!("{:?}", docstring.target_kind).to_ascii_lowercase(),
        "content": docstring.content,
    })
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn test_julia_parser_scenarios() {
    let manifest = manifest_dir();
    let scenarios_root = manifest.join("tests").join("fixtures").join("scenarios");
    let snapshot_path = manifest.join("tests").join("snapshots");

    let mut framework = ScenarioFramework::with_snapshot_path(&snapshot_path)
        .with_snapshot_policy(ScenarioSnapshotPolicy::portable_ci());
    framework.register(Box::new(JuliaParserRunner));
    let count = framework
        .run_category_at("julia_parser", &scenarios_root)
        .unwrap_or_else(|error| panic!("julia parser scenarios should pass: {error}"));
    assert!(count >= 2, "should run Julia parser scenarios");
}
