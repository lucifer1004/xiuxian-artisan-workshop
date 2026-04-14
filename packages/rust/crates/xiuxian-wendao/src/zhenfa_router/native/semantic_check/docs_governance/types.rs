//! Shared types for docs governance.

/// Issue type for document identity protocol violations.
pub const DOC_IDENTITY_PROTOCOL_ISSUE_TYPE: &str = "DOC_IDENTITY_PROTOCOL";

/// Issue type for missing package docs index.
pub const MISSING_PACKAGE_DOCS_INDEX_ISSUE_TYPE: &str = "MISSING_PACKAGE_DOCS_INDEX";

/// Issue type for missing package docs index footer block.
pub const MISSING_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE: &str =
    "MISSING_PACKAGE_DOCS_INDEX_FOOTER_BLOCK";

/// Issue type for incomplete package docs index footer block.
pub const INCOMPLETE_PACKAGE_DOCS_INDEX_FOOTER_BLOCK_ISSUE_TYPE: &str =
    "INCOMPLETE_PACKAGE_DOCS_INDEX_FOOTER_BLOCK";

/// Issue type for stale package docs index footer standards.
pub const STALE_PACKAGE_DOCS_INDEX_FOOTER_STANDARDS_ISSUE_TYPE: &str =
    "STALE_PACKAGE_DOCS_INDEX_FOOTER_STANDARDS";

/// Issue type for missing package docs index relations block.
pub const MISSING_PACKAGE_DOCS_INDEX_RELATIONS_BLOCK_ISSUE_TYPE: &str =
    "MISSING_PACKAGE_DOCS_INDEX_RELATIONS_BLOCK";

/// Issue type for missing package docs index relation link.
pub const MISSING_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE: &str =
    "MISSING_PACKAGE_DOCS_INDEX_RELATION_LINK";

/// Issue type for stale package docs index relation link.
pub const STALE_PACKAGE_DOCS_INDEX_RELATION_LINK_ISSUE_TYPE: &str =
    "STALE_PACKAGE_DOCS_INDEX_RELATION_LINK";

/// Issue type for missing package docs index section link.
pub const MISSING_PACKAGE_DOCS_INDEX_SECTION_LINK_ISSUE_TYPE: &str =
    "MISSING_PACKAGE_DOCS_INDEX_SECTION_LINK";

/// Issue type for missing package docs section landing page.
pub const MISSING_PACKAGE_DOCS_SECTION_LANDING_ISSUE_TYPE: &str =
    "MISSING_PACKAGE_DOCS_SECTION_LANDING";

/// Issue type for missing package docs tree.
pub const MISSING_PACKAGE_DOCS_TREE_ISSUE_TYPE: &str = "MISSING_PACKAGE_DOCS_TREE";

/// Issue type for hidden workspace-path links inside canonical docs.
pub const CANONICAL_DOC_HIDDEN_PATH_LINK_ISSUE_TYPE: &str = "CANONICAL_DOC_HIDDEN_PATH_LINK";

/// Metadata about a documentation section.
#[derive(Debug, Clone)]
pub struct SectionSpec {
    /// Canonical section directory name.
    pub section_name: &'static str,
    /// Relative markdown path for the section landing page.
    pub relative_path: String,
    /// Human-readable section title.
    pub title: String,
    /// Stable docs taxonomy label written into the generated page.
    pub doc_type: &'static str,
}
