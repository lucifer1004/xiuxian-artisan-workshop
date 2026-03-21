//! Studio API types for TypeScript bindings and HTTP endpoints.
//!
//! This module defines all types used by the Qianji Studio frontend API,
//! including VFS operations, graph queries, search, and UI configuration.

use serde::{Deserialize, Serialize};
use specta::{Type, TypeCollection};

// === VFS Types ===

/// A single entry in the VFS (file or directory).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VfsEntry {
    /// Full path relative to the VFS root.
    pub path: String,
    /// File or directory name.
    pub name: String,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// File size in bytes (0 for directories).
    pub size: u64,
    /// Last modified timestamp (Unix seconds).
    pub modified: u64,
    /// MIME content type guess for files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// Project grouping label for multi-root monorepo views.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    /// Root label under the grouped project node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_label: Option<String>,
    /// Configured project root used to resolve this VFS root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    /// Configured project directories associated with the resolved root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_dirs: Option<Vec<String>>,
}

/// Category classification for VFS entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum VfsCategory {
    /// Directory/folder.
    Folder,
    /// Skill definition file.
    Skill,
    /// Documentation file.
    Doc,
    /// Knowledge base file.
    Knowledge,
    /// Other/uncategorized file.
    Other,
}

/// A scanned entry with metadata for VFS tree display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VfsScanEntry {
    /// Full path relative to the VFS root.
    pub path: String,
    /// File or directory name.
    pub name: String,
    /// Whether this entry is a directory.
    pub is_dir: bool,
    /// Category classification for UI styling.
    pub category: VfsCategory,
    /// File size in bytes (0 for directories).
    pub size: u64,
    /// Last modified timestamp (Unix seconds).
    pub modified: u64,
    /// MIME content type guess for files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    /// Whether the file has YAML frontmatter.
    pub has_frontmatter: bool,
    /// Wendao document ID if indexed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wendao_id: Option<String>,
    /// Project grouping label for multi-root monorepo views.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    /// Root label under the grouped project node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_label: Option<String>,
    /// Configured project root used to resolve this VFS root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    /// Configured project directories associated with the resolved root.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_dirs: Option<Vec<String>>,
}

/// Result of a VFS scan operation.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VfsScanResult {
    /// All entries found during the scan.
    pub entries: Vec<VfsScanEntry>,
    /// Total number of files scanned.
    pub file_count: usize,
    /// Total number of directories scanned.
    pub dir_count: usize,
    /// Time taken for the scan in milliseconds.
    pub scan_duration_ms: u64,
}

/// Payload for file content operations.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VfsContentResponse {
    /// Full path to the file.
    pub path: String,
    /// MIME content type.
    pub content_type: String,
    /// Raw file content.
    pub content: String,
    /// File modification timestamp.
    pub modified: u64,
}

// === Graph Types ===

/// A single node in the link-graph visualization.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphNode {
    /// Global identifier for the node.
    pub id: String,
    /// Display label.
    pub label: String,
    /// File path if the node represents a document.
    pub path: String,
    /// Display-ready navigation target.
    pub navigation_target: StudioNavigationTarget,
    /// Optional node type (e.g., "CORE", "FEATURE").
    pub node_type: String,
    /// Whether this is the focal node of the query.
    pub is_center: bool,
    /// Shortest-path distance from the center node.
    pub distance: usize,
}

/// A single edge in the link-graph visualization.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphEdge {
    /// Global identifier for the edge.
    pub id: String,
    /// Semantic kind of the relationship.
    pub kind: String,
    /// Source node identifier.
    pub source_id: String,
    /// Target node identifier.
    pub target_id: String,
    /// Display label for the relationship.
    pub label: String,
}

/// Result of a graph neighbor traversal.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct GraphNeighborsResult {
    /// Nodes in the neighbor subgraph.
    pub nodes: Vec<GraphNode>,
    /// Edes connecting the neighbors.
    pub edges: Vec<GraphEdge>,
}

/// Payload for 3D graph topology visualization.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Topology3dPayload {
    /// All nodes in the global graph.
    pub nodes: Vec<GraphNode>,
    /// All edges in the global graph.
    pub links: Vec<GraphEdge>,
}

// === Search Types ===

/// Navigation target for opening files/symbols in the Studio editor.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct StudioNavigationTarget {
    /// Full path or URI.
    pub path: String,
    /// Navigation category (e.g., "doc", "symbol").
    pub category: String,
    /// Optional project label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    /// Optional root label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_label: Option<String>,
    /// 1-based line number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    /// 1-based end line number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_end: Option<usize>,
    /// 1-based column number.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
}

