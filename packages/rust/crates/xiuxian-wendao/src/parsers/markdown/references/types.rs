use serde::{Deserialize, Serialize};

/// The concrete reference syntax found in Markdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MarkdownReferenceKind {
    /// A standard Markdown inline link such as `[label](target)`.
    Markdown,
    /// A wiki-style link such as `[[target]]` or `[[target|alias]]`.
    WikiLink,
}

/// One ordinary Markdown reference with structural target coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownReference {
    /// Which reference syntax produced this record.
    pub kind: MarkdownReferenceKind,
    /// The note or resource target when one is present.
    #[serde(default)]
    pub target: Option<String>,
    /// The heading, block, or local address fragment when one is present.
    #[serde(default)]
    pub target_address: Option<String>,
    /// The exact source literal that produced this reference record.
    pub original: String,
}

impl MarkdownReference {
    #[must_use]
    pub(in crate::parsers::markdown::references) fn new(
        kind: MarkdownReferenceKind,
        target: Option<String>,
        target_address: Option<String>,
        original: String,
    ) -> Self {
        Self {
            kind,
            target,
            target_address,
            original,
        }
    }
}
