//! Studio API types for TypeScript bindings and HTTP endpoints.
//!
//! This module defines all types used by the Qianji Studio frontend API,
//! including VFS operations, graph queries, search, and UI configuration.

mod analysis;
mod attachments;
mod code_ast;
mod config;
mod definitions;
mod error;
mod graph;
mod navigation;
mod search;
mod search_index;
mod symbols;
mod vfs;

use specta::TypeCollection;

pub use analysis::{
    AnalysisEdge, AnalysisEdgeKind, AnalysisEvidence, AnalysisNode, AnalysisNodeKind,
    MarkdownAnalysisResponse, MermaidProjection, MermaidViewKind,
};
pub use attachments::{AttachmentSearchHit, AttachmentSearchResponse};
pub use code_ast::{
    CodeAstAnalysisResponse, CodeAstEdge, CodeAstEdgeKind, CodeAstNode, CodeAstNodeKind,
    CodeAstProjection, CodeAstProjectionKind,
};
pub use config::{UiConfig, UiProjectConfig, UiRepoProjectConfig};
pub use definitions::{
    AstSearchHit, AstSearchResponse, DefinitionResolveResponse, DefinitionSearchHit,
    ObservationHint, ReferenceSearchHit, ReferenceSearchResponse,
};
pub use error::ApiError;
pub use graph::{
    GraphLink, GraphNeighborsResponse, GraphNode, NodeNeighbors, Topology3dPayload,
    TopologyCluster, TopologyLink, TopologyNode,
};
pub use navigation::StudioNavigationTarget;
pub use search::{
    IntentSearchHit, KnowledgeSearchHit, SearchBacklinkItem, SearchHit, SearchResponse,
};
pub use search_index::{
    SearchCorpusIndexStatus, SearchIndexMaintenanceStatus, SearchIndexPhase,
    SearchIndexStatusResponse,
};
pub use symbols::{
    AutocompleteHit, AutocompleteResponse, AutocompleteSuggestion, SymbolSearchHit,
    SymbolSearchResponse,
};
pub use vfs::{VfsCategory, VfsContentResponse, VfsEntry, VfsScanEntry, VfsScanResult};

/// Build the Studio Specta type collection used by `export_types`.
#[must_use]
pub fn studio_type_collection() -> TypeCollection {
    TypeCollection::default()
}