/// A single hit in a knowledge base search.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct KnowledgeSearchHit {
    /// Global node identifier.
    pub id: String,
    /// Display label.
    pub label: String,
    /// File path.
    pub path: String,
    /// Navigation target.
    pub navigation_target: StudioNavigationTarget,
    /// Semantic score (0.0 - 1.0).
    pub score: f64,
    /// Snippet highlighting matching terms.
    pub snippet: String,
}

/// Structured backlink metadata surfaced on search hits.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchBacklinkItem {
    /// Stable backlink identifier.
    pub id: String,
    /// Optional display title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional source path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Optional relation kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
}

/// Unified search hit consumed by the frontend search surface.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    /// Stable stem or primary identifier.
    pub stem: String,
    /// Optional display title.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Repository-relative or workspace-relative path.
    pub path: String,
    /// Optional logical hit kind.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc_type: Option<String>,
    /// Search-visible tags.
    pub tags: Vec<String>,
    /// Normalized score.
    pub score: f64,
    /// Optional best section or signature summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub best_section: Option<String>,
    /// Optional match-reason string.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub match_reason: Option<String>,
    /// Optional hierarchical URI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hierarchical_uri: Option<String>,
    /// Optional hierarchy segments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hierarchy: Option<Vec<String>>,
    /// Optional saliency score.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub saliency_score: Option<f64>,
    /// Optional audit status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audit_status: Option<String>,
    /// Optional verification state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verification_state: Option<String>,
    /// Optional backlink identifiers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implicit_backlinks: Option<Vec<String>>,
    /// Optional structured backlink items.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implicit_backlink_items: Option<Vec<SearchBacklinkItem>>,
    /// Optional navigation target.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub navigation_target: Option<StudioNavigationTarget>,
}

/// Unified search response consumed by the frontend search shell.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    /// Original query string.
    pub query: String,
    /// Matching hits.
    pub hits: Vec<SearchHit>,
    /// Total number of hits returned.
    pub hit_count: usize,
    /// Optional graph confidence score.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph_confidence_score: Option<f64>,
    /// Optional selected mode label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_mode: Option<String>,
    /// Optional resolved intent label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    /// Optional resolved intent confidence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent_confidence: Option<f64>,
    /// Optional backend search mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_mode: Option<String>,
    /// Whether the backend returned partial results because repo indexes are still warming.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub partial: bool,
    /// Optional aggregate indexing state for code search.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indexing_state: Option<String>,
    /// Repo ids that are still queued or indexing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub pending_repos: Vec<String>,
    /// Repo ids skipped because their repo index is unsupported or failed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skipped_repos: Vec<String>,
}

/// A hit derived from search intent hints (e.g., task-oriented).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IntentSearchHit {
    /// Display label for the intent.
    pub label: String,
    /// Target semantic action.
    pub action: String,
    /// Score indicating intent alignment.
    pub score: f64,
}

/// A hit representing an attachment or external resource.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentSearchHit {
    /// Attachment filename.
    pub name: String,
    /// Relative path.
    pub path: String,
    /// Stable source document identifier.
    pub source_id: String,
    /// Source document stem.
    pub source_stem: String,
    /// Source document title.
    pub source_title: String,
    /// Source document path.
    pub source_path: String,
    /// Stable attachment identifier.
    pub attachment_id: String,
    /// Relative attachment path.
    pub attachment_path: String,
    /// Attachment display name.
    pub attachment_name: String,
    /// Lowercased attachment extension without leading dot.
    pub attachment_ext: String,
    /// Attachment kind label.
    pub kind: String,
    /// Navigation target.
    pub navigation_target: StudioNavigationTarget,
    /// Relevance score.
    pub score: f64,
    /// Optional OCR or vision snippet for the attachment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vision_snippet: Option<String>,
}

/// Response for Studio attachment search queries.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentSearchResponse {
    /// Original query string.
    pub query: String,
    /// Matching attachment hits.
    pub hits: Vec<AttachmentSearchHit>,
    /// Total number of hits returned.
    pub hit_count: usize,
    /// Selected attachment scope label.
    pub selected_scope: String,
}

/// A single hit in an AST definition search.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AstSearchHit {
    /// Captured definition name.
    pub name: String,
    /// Signature line or skeleton snippet.
    pub signature: String,
    /// Source file path relative to the project root.
    pub path: String,
    /// Source language name.
    pub language: String,
    /// Owning crate or package name.
    pub crate_name: String,
    /// Configured project name when the source path maps to a studio project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    /// Configured root label when the source path maps to a project root path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_label: Option<String>,
    /// Optional AST node kind for richer Markdown search presentation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_kind: Option<String>,
    /// Optional owning Markdown section title/path for property-box derived hits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_title: Option<String>,
    /// Display-ready navigation target for opening this hit in studio.
    pub navigation_target: StudioNavigationTarget,
    /// 1-based start line.
    pub line_start: usize,
    /// 1-based end line.
    pub line_end: usize,
    /// Search relevance score.
    pub score: f64,
}

