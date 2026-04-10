use serde_json::json;
use xiuxian_wendao_core::repo_intelligence::{RegisteredRepository, RepositoryPluginConfig};

use super::parse_symbol_declarations_for_repository;
use crate::julia_plugin_test_support::common::ensure_linked_modelica_parser_summary_service;

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
