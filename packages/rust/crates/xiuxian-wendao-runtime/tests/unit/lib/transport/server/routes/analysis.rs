use std::sync::Arc;

use arrow_flight::flight_service_server::FlightService;
use tonic::Request;

use crate::transport::{
    ANALYSIS_CODE_AST_ROUTE, ANALYSIS_MARKDOWN_ROUTE, ANALYSIS_REPO_DOC_COVERAGE_ROUTE,
    ANALYSIS_REPO_INDEX_STATUS_ROUTE, ANALYSIS_REPO_OVERVIEW_ROUTE, ANALYSIS_REPO_SYNC_ROUTE,
};

use super::super::assertions::{must_err, must_ok, parse_json, route_descriptor, ticket_string};
use super::super::fixtures::build_service_with_route_providers;
use super::super::providers::{
    RecordingCodeAstAnalysisProvider, RecordingMarkdownAnalysisProvider,
    RecordingRepoDocCoverageProvider, RecordingRepoIndexStatusProvider,
    RecordingRepoOverviewProvider, RecordingRepoSyncProvider, RecordingSearchProvider,
};
use super::super::request_headers::{
    populate_schema_and_code_ast_analysis_headers, populate_schema_and_markdown_analysis_headers,
    populate_schema_and_repo_doc_coverage_headers, populate_schema_and_repo_index_status_headers,
    populate_schema_and_repo_overview_headers, populate_schema_and_repo_sync_headers,
};

#[tokio::test]
async fn wendao_flight_service_get_flight_info_uses_markdown_analysis_provider() {
    let provider = Arc::new(RecordingMarkdownAnalysisProvider::default());
    let service = build_service_with_route_providers(|route_providers| {
        route_providers.markdown_analysis = Some(provider.clone());
    });
    let mut request = Request::new(route_descriptor(ANALYSIS_MARKDOWN_ROUTE));
    populate_schema_and_markdown_analysis_headers(request.metadata_mut(), "docs/analysis.md");

    let flight_info = must_ok(
        service.get_flight_info(request).await,
        "markdown analysis route should resolve through the dedicated provider",
    )
    .into_inner();
    let ticket = ticket_string(
        &flight_info,
        "markdown analysis route should emit one ticket",
    );

    assert_eq!(ticket, ANALYSIS_MARKDOWN_ROUTE);
    assert_eq!(
        provider.recorded_request(),
        Some("docs/analysis.md".to_string())
    );
    let app_metadata = parse_json(&flight_info.app_metadata, "app_metadata should decode");
    assert_eq!(app_metadata["path"], "docs/analysis.md");
    assert_eq!(app_metadata["documentHash"], "fp:markdown");
    assert_eq!(app_metadata["nodeCount"], 1);
    assert_eq!(app_metadata["edgeCount"], 0);
}

#[tokio::test]
async fn wendao_flight_service_get_flight_info_uses_code_ast_analysis_provider() {
    let provider = Arc::new(RecordingCodeAstAnalysisProvider::default());
    let service = build_service_with_route_providers(|route_providers| {
        route_providers.code_ast_analysis = Some(provider.clone());
    });
    let mut request = Request::new(route_descriptor(ANALYSIS_CODE_AST_ROUTE));
    populate_schema_and_code_ast_analysis_headers(
        request.metadata_mut(),
        "src/lib.jl",
        "demo",
        Some("7"),
    );

    let flight_info = must_ok(
        service.get_flight_info(request).await,
        "code-AST analysis route should resolve through the dedicated provider",
    )
    .into_inner();
    let ticket = ticket_string(
        &flight_info,
        "code-AST analysis route should emit one ticket",
    );

    assert_eq!(ticket, ANALYSIS_CODE_AST_ROUTE);
    assert_eq!(
        provider.recorded_request(),
        Some(("src/lib.jl".to_string(), "demo".to_string(), Some(7)))
    );
    let app_metadata = parse_json(&flight_info.app_metadata, "app_metadata should decode");
    assert_eq!(app_metadata["repoId"], "demo");
    assert_eq!(app_metadata["path"], "src/lib.jl");
    assert_eq!(app_metadata["language"], "julia");
    assert_eq!(app_metadata["nodeCount"], 1);
    assert_eq!(app_metadata["edgeCount"], 0);
    assert_eq!(app_metadata["focusNodeId"], "line:7");
}

#[tokio::test]
async fn wendao_flight_service_rejects_unconfigured_markdown_analysis_route() {
    let service = build_service_with_route_providers(|route_providers| {
        route_providers.search = Some(Arc::new(RecordingSearchProvider::default()));
    });
    let mut request = Request::new(route_descriptor(ANALYSIS_MARKDOWN_ROUTE));
    populate_schema_and_markdown_analysis_headers(request.metadata_mut(), "docs/analysis.md");

    let error = must_err(
        service.get_flight_info(request).await,
        "unconfigured markdown analysis route should fail",
    );

    assert_eq!(error.code(), tonic::Code::Unimplemented);
    assert_eq!(
        error.message(),
        "markdown analysis Flight route `/analysis/markdown` is not configured for this runtime host"
    );
}

