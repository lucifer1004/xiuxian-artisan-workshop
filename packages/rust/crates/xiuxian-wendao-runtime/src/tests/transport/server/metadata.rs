use crate::transport::{
    ANALYSIS_CODE_AST_ROUTE, ANALYSIS_MARKDOWN_ROUTE, ANALYSIS_REPO_DOC_COVERAGE_ROUTE,
    ANALYSIS_REPO_INDEX_STATUS_ROUTE, ANALYSIS_REPO_OVERVIEW_ROUTE, ANALYSIS_REPO_SYNC_ROUTE,
    GRAPH_NEIGHBORS_ROUTE, SEARCH_AST_ROUTE, SEARCH_ATTACHMENTS_ROUTE, SEARCH_INTENT_ROUTE,
    SEARCH_KNOWLEDGE_ROUTE, VFS_CONTENT_ROUTE, VFS_RESOLVE_ROUTE, is_search_family_route,
    validate_attachment_search_request_metadata, validate_autocomplete_request_metadata,
    validate_code_ast_analysis_request_metadata, validate_definition_request_metadata,
    validate_graph_neighbors_request_metadata, validate_markdown_analysis_request_metadata,
    validate_repo_doc_coverage_request_metadata, validate_repo_index_status_request_metadata,
    validate_repo_overview_request_metadata, validate_repo_search_request_metadata,
    validate_repo_sync_request_metadata, validate_search_request_metadata,
    validate_sql_request_metadata, validate_vfs_content_request_metadata,
    validate_vfs_resolve_request_metadata,
};

use super::assertions::{must_err, must_ok};
use super::request_headers::{
    build_attachment_search_metadata, build_autocomplete_metadata,
    build_code_ast_analysis_metadata, build_definition_metadata, build_graph_neighbors_metadata,
    build_markdown_analysis_metadata, build_repo_doc_coverage_metadata,
    build_repo_index_status_metadata, build_repo_overview_metadata, build_repo_search_metadata,
    build_repo_sync_metadata, build_search_metadata, build_sql_metadata,
    build_vfs_content_metadata, build_vfs_resolve_metadata,
    populate_schema_and_search_headers_with_hints,
};

#[test]
fn validate_search_request_metadata_accepts_stable_request() {
    let metadata = build_search_metadata("semantic-route", "7");

    let (query_text, limit, intent, repo_hint) = must_ok(
        validate_search_request_metadata(&metadata),
        "stable search-family metadata should validate",
    );

    assert_eq!(query_text, "semantic-route");
    assert_eq!(limit, 7);
    assert_eq!(intent, None);
    assert_eq!(repo_hint, None);
}

#[test]
fn validate_search_request_metadata_accepts_intent_and_repo_hints() {
    let mut metadata = tonic::metadata::MetadataMap::new();
    populate_schema_and_search_headers_with_hints(
        &mut metadata,
        "semantic-route",
        "7",
        Some("code_search"),
        Some("gateway-sync"),
    );

    let (query_text, limit, intent, repo_hint) = must_ok(
        validate_search_request_metadata(&metadata),
        "search-family metadata with hints should validate",
    );

    assert_eq!(query_text, "semantic-route");
    assert_eq!(limit, 7);
    assert_eq!(intent.as_deref(), Some("code_search"));
    assert_eq!(repo_hint.as_deref(), Some("gateway-sync"));
}

#[test]
fn validate_search_request_metadata_rejects_blank_query_text() {
    let metadata = build_search_metadata("", "7");

    let error = must_err(
        validate_search_request_metadata(&metadata),
        "blank search-family query text should fail",
    );

    assert_eq!(error.message(), "repo search query text must not be blank");
}

#[test]
fn validate_search_request_metadata_rejects_zero_limit() {
    let metadata = build_search_metadata("semantic-route", "0");

    let error = must_err(
        validate_search_request_metadata(&metadata),
        "zero search-family limit should fail",
    );

    assert_eq!(
        error.message(),
        "repo search limit must be greater than zero"
    );
}

#[test]
fn validate_repo_search_request_metadata_accepts_repo_and_filters() {
    let metadata =
        build_repo_search_metadata("gateway-sync", "solve", "5", Some("julia"), Some("src/"));

    let request = must_ok(
        validate_repo_search_request_metadata(&metadata),
        "stable repo-search metadata should validate",
    );

    assert_eq!(request.repo_id, "gateway-sync");
    assert_eq!(request.query_text, "solve");
    assert_eq!(request.limit, 5);
    assert!(request.language_filters.contains("julia"));
    assert!(request.path_prefixes.contains("src/"));
}

