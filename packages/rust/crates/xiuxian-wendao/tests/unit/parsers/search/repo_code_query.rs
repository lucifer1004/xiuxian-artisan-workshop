use crate::parsers::search::repo_code_query::{
    parse_repo_code_search_query, parse_repo_code_search_query_with_repo_hint,
};

#[test]
fn parse_repo_code_search_query_extracts_lang_kind_and_term() {
    let spec = parse_repo_code_search_query("lang:julia kind:file reexport");

    assert_eq!(spec.search_term(), Some("reexport"));
    assert!(spec.language_filters.contains("julia"));
    assert!(spec.kind_filters.contains("file"));
}

#[test]
fn parse_repo_code_search_query_keeps_unknown_kind_token_in_search_term() {
    let spec = parse_repo_code_search_query("kind:custom reexport");

    assert_eq!(spec.search_term(), Some("kind:custom reexport"));
}

#[test]
fn parse_repo_code_search_query_normalizes_filters_and_ignores_blank_directives() {
    let spec = parse_repo_code_search_query("lang:Julia lang:  kind:MODULE kind:example");

    assert!(spec.language_filters.contains("julia"));
    assert_eq!(spec.language_filters.len(), 1);
    assert!(spec.kind_filters.contains("module"));
    assert!(spec.kind_filters.contains("example"));
    assert_eq!(spec.search_term(), None);
}

#[test]
fn parse_repo_code_search_query_extracts_repo_and_ast_pattern() {
    let spec =
        parse_repo_code_search_query("repo:lancd lang:rust ast:\"fn $NAME($$$ARGS) { $$$BODY }\"");

    assert_eq!(spec.repo.as_deref(), Some("lancd"));
    assert!(spec.language_filters.contains("rust"));
    assert_eq!(
        spec.ast_pattern.as_deref(),
        Some("fn $NAME($$$ARGS) { $$$BODY }")
    );
    assert_eq!(spec.search_term(), None);
}

#[test]
fn parse_repo_code_search_query_preserves_repo_identifier_case() {
    let spec = parse_repo_code_search_query("repo:DifferentialEquations.jl using ModelingToolkit");

    assert_eq!(spec.repo.as_deref(), Some("DifferentialEquations.jl"));
    assert_eq!(spec.search_term(), Some("using ModelingToolkit"));
}

#[test]
fn parse_repo_code_search_query_with_repo_hint_uses_hint_until_repo_directive_overrides() {
    let hinted = parse_repo_code_search_query_with_repo_hint("lang:julia solve", Some("SciMLBase"));
    assert_eq!(hinted.repo.as_deref(), Some("SciMLBase"));
    assert_eq!(hinted.search_term(), Some("solve"));

    let overridden = parse_repo_code_search_query_with_repo_hint(
        "repo:OrdinaryDiffEq.jl solve",
        Some("SciMLBase"),
    );
    assert_eq!(overridden.repo.as_deref(), Some("OrdinaryDiffEq.jl"));
}
