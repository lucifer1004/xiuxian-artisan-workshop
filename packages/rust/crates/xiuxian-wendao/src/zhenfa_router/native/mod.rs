//! Native Zhenfa router tools for Wendao.
//!
//! This module keeps the public tool surface stable while the implementation
//! is organized into feature-focused leaf modules.

mod agentic_nav;
pub mod audit;
mod context;
mod deployment;
mod docs;
mod forwarder;
mod remediation;
mod search;
pub mod semantic_check;
mod semantic_edit;
mod semantic_read;
pub mod sentinel;
mod xml_lite;

pub use agentic_nav::WendaoAgenticNavTool;
pub use audit::{audit_search_payload, evaluate_alignment};
pub use context::WendaoContextExt;
pub(crate) use context::resolve_docs_tool_runtime;
pub use deployment::{
    WendaoPluginArtifactArgs, WendaoPluginArtifactOutputFormat, WendaoPluginArtifactTool,
    export_plugin_artifact, render_plugin_artifact, render_plugin_artifact_json,
    render_plugin_artifact_toml, wendao_plugin_artifact,
};
pub use docs::{
    WendaoDocsGetDocumentArgs, WendaoDocsGetDocumentNodeArgs, WendaoDocsGetDocumentNodeTool,
    WendaoDocsGetDocumentSegmentArgs, WendaoDocsGetDocumentSegmentTool,
    WendaoDocsGetDocumentStructureArgs, WendaoDocsGetDocumentStructureCatalogArgs,
    WendaoDocsGetDocumentStructureCatalogTool, WendaoDocsGetDocumentStructureOutlineArgs,
    WendaoDocsGetDocumentStructureOutlineTool, WendaoDocsGetDocumentStructureTool,
    WendaoDocsGetDocumentTool, WendaoDocsGetNavigationArgs, WendaoDocsGetNavigationTool,
    WendaoDocsGetRetrievalContextArgs, WendaoDocsGetRetrievalContextTool,
    WendaoDocsGetTocDocumentsArgs, WendaoDocsGetTocDocumentsTool,
    WendaoDocsSearchDocumentStructureArgs, WendaoDocsSearchDocumentStructureTool,
    register_wendao_docs_native_tools, wendao_docs_get_document, wendao_docs_get_document_node,
    wendao_docs_get_document_segment, wendao_docs_get_document_structure,
    wendao_docs_get_document_structure_catalog, wendao_docs_get_document_structure_outline,
    wendao_docs_get_navigation, wendao_docs_get_retrieval_context, wendao_docs_get_toc_documents,
    wendao_docs_search_document_structure,
};
pub use forwarder::{
    AffectedDocInfo, ForwardNotification, ForwardNotifier, ForwarderConfig, SuggestedAction,
};
pub use remediation::{
    RemediationAction, RemediationConfig, RemediationContextExt, RemediationResult,
    RemediationWorker,
};
pub use search::{WendaoSearchArgs, WendaoSearchTool, render_xml_lite_hits, wendao_search};
pub use semantic_check::WendaoSemanticCheckTool;
pub use semantic_edit::{WendaoSemanticEditArgs, WendaoSemanticEditTool, wendao_semantic_edit};
pub use semantic_read::{WendaoSemanticReadArgs, WendaoSemanticReadTool, wendao_semantic_read};
pub use sentinel::{
    AffectedDoc, DriftConfidence, ObservationBus, ObservationRef, ObservationSignal,
    SemanticDriftSignal, propagate_source_change, signals_to_status_batch,
};
