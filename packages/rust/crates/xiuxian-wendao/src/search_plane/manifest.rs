use serde::{Deserialize, Serialize};

use super::{SearchCorpusKind, SearchCorpusStatus};

/// Valkey key namespace for search-plane manifests, leases, and caches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchManifestKeyspace {
    namespace: String,
}

impl SearchManifestKeyspace {
    /// Build a keyspace rooted at a caller-supplied namespace prefix.
    #[must_use]
    pub fn new(namespace: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
        }
    }

    /// Return the raw namespace prefix.
    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    /// Key that stores the published manifest for one corpus.
    #[must_use]
    pub fn corpus_manifest_key(&self, corpus: SearchCorpusKind) -> String {
        format!("{}:manifest:{corpus}", self.namespace)
    }

    /// Key that stores the published manifest for one repo-backed corpus rowset.
    #[must_use]
    pub fn repo_corpus_manifest_key(&self, corpus: SearchCorpusKind, repo_id: &str) -> String {
        format!(
            "{}:manifest:{corpus}:repo:{}",
            self.namespace,
            blake3::hash(repo_id.as_bytes()).to_hex()
        )
    }

    /// Lease key used to enforce single-flight indexing for one corpus.
    #[must_use]
    pub fn corpus_lease_key(&self, corpus: SearchCorpusKind) -> String {
        format!("{}:lease:{corpus}", self.namespace)
    }

    /// Key for short-lived query result caching.
    #[must_use]
    pub fn corpus_query_cache_key(&self, corpus: SearchCorpusKind, cache_key: &str) -> String {
        self.search_query_cache_key(corpus.as_str(), cache_key)
    }

    /// Key for short-lived search response caching scoped by logical query surface.
    #[must_use]
    pub fn search_query_cache_key(&self, scope: &str, cache_key: &str) -> String {
        format!("{}:query:{scope}:{cache_key}", self.namespace)
    }

    /// Key for autocomplete prefix caching.
    #[must_use]
    pub fn autocomplete_cache_key(&self, prefix: &str) -> String {
        format!("{}:autocomplete:{prefix}", self.namespace)
    }
}

impl Default for SearchManifestKeyspace {
    fn default() -> Self {
        Self::new("xiuxian:wendao:search_plane")
    }
}

/// Materialized manifest row persisted to Valkey for one corpus.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchManifestRecord {
    /// Corpus this manifest row belongs to.
    pub corpus: SearchCorpusKind,
    /// Active published epoch for readers.
    pub active_epoch: Option<u64>,
    /// Schema version associated with the active epoch.
    pub schema_version: u32,
    /// Current published or in-flight fingerprint.
    pub fingerprint: Option<String>,
    /// RFC3339 time when the manifest was updated.
    pub updated_at: Option<String>,
}

impl SearchManifestRecord {
    /// Project a manifest row from an in-memory coordinator status.
    #[must_use]
    pub fn from_status(status: &SearchCorpusStatus) -> Self {
        Self {
            corpus: status.corpus,
            active_epoch: status.active_epoch,
            schema_version: status.schema_version,
            fingerprint: status.fingerprint.clone(),
            updated_at: status.updated_at.clone(),
        }
    }
}

/// Materialized manifest row for one published repo-backed corpus table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchRepoManifestRecord {
    /// Repo-backed corpus this manifest row belongs to.
    pub corpus: SearchCorpusKind,
    /// Stable repository identifier.
    pub repo_id: String,
    /// Explicit publication token for the currently readable repo-backed table.
    pub publication_id: String,
    /// Table name that currently serves reads for this repo-backed corpus.
    pub table_name: String,
    /// Current table version id published to readers.
    pub table_version_id: u64,
    /// Schema version associated with the published table.
    pub schema_version: u32,
    /// Source revision that produced the published table, when known.
    pub source_revision: Option<String>,
    /// Logical row count for the published table.
    pub row_count: u64,
    /// Fragment count for the published table.
    pub fragment_count: u64,
    /// RFC3339 timestamp of the published table commit.
    pub published_at: String,
}