/// Result of a best-definition resolution.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionSearchHit {
    /// Symbol or definition name.
    pub name: String,
    /// Display signature for the definition.
    pub signature: String,
    /// Repository-relative path to the definition.
    pub path: String,
    /// Source language label for the definition.
    pub language: String,
    /// Owning crate or repository identifier.
    pub crate_name: String,
    /// Optional project grouping label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    /// Optional root label derived from configured project scopes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_label: Option<String>,
    /// Optional AST node kind for the resolved symbol.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_kind: Option<String>,
    /// Optional owner title or containing symbol label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner_title: Option<String>,
    /// Navigation target for opening the definition in Studio.
    pub navigation_target: StudioNavigationTarget,
    /// 1-based starting line for the definition span.
    pub line_start: usize,
    /// 1-based ending line for the definition span.
    pub line_end: usize,
    /// Resolution score assigned to this candidate.
    pub score: f64,
    /// Hints derived from :OBSERVE: property boxes.
    pub observation_hints: Vec<ObservationHint>,
}

/// A hint for observing code patterns near a definition.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ObservationHint {
    /// Language constraint (e.g., "rust").
    pub language: String,
    /// File path scope (e.g., "src/**").
    pub scope: String,
    /// Pattern to observe.
    pub pattern: String,
}

/// Response for studio AST definition search queries.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AstSearchResponse {
    /// Original query string.
    pub query: String,
    /// Matching AST hits.
    pub hits: Vec<AstSearchHit>,
    /// Total number of hits returned.
    pub hit_count: usize,
    /// Selected AST scope.
    pub selected_scope: String,
}

/// Response for native studio definition resolution.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DefinitionResolveResponse {
    /// Original query string.
    pub query: String,
    /// Optional source path used to bias resolution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    /// Optional source line used by the caller.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_line: Option<usize>,
    /// Number of candidate definitions considered for this resolution.
    pub candidate_count: usize,
    /// The selected scope used to resolve the definition.
    pub selected_scope: String,
    /// Display-ready navigation target for the resolved definition.
    pub navigation_target: StudioNavigationTarget,
    /// The resolved definition hit.
    pub definition: DefinitionSearchHit,
    /// Display-ready navigation target for the resolved definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_target: Option<StudioNavigationTarget>,
    /// The actual hit that was resolved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_hit: Option<DefinitionSearchHit>,
}

/// A hit indicating where a symbol is referenced or used.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceSearchHit {
    /// Symbol name being referenced.
    pub name: String,
    /// Referencing file path.
    pub path: String,
    /// Language of the referencing file.
    pub language: String,
    /// Crate name of the referencing file.
    pub crate_name: String,
    /// Project grouping label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    /// Root label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_label: Option<String>,
    /// Navigation target for the reference site.
    pub navigation_target: StudioNavigationTarget,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number.
    pub column: usize,
    /// Snippet showing matching line.
    pub line_text: String,
    /// Scoring weight.
    pub score: f64,
}

/// Response for Studio reference search queries.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceSearchResponse {
    /// Original query string.
    pub query: String,
    /// Matching reference hits.
    pub hits: Vec<ReferenceSearchHit>,
    /// Total number of hits returned.
    pub hit_count: usize,
    /// Selected reference scope label.
    pub selected_scope: String,
}

/// A hit in a project-wide symbol index (e.g. Tantivy-backed).
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SymbolSearchHit {
    /// Symbol name.
    pub name: String,
    /// Symbol kind (e.g. "fn", "struct").
    pub kind: String,
    /// Display path.
    pub path: String,
    /// 1-based line number for the symbol location.
    pub line: usize,
    /// Canonical `path:line` location string.
    pub location: String,
    /// Source language label inferred from the symbol path.
    pub language: String,
    /// Source identifier (e.g. "project", "external").
    pub source: String,
    /// Owning crate name.
    pub crate_name: String,
    /// Project grouping label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_name: Option<String>,
    /// Root label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_label: Option<String>,
    /// Navigation target.
    pub navigation_target: StudioNavigationTarget,
    /// Semantic score.
    pub score: f64,
}

/// Response for Studio symbol search queries.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SymbolSearchResponse {
    /// Original query string.
    pub query: String,
    /// Matching symbol hits.
    pub hits: Vec<SymbolSearchHit>,
    /// Total number of hits returned.
    pub hit_count: usize,
    /// Selected symbol scope label.
    pub selected_scope: String,
}

/// A suggested autocomplete entry.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AutocompleteHit {
    /// Suggestion text.
    pub label: String,
    /// Category classification for icons.
    pub category: String,
    /// Optional snippet or description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// A single autocomplete suggestion.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AutocompleteSuggestion {
    /// Suggestion text emitted to the caller.
    pub text: String,
    /// Logical suggestion classification.
    pub suggestion_type: String,
}