#[test]
fn validate_repo_search_request_metadata_rejects_blank_repo() {
    let metadata = build_repo_search_metadata("   ", "solve", "5", None, None);

    let error: tonic::Status = must_err(
        validate_repo_search_request_metadata(&metadata),
        "blank repo-search repo should fail",
    );

    assert_eq!(
        error.message(),
        "repo search header `x-wendao-repo-search-repo` must not be blank"
    );
}

#[test]
fn validate_markdown_analysis_request_metadata_accepts_stable_request() {
    let metadata = build_markdown_analysis_metadata("docs/analysis.md");

    let path = must_ok(
        validate_markdown_analysis_request_metadata(&metadata),
        "stable markdown analysis metadata should validate",
    );

    assert_eq!(path, "docs/analysis.md");
}

#[test]
fn validate_markdown_analysis_request_metadata_rejects_blank_path() {
    let metadata = build_markdown_analysis_metadata("   ");

    let error = must_err(
        validate_markdown_analysis_request_metadata(&metadata),
        "blank markdown analysis path should fail",
    );

    assert_eq!(error.message(), "markdown analysis path must not be blank");
}

#[test]
fn validate_repo_doc_coverage_request_metadata_accepts_stable_request() {
    let metadata = build_repo_doc_coverage_metadata("gateway-sync", Some("GatewaySyncPkg"));

    let (repo_id, module_id) = must_ok(
        validate_repo_doc_coverage_request_metadata(&metadata),
        "stable repo doc coverage metadata should validate",
    );

    assert_eq!(repo_id, "gateway-sync");
    assert_eq!(module_id.as_deref(), Some("GatewaySyncPkg"));
}

#[test]
fn validate_repo_overview_request_metadata_accepts_stable_request() {
    let metadata = build_repo_overview_metadata("gateway-sync");

    let repo_id = must_ok(
        validate_repo_overview_request_metadata(&metadata),
        "stable repo overview metadata should validate",
    );

    assert_eq!(repo_id, "gateway-sync");
}

#[test]
fn validate_repo_index_status_request_metadata_accepts_stable_request() {
    let metadata = build_repo_index_status_metadata(Some("gateway-sync"));

    let repo_id = must_ok(
        validate_repo_index_status_request_metadata(&metadata),
        "stable repo index status metadata should validate",
    );

    assert_eq!(repo_id.as_deref(), Some("gateway-sync"));

    let metadata = build_repo_index_status_metadata(None);
    let repo_id = must_ok(
        validate_repo_index_status_request_metadata(&metadata),
        "unfiltered repo index status metadata should validate",
    );
    assert_eq!(repo_id, None);
}

#[test]
fn validate_repo_sync_request_metadata_accepts_stable_request() {
    let metadata = build_repo_sync_metadata("gateway-sync", Some("status"));

    let (repo_id, mode) = must_ok(
        validate_repo_sync_request_metadata(&metadata),
        "stable repo sync metadata should validate",
    );

    assert_eq!(repo_id, "gateway-sync");
    assert_eq!(mode, "status");

    let metadata = build_repo_sync_metadata("gateway-sync", None);
    let (repo_id, mode) = must_ok(
        validate_repo_sync_request_metadata(&metadata),
        "repo sync metadata without explicit mode should validate",
    );
    assert_eq!(repo_id, "gateway-sync");
    assert_eq!(mode, "ensure");
}

#[test]
fn validate_repo_overview_request_metadata_rejects_blank_repo() {
    let metadata = build_repo_overview_metadata("   ");

    let error = must_err(
        validate_repo_overview_request_metadata(&metadata),
        "blank repo overview repo should fail",
    );

    assert_eq!(error.message(), "repo overview repo must not be blank");
}

#[test]
fn validate_repo_doc_coverage_request_metadata_rejects_blank_repo() {
    let metadata = build_repo_doc_coverage_metadata("   ", Some("GatewaySyncPkg"));

    let error = must_err(
        validate_repo_doc_coverage_request_metadata(&metadata),
        "blank repo doc coverage repo should fail",
    );

    assert_eq!(error.message(), "repo doc coverage repo must not be blank");
}

#[test]
fn validate_repo_sync_request_metadata_rejects_blank_repo() {
    let metadata = build_repo_sync_metadata("   ", Some("status"));

    let error = must_err(
        validate_repo_sync_request_metadata(&metadata),
        "blank repo sync repo should fail",
    );

    assert_eq!(error.message(), "repo sync repo must not be blank");
}

#[test]
fn validate_repo_sync_request_metadata_rejects_invalid_mode() {
    let metadata = build_repo_sync_metadata("gateway-sync", Some("bogus"));

    let error = must_err(
        validate_repo_sync_request_metadata(&metadata),
        "invalid repo sync mode should fail",
    );

    assert_eq!(error.message(), "unsupported repo sync mode `bogus`");
}

