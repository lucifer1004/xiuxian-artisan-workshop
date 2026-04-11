use std::fs;

use serde_json::json;
use xiuxian_wendao_core::repo_intelligence::{RegisteredRepository, RepositoryPluginConfig};

use super::{parse_imports_for_repository, parse_symbol_declarations_for_repository};
use crate::julia_plugin_test_support::common::{
    ensure_linked_modelica_parser_summary_service, repo_root,
};
use crate::modelica_plugin::parser_summary::fetch_modelica_parser_file_summary_blocking_for_repository;

#[test]
fn parse_symbol_declarations_supports_secondary_keywords() -> Result<(), Box<dyn std::error::Error>>
{
    ensure_linked_modelica_parser_summary_service()?;
    let repository = RegisteredRepository {
        id: "modelica-parsing".to_string(),
        plugins: vec![RepositoryPluginConfig::Id("modelica".to_string())],
        ..RegisteredRepository::default()
    };
    let payload = parse_symbol_declarations_for_repository(
        &repository,
        "SecondaryKeywords.mo",
        r"
within;
package SecondaryKeywords
  record ControllerState
  end ControllerState;

  model GainHolder
    parameter Real Gain = 1;
  end GainHolder;

  block Limiter
  end Limiter;
end SecondaryKeywords;
",
    )?
    .into_iter()
    .map(|declaration| {
        json!({
            "name": declaration.name,
            "kind": format!("{:?}", declaration.kind),
            "signature": declaration.signature,
            "line_start": declaration.line_start,
            "equations_count": declaration.equations.len(),
        })
    })
    .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "parse_symbol_declarations_supports_secondary_keywords",
        payload
    );
    Ok(())
}

#[test]
fn parse_imports_preserves_modelica_package_import_attributes()
-> Result<(), Box<dyn std::error::Error>> {
    ensure_linked_modelica_parser_summary_service()?;
    let repository = RegisteredRepository {
        id: "modelica-imports".to_string(),
        plugins: vec![RepositoryPluginConfig::Id("modelica".to_string())],
        ..RegisteredRepository::default()
    };
    let payload = parse_imports_for_repository(
        &repository,
        "Modelica/Blocks/package.mo",
        r"
within Modelica;
package Blocks
  import SI = Modelica.Units.SI;
  import Modelica.Math;
  import Modelica.Math.*;
end Blocks;
",
    )?
    .into_iter()
    .map(|import| {
        json!({
            "name": import.name,
            "alias": import.alias,
            "kind": format!("{:?}", import.kind),
            "line_start": import.line_start,
            "attributes": import.attributes,
        })
    })
    .collect::<Vec<_>>();

    insta::assert_json_snapshot!(
        "parse_imports_preserves_modelica_package_import_attributes",
        payload
    );
    Ok(())
}

#[test]
fn fetch_modelica_standard_library_package_summary_via_process_managed_parser_summary()
-> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("RUN_PROCESS_MANAGED_WENDAOSEARCH_TEST").is_none() {
        eprintln!("skipping process-managed Modelica parser-summary proof");
        return Ok(());
    }

    let source_path = repo_root().join(
        ".data/xiuxian-wendao/repo-intelligence/repos/github.com/modelica/ModelicaStandardLibrary/Modelica/Blocks/package.mo",
    );
    if !source_path.is_file() {
        eprintln!(
            "skipping process-managed Modelica parser-summary proof; missing {}",
            source_path.display()
        );
        return Ok(());
    }

    let repository = RegisteredRepository {
        id: "mcl-live".to_string(),
        plugins: vec![RepositoryPluginConfig::Id("modelica".to_string())],
        ..RegisteredRepository::default()
    };
    let source_text = fs::read_to_string(&source_path)?;
    let summary = fetch_modelica_parser_file_summary_blocking_for_repository(
        &repository,
        "Modelica/Blocks/package.mo",
        &source_text,
    )?;
    let class_name = summary.class_name.clone();
    let imports = summary.imports;
    let declarations = summary.declarations;

    assert!(
        !imports.is_empty(),
        "expected Modelica Standard Library package imports"
    );
    assert_eq!(class_name.as_deref(), Some("Blocks"));
    assert!(
        declarations.iter().any(|declaration| declaration.name == "Blocks"),
        "expected top-level Blocks declaration"
    );
    assert!(
        declarations.iter().any(|declaration| declaration.name == "Init"),
        "expected nested Init declaration from package.mo"
    );
    Ok(())
}
