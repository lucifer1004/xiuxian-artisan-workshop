#![cfg(feature = "julia")]

use std::sync::Arc;

use serde_json::json;
use xiuxian_zhenfa::{ZhenfaContext, ZhenfaRegistry, ZhenfaTool};

use crate::analyzers::service::{
    DocsDocumentSegmentResult, DocsToolRuntime, DocsToolRuntimeHandle,
};
use crate::analyzers::{
    DocsNavigationOptions, DocsPageIndexDocumentsResult, DocsPageIndexNodeResult,
    DocsPageIndexTreeResult, DocsPageIndexTreeSearchResult, DocsPageIndexTreesResult,
    DocsPageResult, DocsRetrievalContextOptions, DocsRetrievalContextResult, DocsToolService,
    ProjectedPageIndexDocument, ProjectedPageIndexNodeHit, ProjectedPageRecord, ProjectionPageKind,
    RepoIntelligenceError,
};
use crate::zhenfa_router::native::{
    WendaoContextExt, WendaoDocsGetDocumentNodeTool, WendaoDocsGetDocumentSegmentTool,
    WendaoDocsGetDocumentStructureCatalogTool, WendaoDocsGetDocumentStructureOutlineTool,
    WendaoDocsGetDocumentTool, WendaoDocsGetTocDocumentsTool,
    WendaoDocsSearchDocumentStructureTool, register_wendao_docs_native_tools,
    resolve_docs_tool_runtime,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TEST_REPO_ID: &str = "modelica-docs-native-tool";
const TEST_PAGE_ID: &str = "repo:modelica-docs-native-tool:projection:reference:symbol:repo:modelica-docs-native-tool:symbol:Projectionica.Controllers.PI";

#[derive(Default)]
struct FakeDocsToolRuntime;

impl DocsToolRuntime for FakeDocsToolRuntime {
    fn get_document(&self, page_id: &str) -> Result<DocsPageResult, RepoIntelligenceError> {
        Ok(DocsPageResult {
            repo_id: TEST_REPO_ID.to_string(),
            page: ProjectedPageRecord {
                repo_id: TEST_REPO_ID.to_string(),
                page_id: page_id.to_string(),
                kind: ProjectionPageKind::Reference,
                title: "Projectionica.Controllers.PI".to_string(),
                module_ids: Vec::new(),
                symbol_ids: vec![
                    "repo:modelica-docs-native-tool:symbol:Projectionica.Controllers.PI"
                        .to_string(),
                ],
                example_ids: Vec::new(),
                doc_ids: Vec::new(),
                paths: vec!["Projectionica/Controllers/PI.mo".to_string()],
                format_hints: vec!["modelica".to_string()],
                sections: Vec::new(),
                doc_id: String::new(),
                path: "Projectionica/Controllers/PI.mo".to_string(),
                keywords: vec![
                    "Projectionica.Controllers.PI".to_string(),
                    "Projectionica/Controllers/PI.mo".to_string(),
                    "modelica".to_string(),
                ],
            },
        })
    }

    fn get_document_structure(
        &self,
        _page_id: &str,
    ) -> Result<DocsPageIndexTreeResult, RepoIntelligenceError> {
        panic!("get_document_structure is not used in this test")
    }

    fn get_document_structure_outline(
        &self,
        _page_id: &str,
    ) -> Result<DocsPageIndexTreeResult, RepoIntelligenceError> {
        Ok(DocsPageIndexTreeResult {
            repo_id: TEST_REPO_ID.to_string(),
            tree: Some(crate::analyzers::ProjectedPageIndexTree {
                repo_id: TEST_REPO_ID.to_string(),
                page_id: TEST_PAGE_ID.to_string(),
                kind: ProjectionPageKind::Reference,
                path: "reference/projectionica-controllers-pi.md".to_string(),
                doc_id: "doc:projectionica-controllers-pi".to_string(),
                title: "Projectionica.Controllers.PI".to_string(),
                root_count: 1,
                roots: vec![crate::analyzers::ProjectedPageIndexNode {
                    node_id: "outline:1".to_string(),
                    title: "Anchors".to_string(),
                    level: 2,
                    structural_path: vec![
                        "Projectionica.Controllers.PI".to_string(),
                        "Anchors".to_string(),
                    ],
                    line_range: (12, 18),
                    token_count: 4,
                    is_thinned: false,
                    text: String::new(),
                    summary: Some("Anchor summary".to_string()),
                    children: Vec::new(),
                }],
            }),
        })
    }

    fn get_document_structure_catalog(
        &self,
    ) -> Result<DocsPageIndexTreesResult, RepoIntelligenceError> {
        Ok(DocsPageIndexTreesResult {
            repo_id: TEST_REPO_ID.to_string(),
            trees: vec![crate::analyzers::ProjectedPageIndexTree {
                repo_id: TEST_REPO_ID.to_string(),
                page_id: TEST_PAGE_ID.to_string(),
                kind: ProjectionPageKind::Reference,
                path: "reference/projectionica-controllers-pi.md".to_string(),
                doc_id: "doc:projectionica-controllers-pi".to_string(),
                title: "Projectionica.Controllers.PI".to_string(),
                root_count: 1,
                roots: vec![crate::analyzers::ProjectedPageIndexNode {
                    node_id: "catalog:1".to_string(),
                    title: "Anchors".to_string(),
                    level: 2,
                    structural_path: vec![
                        "Projectionica.Controllers.PI".to_string(),
                        "Anchors".to_string(),
                    ],
                    line_range: (12, 18),
                    token_count: 4,
                    is_thinned: false,
                    text: String::new(),
                    summary: Some("Anchor summary".to_string()),
                    children: Vec::new(),
                }],
            }],
        })
    }

    fn get_document_segment(
        &self,
        page_id: &str,
        line_start: usize,
        line_end: usize,
    ) -> Result<DocsDocumentSegmentResult, RepoIntelligenceError> {
        Ok(DocsDocumentSegmentResult {
            repo_id: TEST_REPO_ID.to_string(),
            page_id: page_id.to_string(),
            kind: ProjectionPageKind::Reference,
            path: "reference/projectionica-controllers-pi.md".to_string(),
            title: "Projectionica.Controllers.PI".to_string(),
            line_range: (line_start, line_end),
            line_count: 18,
            content: "## Anchors\nBody".to_string(),
        })
    }

    fn get_document_node(
        &self,
        page_id: &str,
        node_id: &str,
    ) -> Result<DocsPageIndexNodeResult, RepoIntelligenceError> {
        Ok(DocsPageIndexNodeResult {
            repo_id: TEST_REPO_ID.to_string(),
            hit: Some(ProjectedPageIndexNodeHit {
                repo_id: TEST_REPO_ID.to_string(),
                page_id: page_id.to_string(),
                page_title: "Projectionica.Controllers.PI".to_string(),
                page_kind: ProjectionPageKind::Reference,
                path: "reference/projectionica-controllers-pi.md".to_string(),
                doc_id: "doc:projectionica-controllers-pi".to_string(),
                node_id: node_id.to_string(),
                node_title: "Anchors".to_string(),
                structural_path: vec![
                    "Projectionica.Controllers.PI".to_string(),
                    "Anchors".to_string(),
                ],
                line_range: (12, 18),
                text: "## Anchors\nBody".to_string(),
            }),
        })
    }

    fn search_document_structure(
        &self,
        query: &str,
        kind: Option<ProjectionPageKind>,
        limit: usize,
    ) -> Result<DocsPageIndexTreeSearchResult, RepoIntelligenceError> {
        Ok(DocsPageIndexTreeSearchResult {
            repo_id: TEST_REPO_ID.to_string(),
            hits: vec![ProjectedPageIndexNodeHit {
                repo_id: TEST_REPO_ID.to_string(),
                page_id: TEST_PAGE_ID.to_string(),
                page_title: "Projectionica.Controllers.PI".to_string(),
                page_kind: kind.unwrap_or(ProjectionPageKind::Reference),
                path: "reference/projectionica-controllers-pi.md".to_string(),
                doc_id: "doc:projectionica-controllers-pi".to_string(),
                node_id: format!("search:{limit}"),
                node_title: query.to_ascii_uppercase(),
                structural_path: vec![
                    "Projectionica.Controllers.PI".to_string(),
                    query.to_string(),
                ],
                line_range: (12, 18),
                text: "## Anchors\nBody".to_string(),
            }],
        })
    }

    fn get_toc_documents(&self) -> Result<DocsPageIndexDocumentsResult, RepoIntelligenceError> {
        Ok(DocsPageIndexDocumentsResult {
            repo_id: TEST_REPO_ID.to_string(),
            documents: vec![ProjectedPageIndexDocument {
                repo_id: TEST_REPO_ID.to_string(),
                page_id: TEST_PAGE_ID.to_string(),
                path: "reference/projectionica-controllers-pi.md".to_string(),
                doc_id: "doc:projectionica-controllers-pi".to_string(),
                title: "Projectionica.Controllers.PI".to_string(),
                sections: Vec::new(),
            }],
        })
    }

    fn get_navigation_with_options(
        &self,
        _page_id: &str,
        _options: DocsNavigationOptions,
    ) -> Result<crate::analyzers::DocsNavigationResult, RepoIntelligenceError> {
        panic!("get_navigation_with_options is not used in this test")
    }

    fn get_retrieval_context_with_options(
        &self,
        _page_id: &str,
        _options: DocsRetrievalContextOptions,
    ) -> Result<DocsRetrievalContextResult, RepoIntelligenceError> {
        panic!("get_retrieval_context_with_options is not used in this test")
    }
}

#[test]
fn get_document_tool_returns_serialized_page_payload() -> TestResult {
    let runtime: Arc<dyn DocsToolRuntime> = Arc::new(FakeDocsToolRuntime);
    let mut ctx = ZhenfaContext::default();
    let _ = ctx.insert_extension(DocsToolRuntimeHandle::new(runtime));

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let output = runtime.block_on(
        WendaoDocsGetDocumentTool.call_native(&ctx, json!({ "page_id": TEST_PAGE_ID })),
    )?;

    let payload: serde_json::Value = serde_json::from_str(&output)?;
    assert_eq!(payload["page"]["title"], "Projectionica.Controllers.PI");
    assert_eq!(payload["page"]["page_id"], TEST_PAGE_ID);
    Ok(())
}

#[test]
fn get_toc_documents_tool_returns_serialized_page_index_documents_payload() -> TestResult {
    let runtime: Arc<dyn DocsToolRuntime> = Arc::new(FakeDocsToolRuntime);
    let mut ctx = ZhenfaContext::default();
    let _ = ctx.insert_extension(DocsToolRuntimeHandle::new(runtime));

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let output = runtime.block_on(WendaoDocsGetTocDocumentsTool.call_native(&ctx, json!({})))?;

    let payload: serde_json::Value = serde_json::from_str(&output)?;
    assert_eq!(payload["repo_id"], TEST_REPO_ID);
    assert_eq!(payload["documents"][0]["page_id"], TEST_PAGE_ID);
    Ok(())
}

#[test]
fn get_document_node_tool_returns_serialized_node_payload() -> TestResult {
    let runtime: Arc<dyn DocsToolRuntime> = Arc::new(FakeDocsToolRuntime);
    let mut ctx = ZhenfaContext::default();
    let _ = ctx.insert_extension(DocsToolRuntimeHandle::new(runtime));

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let output = runtime.block_on(
        WendaoDocsGetDocumentNodeTool
            .call_native(&ctx, json!({ "page_id": TEST_PAGE_ID, "node_id": "0007" })),
    )?;

    let payload: serde_json::Value = serde_json::from_str(&output)?;
    assert_eq!(payload["repo_id"], TEST_REPO_ID);
    assert_eq!(payload["hit"]["page_id"], TEST_PAGE_ID);
    assert_eq!(payload["hit"]["node_id"], "0007");
    assert_eq!(payload["hit"]["node_title"], "Anchors");
    Ok(())
}

#[test]
fn get_document_structure_outline_tool_returns_text_free_tree_payload() -> TestResult {
    let runtime: Arc<dyn DocsToolRuntime> = Arc::new(FakeDocsToolRuntime);
    let mut ctx = ZhenfaContext::default();
    let _ = ctx.insert_extension(DocsToolRuntimeHandle::new(runtime));

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let output = runtime.block_on(
        WendaoDocsGetDocumentStructureOutlineTool
            .call_native(&ctx, json!({ "page_id": TEST_PAGE_ID })),
    )?;

    let payload: serde_json::Value = serde_json::from_str(&output)?;
    assert_eq!(payload["repo_id"], TEST_REPO_ID);
    assert_eq!(payload["tree"]["page_id"], TEST_PAGE_ID);
    assert_eq!(payload["tree"]["roots"][0]["text"], "");
    assert_eq!(payload["tree"]["roots"][0]["summary"], "Anchor summary");
    Ok(())
}

#[test]
fn get_document_structure_catalog_tool_returns_text_free_trees_payload() -> TestResult {
    let runtime: Arc<dyn DocsToolRuntime> = Arc::new(FakeDocsToolRuntime);
    let mut ctx = ZhenfaContext::default();
    let _ = ctx.insert_extension(DocsToolRuntimeHandle::new(runtime));

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let output =
        runtime.block_on(WendaoDocsGetDocumentStructureCatalogTool.call_native(&ctx, json!({})))?;

    let payload: serde_json::Value = serde_json::from_str(&output)?;
    assert_eq!(payload["repo_id"], TEST_REPO_ID);
    assert_eq!(payload["trees"][0]["page_id"], TEST_PAGE_ID);
    assert_eq!(payload["trees"][0]["roots"][0]["text"], "");
    assert_eq!(payload["trees"][0]["roots"][0]["summary"], "Anchor summary");
    Ok(())
}

#[test]
fn get_document_segment_tool_returns_serialized_segment_payload() -> TestResult {
    let runtime: Arc<dyn DocsToolRuntime> = Arc::new(FakeDocsToolRuntime);
    let mut ctx = ZhenfaContext::default();
    let _ = ctx.insert_extension(DocsToolRuntimeHandle::new(runtime));

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let output = runtime.block_on(WendaoDocsGetDocumentSegmentTool.call_native(
        &ctx,
        json!({ "page_id": TEST_PAGE_ID, "line_start": 12, "line_end": 18 }),
    ))?;

    let payload: serde_json::Value = serde_json::from_str(&output)?;
    assert_eq!(payload["repo_id"], TEST_REPO_ID);
    assert_eq!(payload["page_id"], TEST_PAGE_ID);
    assert_eq!(payload["line_range"][0], 12);
    assert_eq!(payload["line_range"][1], 18);
    assert_eq!(payload["content"], "## Anchors\nBody");
    Ok(())
}

#[test]
fn search_document_structure_tool_returns_serialized_hits_payload() -> TestResult {
    let runtime: Arc<dyn DocsToolRuntime> = Arc::new(FakeDocsToolRuntime);
    let mut ctx = ZhenfaContext::default();
    let _ = ctx.insert_extension(DocsToolRuntimeHandle::new(runtime));

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let output = runtime.block_on(WendaoDocsSearchDocumentStructureTool.call_native(
        &ctx,
        json!({ "query": "anchors", "kind": "reference", "limit": 3 }),
    ))?;

    let payload: serde_json::Value = serde_json::from_str(&output)?;
    assert_eq!(payload["repo_id"], TEST_REPO_ID);
    assert_eq!(payload["hits"][0]["page_id"], TEST_PAGE_ID);
    assert_eq!(payload["hits"][0]["node_title"], "ANCHORS");
    assert_eq!(payload["hits"][0]["node_id"], "search:3");
    Ok(())
}

#[test]
fn docs_tool_service_context_extension_requires_presence() {
    let ctx = ZhenfaContext::default();
    assert!(ctx.docs_tool_service().is_err());
    assert!(resolve_docs_tool_runtime(&ctx).is_err());
}

#[test]
fn docs_tool_runtime_falls_back_to_docs_tool_service_extension() {
    let mut ctx = ZhenfaContext::default();
    let _ = ctx.insert_extension(DocsToolService::from_project_root(".", TEST_REPO_ID));

    assert!(resolve_docs_tool_runtime(&ctx).is_ok());
}

#[test]
fn docs_native_tools_register_all_capabilities() {
    let mut registry = ZhenfaRegistry::new();
    register_wendao_docs_native_tools(&mut registry);

    assert_eq!(registry.len(), 10);
    assert!(registry.contains("wendao.docs.get_document"));
    assert!(registry.contains("wendao.docs.get_document_structure"));
    assert!(registry.contains("wendao.docs.get_document_structure_outline"));
    assert!(registry.contains("wendao.docs.get_document_structure_catalog"));
    assert!(registry.contains("wendao.docs.get_document_segment"));
    assert!(registry.contains("wendao.docs.search_document_structure"));
    assert!(registry.contains("wendao.docs.get_document_node"));
    assert!(registry.contains("wendao.docs.get_toc_documents"));
    assert!(registry.contains("wendao.docs.get_navigation"));
    assert!(registry.contains("wendao.docs.get_retrieval_context"));
}
