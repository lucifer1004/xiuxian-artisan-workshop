use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Deterministic projected page family derived from Stage-1 repository truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectionPageKind {
    /// Reference-oriented projected page.
    Reference,
    /// How-to oriented projected page.
    HowTo,
    /// Tutorial-oriented projected page.
    Tutorial,
    /// Explanation-oriented projected page.
    Explanation,
}

/// One deterministic projected-page seed produced from repository truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectionPageSeed {
    /// Owning repository identifier.
    pub repo_id: String,
    /// Stable projected page identifier.
    pub page_id: String,
    /// Diataxis-aligned page family.
    pub kind: ProjectionPageKind,
    /// Human-readable projected page title.
    pub title: String,
    /// Attached module anchors.
    pub module_ids: Vec<String>,
    /// Attached symbol anchors.
    pub symbol_ids: Vec<String>,
    /// Attached example anchors.
    pub example_ids: Vec<String>,
    /// Attached documentation anchors.
    pub doc_ids: Vec<String>,
    /// Repository-relative source paths contributing to the page.
    pub paths: Vec<String>,
    /// Format hints carried forward from source docs.
    pub format_hints: Vec<String>,
}

/// Bundle of deterministic projection seeds emitted from Stage-1 records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectionInputBundle {
    /// Owning repository identifier.
    pub repo_id: String,
    /// All projected page seeds sorted deterministically.
    pub pages: Vec<ProjectionPageSeed>,
}

/// One deterministic projected page section emitted from a projection seed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectedPageSection {
    /// Stable section identifier scoped to the owning page.
    pub section_id: String,
    /// Human-readable section title.
    pub title: String,
    /// Heading level intended for downstream page-index ingestion.
    pub level: usize,
    /// Deterministic section body.
    pub body: String,
    /// Source paths contributing to this section.
    pub paths: Vec<String>,
}

/// Deterministic projected page record derived from stage-one repository truth.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectedPageRecord {
    /// Owning repository identifier.
    pub repo_id: String,
    /// Stable projected page identifier.
    pub page_id: String,
    /// Diataxis-aligned page family.
    pub kind: ProjectionPageKind,
    /// Human-readable projected page title.
    pub title: String,
    /// Attached module anchors.
    pub module_ids: Vec<String>,
    /// Attached symbol anchors.
    pub symbol_ids: Vec<String>,
    /// Attached example anchors.
    pub example_ids: Vec<String>,
    /// Attached documentation anchors.
    pub doc_ids: Vec<String>,
    /// Repository-relative source paths contributing to the page.
    pub paths: Vec<String>,
    /// Format hints carried forward from source docs.
    pub format_hints: Vec<String>,
    /// Deterministic page sections for downstream indexing or rendering.
    pub sections: Vec<ProjectedPageSection>,
}

/// Deterministic markdown document rendered from a projected page record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectedMarkdownDocument {
    /// Owning repository identifier.
    pub repo_id: String,
    /// Stable projected page identifier.
    pub page_id: String,
    /// Stable virtual markdown path used for downstream parsing.
    pub path: String,
    /// Human-readable page title.
    pub title: String,
    /// Deterministic markdown body.
    pub markdown: String,
}

/// Page-index-ready section summary parsed from a projected markdown document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectedPageIndexSection {
    /// Hierarchical heading path as parsed by the markdown parser.
    pub heading_path: String,
    /// Leaf heading title.
    pub title: String,
    /// Markdown heading depth.
    pub level: usize,
    /// Inclusive 1-based source line range.
    pub line_range: (usize, usize),
    /// Property drawer attributes parsed for this heading.
    pub attributes: Vec<(String, String)>,
}

/// Page-index-ready parsed document derived from a projected markdown document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectedPageIndexDocument {
    /// Owning repository identifier.
    pub repo_id: String,
    /// Stable projected page identifier.
    pub page_id: String,
    /// Virtual markdown path used for parsing.
    pub path: String,
    /// Parsed document identifier as seen by the markdown parser.
    pub doc_id: String,
    /// Human-readable page title.
    pub title: String,
    /// Parsed section summaries ready for page-index construction.
    pub sections: Vec<ProjectedPageIndexSection>,
}

/// One projected page-index node summary derived from the real page-index tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectedPageIndexNode {
    /// Stable page-index node identifier.
    pub node_id: String,
    /// Human-readable node title.
    pub title: String,
    /// Markdown heading depth.
    pub level: usize,
    /// Structural path carried by the page-index builder.
    pub structural_path: Vec<String>,
    /// Inclusive 1-based source line range.
    pub line_range: (usize, usize),
    /// Best-effort token count after optional thinning.
    pub token_count: usize,
    /// Whether the node was thinned.
    pub is_thinned: bool,
    /// Node text payload after optional thinning.
    pub text: String,
    /// Child nodes.
    pub children: Vec<ProjectedPageIndexNode>,
}

/// One projected page-index tree summary derived from a projected markdown page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectedPageIndexTree {
    /// Owning repository identifier.
    pub repo_id: String,
    /// Stable projected page identifier.
    pub page_id: String,
    /// Virtual markdown path used for parsing.
    pub path: String,
    /// Parsed document identifier as seen by the markdown parser.
    pub doc_id: String,
    /// Human-readable page title.
    pub title: String,
    /// Root node count.
    pub root_count: usize,
    /// Root tree nodes.
    pub roots: Vec<ProjectedPageIndexNode>,
}