#[tokio::test]
async fn wendao_flight_service_rejects_unconfigured_code_ast_analysis_route() {
    let service = build_service_with_route_providers(|route_providers| {
        route_providers.search = Some(Arc::new(RecordingSearchProvider::default()));
    });
    let mut request = Request::new(route_descriptor(ANALYSIS_CODE_AST_ROUTE));
    populate_schema_and_code_ast_analysis_headers(
        request.metadata_mut(),
        "src/lib.jl",
        "demo",
        Some("7"),
    );

    let error = must_err(
        service.get_flight_info(request).await,
        "unconfigured code-AST analysis route should fail",
    );

    assert_eq!(error.code(), tonic::Code::Unimplemented);
    assert_eq!(
        error.message(),
        "code-AST analysis Flight route `/analysis/code-ast` is not configured for this runtime host"
    );
}

#[tokio::test]
async fn wendao_flight_service_get_flight_info_uses_repo_doc_coverage_provider() {
    let provider = Arc::new(RecordingRepoDocCoverageProvider::default());
    let service = build_service_with_route_providers(|route_providers| {
        route_providers.repo_doc_coverage = Some(provider.clone());
    });
    let mut request = Request::new(route_descriptor(ANALYSIS_REPO_DOC_COVERAGE_ROUTE));
    populate_schema_and_repo_doc_coverage_headers(
        request.metadata_mut(),
        "gateway-sync",
        Some("GatewaySyncPkg"),
    );

    let flight_info = must_ok(
        service.get_flight_info(request).await,
        "repo doc coverage route should resolve through the dedicated provider",
    )
    .into_inner();
    let ticket = ticket_string(
        &flight_info,
        "repo doc coverage route should emit one ticket",
    );

    assert_eq!(ticket, ANALYSIS_REPO_DOC_COVERAGE_ROUTE);
    assert_eq!(
        provider.recorded_request(),
        Some((
            "gateway-sync".to_string(),
            Some("GatewaySyncPkg".to_string())
        ))
    );
    let app_metadata = parse_json(&flight_info.app_metadata, "app_metadata should decode");
    assert_eq!(app_metadata["repoId"], "gateway-sync");
    assert_eq!(app_metadata["moduleId"], "GatewaySyncPkg");
    assert_eq!(app_metadata["coveredSymbols"], 3);
    assert_eq!(app_metadata["uncoveredSymbols"], 1);
}

#[tokio::test]
async fn wendao_flight_service_get_flight_info_uses_repo_overview_provider() {
    let provider = Arc::new(RecordingRepoOverviewProvider::default());
    let service = build_service_with_route_providers(|route_providers| {
        route_providers.repo_overview = Some(provider.clone());
    });
    let mut request = Request::new(route_descriptor(ANALYSIS_REPO_OVERVIEW_ROUTE));
    populate_schema_and_repo_overview_headers(request.metadata_mut(), "gateway-sync");

    let flight_info = must_ok(
        service.get_flight_info(request).await,
        "repo overview route should resolve through the dedicated provider",
    )
    .into_inner();
    let ticket = ticket_string(&flight_info, "repo overview route should emit one ticket");

    assert_eq!(ticket, ANALYSIS_REPO_OVERVIEW_ROUTE);
    assert_eq!(
        provider.recorded_request(),
        Some("gateway-sync".to_string())
    );
    let app_metadata = parse_json(&flight_info.app_metadata, "app_metadata should decode");
    assert_eq!(app_metadata["repoId"], "gateway-sync");
    assert_eq!(app_metadata["displayName"], "Gateway Sync");
    assert_eq!(app_metadata["moduleCount"], 3);
}

#[tokio::test]
async fn wendao_flight_service_get_flight_info_uses_repo_index_status_provider() {
    let provider = Arc::new(RecordingRepoIndexStatusProvider::default());
    let service = build_service_with_route_providers(|route_providers| {
        route_providers.repo_index_status = Some(provider.clone());
    });
    let mut request = Request::new(route_descriptor(ANALYSIS_REPO_INDEX_STATUS_ROUTE));
    populate_schema_and_repo_index_status_headers(request.metadata_mut(), Some("gateway-sync"));

    let flight_info = must_ok(
        service.get_flight_info(request).await,
        "repo index status route should resolve through the dedicated provider",
    )
    .into_inner();
    let ticket = ticket_string(
        &flight_info,
        "repo index status route should emit one ticket",
    );

    assert_eq!(ticket, ANALYSIS_REPO_INDEX_STATUS_ROUTE);
    assert_eq!(
        provider.recorded_request(),
        Some(Some("gateway-sync".to_string()))
    );
    let app_metadata = parse_json(&flight_info.app_metadata, "app_metadata should decode");
    assert_eq!(app_metadata["total"], 3);
    assert_eq!(app_metadata["targetConcurrency"], 2);
}

