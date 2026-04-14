use crate::document::MarkdownDocument;
use crate::sections::MarkdownSection;
use serde::{Deserialize, Serialize};

/// Parser-owned reusable table-of-contents aggregate shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(bound(
    serialize = "Document: serde::Serialize, Section: serde::Serialize",
    deserialize = "Document: serde::Deserialize<'de>, Section: serde::Deserialize<'de>"
))]
pub struct TocDocument<Document, Section> {
    /// Parser-owned format wrapper and stripped body.
    pub document: Document,
    /// Parser-owned section structure extracted from the document body.
    #[serde(default)]
    pub sections: Vec<Section>,
}

/// Parser-owned aggregate for one Markdown TOC/body structure.
pub type MarkdownTocDocument = TocDocument<MarkdownDocument, MarkdownSection>;
