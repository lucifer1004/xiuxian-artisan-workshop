use crate::parsers::search::repo_code_query::parse_repo_code_search_query;

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
