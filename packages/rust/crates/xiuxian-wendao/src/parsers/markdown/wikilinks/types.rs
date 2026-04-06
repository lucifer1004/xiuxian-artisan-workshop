use serde::{Deserialize, Serialize};

/// One ordinary body wikilink extracted from Markdown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarkdownWikiLink {
    /// Optional note or resource target without any address fragment.
    #[serde(default)]
    pub target: Option<String>,
    /// Optional Obsidian-style heading or block address.
    #[serde(default)]
    pub target_address: Option<String>,
    /// Original literal slice from the Markdown source.
    pub original: String,
}

impl MarkdownWikiLink {
    #[must_use]
    pub(in crate::parsers::markdown::wikilinks) fn new(
        target: Option<String>,
        target_address: Option<String>,
        original: String,
    ) -> Self {
        Self {
            target,
            target_address,
            original,
        }
    }
}
