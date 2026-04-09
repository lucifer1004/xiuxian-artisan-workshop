use crate::gateway::studio::router::handlers::repo::parse::required_import_search_filters;

#[test]
fn import_search_filters_require_package_or_module() {
    let Err(error) = required_import_search_filters(None, None) else {
        panic!("missing import filters should fail");
    };
    assert_eq!(error.code(), "MISSING_IMPORT_FILTER");
}