/// Response for Studio autocomplete queries.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AutocompleteResponse {
    /// Original prefix used to generate suggestions.
    pub prefix: String,
    /// Ranked autocomplete suggestions.
    pub suggestions: Vec<AutocompleteSuggestion>,
}

// === Analysis Types ===

/// Kind of an analysis node.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AnalysisNodeKind {
    /// Markdown section heading.
    Section,
    /// Task list item.
    Task,
    /// Observation/evidence block.
    Observation,
    /// Symbolic link or relation.
    Relation,
    /// Document-level node.
    Document,
    /// Code block node.
    CodeBlock,
    /// Semantic reference site.
    Reference,
    /// Property box node.
    Property,
    /// Symbolic entity node.
    Symbol,
}

/// Kind of an analysis edge.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AnalysisEdgeKind {
    /// Parent-child hierarchy.
    Parent,
    /// Semantic reference or mention.
    Mentions,
    /// Document membership.
    Contains,
    /// Next task in sequence.
    NextStep,
    /// Explicit document reference.
    References,
}

/// Metadata about an analysis edge evidence.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisEvidence {
    /// Evidence file path.
    pub path: String,
    /// 1-based start line.
    pub line_start: usize,
    /// 1-based end line.
    pub line_end: usize,
    /// Confidence score (0.0 - 1.0).
    pub confidence: f64,
}

/// A single node in the structural IR of a document.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisNode {
    /// Node identifier.
    pub id: String,
    /// Node kind.
    pub kind: AnalysisNodeKind,
    /// Display label.
    pub label: String,
    /// Nesting depth.
    pub depth: usize,
    /// 1-based start line.
    pub line_start: usize,
    /// 1-based end line.
    pub line_end: usize,
    /// Optional parent node identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
}

/// A relationship edge in the document IR.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AnalysisEdge {
    /// Edge identifier.
    pub id: String,
    /// Source node identifier.
    pub source_id: String,
    /// Target node identifier.
    pub target_id: String,
    /// Relationship kind.
    pub kind: AnalysisEdgeKind,
    /// Display label.
    pub label: String,
    /// Evidence metadata.
    pub evidence: AnalysisEvidence,
}

/// Full response for Markdown analysis.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MarkdownAnalysisResponse {
    /// Analyzed file path.
    pub path: String,
    /// Content fingerprint.
    pub document_hash: String,
    /// Total number of nodes.
    pub node_count: usize,
    /// Total number of edges.
    pub edge_count: usize,
    /// IR nodes.
    pub nodes: Vec<AnalysisNode>,
    /// IR edges.
    pub edges: Vec<AnalysisEdge>,
    /// Mermaid diagram projections.
    pub projections: Vec<MermaidProjection>,
    /// Analysis diagnostics.
    pub diagnostics: Vec<String>,
}

/// Mermaid projection view kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Type)]
#[serde(rename_all = "lowercase")]
pub enum MermaidViewKind {
    /// Hierarchical document outline.
    Outline,
    /// Task dependency graph.
    Tasks,
    /// Semantic entity relations.
    Knowledge,
}

/// A single Mermaid diagram projection.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct MermaidProjection {
    /// Type of projection.
    pub kind: MermaidViewKind,
    /// Generated Mermaid source.
    pub source: String,
    /// Number of nodes in projection.
    pub node_count: usize,
    /// Number of edges in projection.
    pub edge_count: usize,
}

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

// === Configuration Types ===

/// Global UI configuration for Studio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type, Default)]
#[serde(rename_all = "camelCase")]
pub struct UiConfig {
    /// Local project roots to scan.
    pub projects: Vec<UiProjectConfig>,
    /// External repository projects.
    pub repo_projects: Vec<UiRepoProjectConfig>,
}

/// Configuration for a local project root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UiProjectConfig {
    /// Unique name.
    pub name: String,
    /// Relative path to project root.
    pub root: String,
    /// Explicit subdirectories to index.
    pub dirs: Vec<String>,
}

/// Configuration for an external analyzed repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UiRepoProjectConfig {
    /// Unique identifier.
    pub id: String,
    /// Optional local path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
    /// Optional upstream URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Optional git reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_ref: Option<String>,
    /// Refresh policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh: Option<String>,
    /// Enabled analysis plugins.
    pub plugins: Vec<String>,
}

/// Base error for Studio API.
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    /// Machine-readable error code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
    /// Optional failure details.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// Build the Studio Specta type collection used by `export_types`.
#[must_use]
pub fn studio_type_collection() -> TypeCollection {
    TypeCollection::default()
}
