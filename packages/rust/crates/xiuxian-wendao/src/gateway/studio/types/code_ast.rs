use serde::{Deserialize, Serialize};
use specta::Type;

/// Kind of a code-AST node.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CodeAstNodeKind {
    /// Module/namespace container.
    Module,
    /// Function/method declaration.
    Function,
    /// Type/struct/class declaration.
    Type,
    /// Constant declaration.
    Constant,
    /// External symbol imported from outside the file.
    ExternalSymbol,
    /// Other AST entities.
    Other,
}

/// Kind of a code-AST edge.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CodeAstEdgeKind {
    /// Ownership / nesting relation.
    Contains,
    /// Call relation.
    Calls,
    /// Usage relation.
    Uses,
    /// Import relation.
    Imports,
    /// Other relation.
    Other,
}

/// Kind of an AST projection view.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CodeAstProjectionKind {
    /// Containment projection.
    Contains,
    /// Call-graph projection.
    Calls,
    /// Usage projection.
    Uses,
}

/// A single AST node entry for diagram rendering.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CodeAstNode {
    /// Node identifier.
    pub id: String,
    /// Display label.
    pub label: String,
    /// Semantic node kind.
    pub kind: CodeAstNodeKind,
    /// Optional source path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Optional 1-based source line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
}

/// A single AST edge entry for diagram rendering.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CodeAstEdge {
    /// Edge identifier.
    pub id: String,
    /// Source node identifier.
    pub source_id: String,
    /// Target node identifier.
    pub target_id: String,
    /// Semantic edge kind.
    pub kind: CodeAstEdgeKind,
    /// Optional display label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Precomputed AST projection metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CodeAstProjection {
    /// Projection category.
    pub kind: CodeAstProjectionKind,
    /// Number of nodes included in projection.
    pub node_count: usize,
    /// Number of edges included in projection.
    pub edge_count: usize,
}

/// Response payload for code AST analysis.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CodeAstAnalysisResponse {
    /// Repository identifier.
    pub repo_id: String,
    /// Repository-relative source path.
    pub path: String,
    /// Source language.
    pub language: String,
    /// AST nodes.
    pub nodes: Vec<CodeAstNode>,
    /// AST edges.
    pub edges: Vec<CodeAstEdge>,
    /// Projection summaries.
    pub projections: Vec<CodeAstProjection>,
    /// Optional node identifier selected by line hint.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus_node_id: Option<String>,
    /// Diagnostics emitted by parser/analyzer.
    pub diagnostics: Vec<String>,
}