#[test]
fn analysis_routes_do_not_alias_search_family_contracts() {
    assert!(!is_search_family_route(ANALYSIS_REPO_DOC_COVERAGE_ROUTE));
    assert!(!is_search_family_route(ANALYSIS_REPO_INDEX_STATUS_ROUTE));
    assert!(!is_search_family_route(ANALYSIS_REPO_OVERVIEW_ROUTE));
    assert!(!is_search_family_route(ANALYSIS_REPO_SYNC_ROUTE));
}

#[test]
fn validate_vfs_content_request_metadata_accepts_stable_request() {
    let metadata = build_vfs_content_metadata("main/docs/index.md");

    let path = must_ok(
        validate_vfs_content_request_metadata(&metadata),
        "stable VFS content metadata should validate",
    );

    assert_eq!(path, "main/docs/index.md");
}

#[test]
fn validate_vfs_content_request_metadata_rejects_blank_path() {
    let metadata = build_vfs_content_metadata("   ");

    let error = must_err(
        validate_vfs_content_request_metadata(&metadata),
        "blank VFS content path should fail",
    );

    assert_eq!(error.message(), "VFS content requires a non-empty path");
}

#[test]
fn validate_code_ast_analysis_request_metadata_accepts_stable_request() {
    let metadata = build_code_ast_analysis_metadata("src/lib.jl", "demo", Some("7"));

    let (path, repo_id, line_hint) = must_ok(
        validate_code_ast_analysis_request_metadata(&metadata),
        "stable code-AST analysis metadata should validate",
    );

    assert_eq!(path, "src/lib.jl");
    assert_eq!(repo_id, "demo");
    assert_eq!(line_hint, Some(7));
}

#[test]
fn validate_code_ast_analysis_request_metadata_rejects_blank_repo() {
    let metadata = build_code_ast_analysis_metadata("src/lib.jl", "   ", None);

    let error = must_err(
        validate_code_ast_analysis_request_metadata(&metadata),
        "blank code-AST repo should fail",
    );

    assert_eq!(error.message(), "code AST analysis repo must not be blank");
}

#[test]
fn validate_code_ast_analysis_request_metadata_rejects_non_numeric_line_hint() {
    let metadata = build_code_ast_analysis_metadata("src/lib.jl", "demo", Some("abc"));

    let error = must_err(
        validate_code_ast_analysis_request_metadata(&metadata),
        "non-numeric code-AST line hint should fail",
    );

    assert_eq!(
        error.message(),
        "invalid analysis line header `x-wendao-analysis-line`: abc"
    );
}

#[test]
fn validate_attachment_search_request_metadata_accepts_stable_request() {
    let metadata = build_attachment_search_metadata(
        "image",
        "5",
        Some("png,jpg"),
        Some("image,screenshot"),
        Some("true"),
    );

    let (query_text, limit, ext_filters, kind_filters, case_sensitive) = must_ok(
        validate_attachment_search_request_metadata(&metadata),
        "stable attachment-search metadata should validate",
    );

    assert_eq!(query_text, "image");
    assert_eq!(limit, 5);
    assert!(ext_filters.contains("png"));
    assert!(ext_filters.contains("jpg"));
    assert!(kind_filters.contains("image"));
    assert!(kind_filters.contains("screenshot"));
    assert!(case_sensitive);
}

#[test]
fn validate_attachment_search_request_metadata_rejects_blank_extension_filters() {
    let metadata =
        build_attachment_search_metadata("image", "5", Some("png, "), Some("image"), None);

    let error = must_err(
        validate_attachment_search_request_metadata(&metadata),
        "blank extension filter should fail",
    );

    assert_eq!(
        error.message(),
        "attachment search extension filters must not contain blank values"
    );
}

#[test]
fn validate_definition_request_metadata_accepts_stable_request() {
    let metadata = build_definition_metadata("AlphaService", Some("src/lib.rs"), Some("7"));

    let (query_text, source_path, source_line) = must_ok(
        validate_definition_request_metadata(&metadata),
        "stable definition metadata should validate",
    );

    assert_eq!(query_text, "AlphaService");
    assert_eq!(source_path.as_deref(), Some("src/lib.rs"));
    assert_eq!(source_line, Some(7));
}

#[test]
fn validate_definition_request_metadata_rejects_non_numeric_line_hint() {
    let metadata = build_definition_metadata("AlphaService", Some("src/lib.rs"), Some("abc"));

    let error = must_err(
        validate_definition_request_metadata(&metadata),
        "non-numeric definition line hint should fail",
    );

    assert_eq!(
        error.message(),
        "invalid definition line header `x-wendao-definition-line`: abc"
    );
}

