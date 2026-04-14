use xiuxian_wendao_core::repo_intelligence::{RegisteredRepository, RepositoryPluginConfig};

use super::{
    fetch_modelica_parser_file_summary_blocking_for_repository,
    shared_modelica_parser_summary_runtime_identity_for_tests,
};
use crate::julia_plugin_test_support::common::ensure_linked_modelica_parser_summary_service;

fn parser_summary_repository() -> RegisteredRepository {
    RegisteredRepository {
        id: "repo-modelica".to_string(),
        plugins: vec![RepositoryPluginConfig::Id("modelica".to_string())],
        ..RegisteredRepository::default()
    }
}

#[test]
#[serial_test::serial(modelica_live)]
fn blocking_fetch_reuses_shared_runtime_and_returns_summary_from_linked_service()
-> Result<(), Box<dyn std::error::Error>> {
    ensure_linked_modelica_parser_summary_service()?;
    let repository = parser_summary_repository();
    let runtime_before = shared_modelica_parser_summary_runtime_identity_for_tests()?;
    let source = r#"
within Demo;
model GainHolder
  parameter Real gain = 1;
end GainHolder;
"#;

    let first = fetch_modelica_parser_file_summary_blocking_for_repository(
        &repository,
        "Demo/GainHolder.mo",
        source,
    )?;
    let runtime_after_first = shared_modelica_parser_summary_runtime_identity_for_tests()?;
    let second = fetch_modelica_parser_file_summary_blocking_for_repository(
        &repository,
        "Demo/GainHolder.mo",
        source,
    )?;
    let runtime_after_second = shared_modelica_parser_summary_runtime_identity_for_tests()?;

    assert_eq!(runtime_before, runtime_after_first);
    assert_eq!(runtime_after_first, runtime_after_second);
    assert_eq!(first.class_name.as_deref(), Some("GainHolder"));
    assert_eq!(second.class_name.as_deref(), Some("GainHolder"));
    assert!(
        first
            .declarations
            .iter()
            .any(|declaration| declaration.name == "GainHolder"),
        "expected GainHolder declaration in first summary: {:?}",
        first.declarations,
    );
    assert!(
        second
            .declarations
            .iter()
            .any(|declaration| declaration.name == "GainHolder"),
        "expected GainHolder declaration in second summary: {:?}",
        second.declarations,
    );

    Ok(())
}
