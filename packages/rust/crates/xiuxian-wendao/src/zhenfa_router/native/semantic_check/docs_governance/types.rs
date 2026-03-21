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

/// A slice of a line in a document.
#[derive(Debug, Clone, Copy)]
pub struct LineSlice<'a> {
    pub line_number: usize,
    pub start_offset: usize,
    pub end_offset: usize,
    pub trimmed: &'a str,
    pub without_newline: &'a str,
    pub newline: &'a str,
}

/// Metadata about a documentation section.
#[derive(Debug, Clone)]
pub struct SectionSpec {
    pub section_name: &'static str,
    pub relative_path: String,
    pub title: String,
    pub doc_type: &'static str,
}

/// Parsed top properties drawer.
#[derive(Debug, Clone, Copy)]
pub struct TopPropertiesDrawer<'a> {
    pub properties_line: usize,
    pub insert_offset: usize,
    pub newline: &'a str,
    pub id_line: Option<IdLine<'a>>,
}

/// Parsed :ID: line in a properties drawer.
#[derive(Debug, Clone, Copy)]
pub struct IdLine<'a> {
    pub line: usize,
    pub value: &'a str,
    pub value_start: usize,
    pub value_end: usize,
}

/// Parsed :LINKS: line in a relations block.
#[derive(Debug, Clone, Copy)]
pub struct LinksLine<'a> {
    pub line: usize,
    pub value: &'a str,
    pub value_start: usize,
    pub value_end: usize,
}

/// Parsed :FOOTER: block.
#[derive(Debug, Clone, Copy)]
pub struct FooterBlock<'a> {
    pub line: usize,
    pub start_offset: usize,
    pub end_offset: usize,
    pub standards_value: Option<&'a str>,
    pub last_sync_value: Option<&'a str>,
}
