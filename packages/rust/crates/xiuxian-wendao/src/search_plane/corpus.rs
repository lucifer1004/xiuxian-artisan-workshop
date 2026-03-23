use serde::{Deserialize, Serialize};

/// Canonical corpus partitions in the Studio search plane.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchCorpusKind {
    /// Knowledge sections derived from the link graph.
    KnowledgeSection,
    /// Attachments associated with indexed knowledge documents.
    Attachment,
    /// Local workspace symbols derived from AST extraction.
    LocalSymbol,
    /// Reference occurrences materialized from source scanning.
    ReferenceOccurrence,
    /// Repository intelligence entities such as modules, symbols, and examples.
    RepoEntity,
    /// Repository content chunks used for fallback code search.
    RepoContentChunk,
}

impl SearchCorpusKind {
    /// Stable iteration order for all supported corpora.
    pub const ALL: [Self; 6] = [
        Self::KnowledgeSection,
        Self::Attachment,
        Self::LocalSymbol,
        Self::ReferenceOccurrence,
        Self::RepoEntity,
        Self::RepoContentChunk,
    ];

    /// Canonical storage and API identifier for the corpus.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::KnowledgeSection => "knowledge_section",
            Self::Attachment => "attachment",
            Self::LocalSymbol => "local_symbol",
            Self::ReferenceOccurrence => "reference_occurrence",
            Self::RepoEntity => "repo_entity",
            Self::RepoContentChunk => "repo_content_chunk",
        }
    }

    /// Current schema version for the corpus table layout.
    #[must_use]
    pub const fn schema_version(self) -> u32 {
        match self {
            Self::LocalSymbol => 2,
            Self::KnowledgeSection
            | Self::Attachment
            | Self::ReferenceOccurrence
            | Self::RepoEntity
            | Self::RepoContentChunk => 1,
        }
    }
}

impl std::fmt::Display for SearchCorpusKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str((*self).as_str())
    }
}