impl SearchRepoManifestRecord {
    /// Construct a repo publication manifest from one published table snapshot.
    #[must_use]
    pub fn new(
        corpus: SearchCorpusKind,
        repo_id: impl Into<String>,
        table_name: impl Into<String>,
        schema_version: u32,
        source_revision: Option<&str>,
        table_version_id: u64,
        row_count: u64,
        fragment_count: u64,
        published_at: impl Into<String>,
    ) -> Self {
        let repo_id = repo_id.into();
        let table_name = table_name.into();
        let published_at = published_at.into();
        let source_revision = source_revision.map(str::to_string);
        Self {
            corpus,
            publication_id: build_repo_publication_id(
                corpus,
                repo_id.as_str(),
                table_name.as_str(),
                schema_version,
                source_revision.as_deref(),
                table_version_id,
                row_count,
                fragment_count,
                published_at.as_str(),
            ),
            repo_id,
            table_name,
            table_version_id,
            schema_version,
            source_revision,
            row_count,
            fragment_count,
            published_at,
        }
    }

    /// Stable cache/status token that changes only when the published table changes.
    #[must_use]
    pub fn cache_version(&self) -> String {
        format!(
            "{}:schema:{}:repo:{}:publication:{}",
            self.corpus,
            self.schema_version,
            self.repo_id.trim().to_ascii_lowercase(),
            self.publication_id
        )
    }
}

fn build_repo_publication_id(
    corpus: SearchCorpusKind,
    repo_id: &str,
    table_name: &str,
    schema_version: u32,
    source_revision: Option<&str>,
    table_version_id: u64,
    row_count: u64,
    fragment_count: u64,
    published_at: &str,
) -> String {
    let payload = format!(
        "{corpus}|{}|{}|{schema_version}|{}|{table_version_id}|{row_count}|{fragment_count}|{}",
        repo_id.trim().to_ascii_lowercase(),
        table_name.trim().to_ascii_lowercase(),
        source_revision
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase(),
        published_at.trim().to_ascii_lowercase()
    );
    blake3::hash(payload.as_bytes()).to_hex().to_string()
}

/// Stable file-level fingerprint payload for incremental manifest updates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchFileFingerprint {
    /// Repo-relative path for the source file.
    pub relative_path: String,
    /// File size captured during manifest generation.
    pub size_bytes: u64,
    /// Modification time expressed as unix milliseconds.
    pub modified_unix_ms: u64,
    /// Extractor version that produced the manifest row.
    pub extractor_version: u32,
    /// Search-plane schema version associated with the row payload.
    pub schema_version: u32,
    /// Optional content hash used when metadata is insufficient.
    pub blake3: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_manifest_publication_id_changes_when_table_version_changes() {
        let first = SearchRepoManifestRecord::new(
            SearchCorpusKind::RepoEntity,
            "alpha/repo",
            "repo_entity_repo_alpha",
            1,
            Some("rev-1"),
            7,
            10,
            2,
            "2026-03-23T12:00:00Z",
        );
        let second = SearchRepoManifestRecord::new(
            SearchCorpusKind::RepoEntity,
            "alpha/repo",
            "repo_entity_repo_alpha",
            1,
            Some("rev-1"),
            8,
            10,
            2,
            "2026-03-23T12:00:00Z",
        );

        assert_ne!(first.publication_id, second.publication_id);
        assert_ne!(first.cache_version(), second.cache_version());
    }

    #[test]
    fn repo_manifest_cache_version_is_stable_for_same_publication() {
        let first = SearchRepoManifestRecord::new(
            SearchCorpusKind::RepoContentChunk,
            "alpha/repo",
            "repo_content_chunk_repo_alpha",
            1,
            Some("rev-7"),
            3,
            42,
            1,
            "2026-03-23T12:00:00Z",
        );
        let second = SearchRepoManifestRecord::new(
            SearchCorpusKind::RepoContentChunk,
            "alpha/repo",
            "repo_content_chunk_repo_alpha",
            1,
            Some("rev-7"),
            3,
            42,
            1,
            "2026-03-23T12:00:00Z",
        );

        assert_eq!(first.publication_id, second.publication_id);
        assert_eq!(first.cache_version(), second.cache_version());
    }

    #[test]
    fn repo_manifest_publication_id_changes_when_source_revision_changes() {
        let first = SearchRepoManifestRecord::new(
            SearchCorpusKind::RepoContentChunk,
            "alpha/repo",
            "repo_content_chunk_repo_alpha",
            1,
            Some("rev-1"),
            3,
            42,
            1,
            "2026-03-23T12:00:00Z",
        );
        let second = SearchRepoManifestRecord::new(
            SearchCorpusKind::RepoContentChunk,
            "alpha/repo",
            "repo_content_chunk_repo_alpha",
            1,
            Some("rev-2"),
            3,
            42,
            1,
            "2026-03-23T12:00:00Z",
        );

        assert_ne!(first.publication_id, second.publication_id);
        assert_ne!(first.cache_version(), second.cache_version());
    }
}
