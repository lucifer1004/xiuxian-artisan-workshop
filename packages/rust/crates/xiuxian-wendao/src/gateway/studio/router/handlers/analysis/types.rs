use serde::Deserialize;

/// Query parameters for Markdown analysis.
#[derive(Debug, Deserialize)]
pub struct MarkdownAnalysisQuery {
    /// The repository-relative path to the Markdown file.
    pub path: Option<String>,
}

/// Query parameters for Code AST analysis.
#[derive(Debug, Deserialize)]
pub struct CodeAstAnalysisQuery {
    /// The repository-relative path to the source file.
    pub path: Option<String>,
    /// Optional repository identifier.
    pub repo: Option<String>,
    /// Optional 1-based line number for focused analysis.
    pub line: Option<usize>,
}