#[tokio::test]
async fn wendao_flight_service_get_flight_info_uses_repo_sync_provider() {
    let provider = Arc::new(RecordingRepoSyncProvider::default());
    let service = build_service_with_route_providers(|route_providers| {
        route_providers.repo_sync = Some(provider.clone());
    });
    let mut request = Request::new(route_descriptor(ANALYSIS_REPO_SYNC_ROUTE));
    populate_schema_and_repo_sync_headers(request.metadata_mut(), "gateway-sync", Some("status"));

    let flight_info = must_ok(
        service.get_flight_info(request).await,
        "repo sync route should resolve through the dedicated provider",
    )
    .into_inner();
    let ticket = ticket_string(&flight_info, "repo sync route should emit one ticket");

    assert_eq!(ticket, ANALYSIS_REPO_SYNC_ROUTE);
    assert_eq!(
        provider.recorded_request(),
        Some(("gateway-sync".to_string(), "status".to_string()))
    );
    let app_metadata = parse_json(&flight_info.app_metadata, "app_metadata should decode");
    assert_eq!(app_metadata["repoId"], "gateway-sync");
    assert_eq!(app_metadata["mode"], "status");
    assert_eq!(app_metadata["healthState"], "healthy");
}

#[tokio::test]
async fn wendao_flight_service_rejects_unconfigured_repo_doc_coverage_route() {
    let service = build_service_with_route_providers(|route_providers| {
        route_providers.search = Some(Arc::new(RecordingSearchProvider::default()));
    });
    let mut request = Request::new(route_descriptor(ANALYSIS_REPO_DOC_COVERAGE_ROUTE));
    populate_schema_and_repo_doc_coverage_headers(
        request.metadata_mut(),
        "gateway-sync",
        Some("GatewaySyncPkg"),
    );

    let error = must_err(
        service.get_flight_info(request).await,
        "unconfigured repo doc coverage route should fail",
    );

    assert_eq!(error.code(), tonic::Code::Unimplemented);
    assert_eq!(
        error.message(),
        "repo doc coverage Flight route `/analysis/repo-doc-coverage` is not configured for this runtime host"
    );
}

#[tokio::test]
async fn wendao_flight_service_rejects_unconfigured_repo_index_status_route() {
    let service = build_service_with_route_providers(|route_providers| {
        route_providers.search = Some(Arc::new(RecordingSearchProvider::default()));
    });
    let mut request = Request::new(route_descriptor(ANALYSIS_REPO_INDEX_STATUS_ROUTE));
    populate_schema_and_repo_index_status_headers(request.metadata_mut(), None);

    let error = must_err(
        service.get_flight_info(request).await,
        "unconfigured repo index status route should fail",
    );

    assert_eq!(error.code(), tonic::Code::Unimplemented);
    assert_eq!(
        error.message(),
        "repo index status Flight route `/analysis/repo-index-status` is not configured for this runtime host"
    );
}

#[tokio::test]
async fn wendao_flight_service_rejects_unconfigured_repo_sync_route() {
    let service = build_service_with_route_providers(|route_providers| {
        route_providers.search = Some(Arc::new(RecordingSearchProvider::default()));
    });
    let mut request = Request::new(route_descriptor(ANALYSIS_REPO_SYNC_ROUTE));
    populate_schema_and_repo_sync_headers(request.metadata_mut(), "gateway-sync", Some("status"));

    let error = must_err(
        service.get_flight_info(request).await,
        "unconfigured repo sync route should fail",
    );

    assert_eq!(error.code(), tonic::Code::Unimplemented);
    assert_eq!(
        error.message(),
        "repo sync Flight route `/analysis/repo-sync` is not configured for this runtime host"
    );
}

#[tokio::test]
async fn wendao_flight_service_rejects_unconfigured_repo_overview_route() {
    let service = build_service_with_route_providers(|route_providers| {
        route_providers.search = Some(Arc::new(RecordingSearchProvider::default()));
    });
    let mut request = Request::new(route_descriptor(ANALYSIS_REPO_OVERVIEW_ROUTE));
    populate_schema_and_repo_overview_headers(request.metadata_mut(), "gateway-sync");

    let error = must_err(
        service.get_flight_info(request).await,
        "unconfigured repo overview route should fail",
    );

    assert_eq!(error.code(), tonic::Code::Unimplemented);
    assert_eq!(
        error.message(),
        "repo overview Flight route `/analysis/repo-overview` is not configured for this runtime host"
    );
}