#[test]
fn validate_autocomplete_request_metadata_accepts_stable_request() {
    let metadata = build_autocomplete_metadata("Alpha", "5");

    let (prefix, limit) = must_ok(
        validate_autocomplete_request_metadata(&metadata),
        "stable autocomplete metadata should validate",
    );

    assert_eq!(prefix, "Alpha");
    assert_eq!(limit, 5);
}

#[test]
fn validate_autocomplete_request_metadata_rejects_zero_limit() {
    let metadata = build_autocomplete_metadata("Alpha", "0");

    let error = must_err(
        validate_autocomplete_request_metadata(&metadata),
        "zero autocomplete limit should fail",
    );

    assert_eq!(
        error.message(),
        "autocomplete limit must be greater than zero"
    );
}

#[test]
fn validate_sql_request_metadata_accepts_read_only_query() {
    let metadata = build_sql_metadata("SELECT doc_id FROM repo_entity");

    let query_text = must_ok(
        validate_sql_request_metadata(&metadata),
        "stable SQL metadata should validate",
    );

    assert_eq!(query_text, "SELECT doc_id FROM repo_entity");
}

#[test]
fn validate_sql_request_metadata_rejects_non_query_statement() {
    let metadata = build_sql_metadata("CREATE VIEW demo AS SELECT 1");

    let error = must_err(
        validate_sql_request_metadata(&metadata),
        "non-query SQL metadata should fail",
    );

    assert_eq!(
        error.message(),
        "SQL query text must be a read-only query statement"
    );
}

#[test]
fn validate_vfs_resolve_request_metadata_accepts_stable_request() {
    let metadata = build_vfs_resolve_metadata("main/docs/index.md");

    let path = must_ok(
        validate_vfs_resolve_request_metadata(&metadata),
        "stable VFS resolve metadata should validate",
    );

    assert_eq!(path, "main/docs/index.md");
}

#[test]
fn validate_vfs_resolve_request_metadata_rejects_blank_path() {
    let metadata = build_vfs_resolve_metadata("   ");

    let error = must_err(
        validate_vfs_resolve_request_metadata(&metadata),
        "blank VFS resolve path should fail",
    );

    assert_eq!(error.message(), "VFS resolve requires a non-empty path");
}

#[test]
fn validate_graph_neighbors_request_metadata_accepts_stable_request() {
    let metadata = build_graph_neighbors_metadata(
        "kernel/docs/index.md",
        Some("outgoing"),
        Some("3"),
        Some("25"),
    );

    let request = must_ok(
        validate_graph_neighbors_request_metadata(&metadata),
        "stable graph-neighbors metadata should validate",
    );

    assert_eq!(
        request,
        (
            "kernel/docs/index.md".to_string(),
            "outgoing".to_string(),
            3,
            25,
        )
    );
}

#[test]
fn validate_graph_neighbors_request_metadata_normalizes_defaults() {
    let metadata =
        build_graph_neighbors_metadata("kernel/docs/index.md", Some("invalid"), None, None);

    let request = must_ok(
        validate_graph_neighbors_request_metadata(&metadata),
        "graph-neighbors metadata should normalize defaults",
    );

    assert_eq!(
        request,
        (
            "kernel/docs/index.md".to_string(),
            "both".to_string(),
            2,
            50,
        )
    );
}

#[test]
fn validate_graph_neighbors_request_metadata_rejects_invalid_limit() {
    let metadata = build_graph_neighbors_metadata(
        "kernel/docs/index.md",
        Some("both"),
        Some("2"),
        Some("abc"),
    );

    let error = must_err(
        validate_graph_neighbors_request_metadata(&metadata),
        "non-numeric graph-neighbors limit should fail",
    );

    assert_eq!(
        error.message(),
        "invalid graph neighbors limit header `x-wendao-graph-limit`: abc"
    );
}

#[test]
fn search_family_route_matcher_accepts_semantic_business_routes() {
    assert!(is_search_family_route(SEARCH_INTENT_ROUTE));
    assert!(is_search_family_route(SEARCH_KNOWLEDGE_ROUTE));
    assert!(!is_search_family_route(SEARCH_ATTACHMENTS_ROUTE));
    assert!(!is_search_family_route(SEARCH_AST_ROUTE));
    assert!(!is_search_family_route(VFS_RESOLVE_ROUTE));
    assert!(!is_search_family_route(VFS_CONTENT_ROUTE));
    assert!(!is_search_family_route(GRAPH_NEIGHBORS_ROUTE));
    assert!(!is_search_family_route(ANALYSIS_MARKDOWN_ROUTE));
    assert!(!is_search_family_route(ANALYSIS_CODE_AST_ROUTE));
    assert!(!is_search_family_route(ANALYSIS_REPO_DOC_COVERAGE_ROUTE));
    assert!(!is_search_family_route(ANALYSIS_REPO_OVERVIEW_ROUTE));
}
