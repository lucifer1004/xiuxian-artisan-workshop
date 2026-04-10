use xiuxian_wendao_core::repo_intelligence::{RegisteredRepository, RepositoryPluginConfig};

use super::{
    fetch_julia_parser_file_summary_for_repository, fetch_julia_parser_root_summary_for_repository,
    validate_julia_parser_summary_preflight_for_repository,
};
use crate::julia_plugin_test_support::common::ensure_linked_julia_parser_summary_service;

fn parser_summary_repository() -> RegisteredRepository {
    RegisteredRepository {
        id: "repo-julia".to_string(),
        plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
        ..RegisteredRepository::default()
    }
}

#[test]
fn parser_summary_preflight_accepts_plain_plugin_default_discovery() {
    let repository = parser_summary_repository();

    validate_julia_parser_summary_preflight_for_repository(&repository).unwrap_or_else(|error| {
        panic!("plain Julia plugin id should resolve parser summary: {error}")
    });
}

#[tokio::test]
#[serial_test::serial(julia_live)]
async fn fetch_parser_summaries_against_linked_real_wendaosearch_service()
-> Result<(), Box<dyn std::error::Error>> {
    ensure_linked_julia_parser_summary_service()?;
    let repository = parser_summary_repository();
    let source = r#"module Demo
export solve
using ..Core: solve as solver
include("solvers.jl")

"""
Solve docs.
"""
function solve(problem::Problem)
    problem.x
end

const LIMIT = 1
end
"#;

    let summary =
        fetch_julia_parser_file_summary_for_repository(&repository, "src/Demo.jl", source)
            .await
            .unwrap_or_else(|error| panic!("file summary fetch should succeed: {error}"));

    assert_eq!(summary.module_name.as_deref(), Some("Demo"));
    assert_eq!(summary.exports, vec!["solve".to_string()]);
    assert_eq!(summary.includes, vec!["solvers.jl".to_string()]);
    assert_eq!(summary.imports.len(), 1);
    assert_eq!(summary.imports[0].module, "..Core.solve".to_string());
    assert!(summary.imports[0].dependency_is_relative);
    assert_eq!(
        summary.imports[0].dependency_alias.as_deref(),
        Some("solver")
    );
    assert!(
        summary
            .symbols
            .iter()
            .any(|symbol| symbol.name == "solve" && symbol.signature.is_some()),
        "missing `solve` symbol: {:?}",
        summary.symbols,
    );
    assert!(
        summary.symbols.iter().any(|symbol| symbol.name == "LIMIT"),
        "missing `LIMIT` binding: {:?}",
        summary.symbols,
    );
    assert!(
        summary
            .docstrings
            .iter()
            .any(|doc| doc.target_name == "solve" && doc.content == "Solve docs."),
        "missing `solve` docstring: {:?}",
        summary.docstrings,
    );
    let error = fetch_julia_parser_root_summary_for_repository(
        &repository,
        "src/standalone.jl",
        "solve(x) = x\n",
    )
    .await
    .expect_err("root summary without module must fail");

    assert!(
        error
            .to_string()
            .contains("Julia root summary requires one root module declaration"),
        "unexpected error: {error}",
    );

    Ok(())
}
