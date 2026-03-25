use serde::{Deserialize, Serialize};

use super::{SearchCorpusKind, SearchCorpusStatus, SearchMaintenanceStatus};
use crate::gateway::studio::repo_index::{RepoIndexEntryStatus, RepoIndexPhase};

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

    /// Key that stores file-level fingerprints for one corpus.
    #[must_use]
    pub fn corpus_file_fingerprints_key(&self, corpus: SearchCorpusKind) -> String {
        format!("{}:file-fingerprints:{corpus}", self.namespace)
    }

    /// Key that stores repo-scoped file-level fingerprints for one repo-backed corpus.
    #[must_use]
    pub fn repo_corpus_file_fingerprints_key(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> String {
        format!(
            "{}:repo-file-fingerprints:{corpus}:repo:{}",
            self.namespace,
            blake3::hash(repo_id.as_bytes()).to_hex()
        )
    }

    /// Key that stores the combined repo-backed corpus record for one repo/corpus pair.
    #[must_use]
    pub fn repo_corpus_record_key(&self, corpus: SearchCorpusKind, repo_id: &str) -> String {
        format!(
            "{}:repo-corpus:{corpus}:repo:{}",
            self.namespace,
            blake3::hash(repo_id.as_bytes()).to_hex()
        )
    }

    /// Key that stores the latest full combined repo-backed corpus snapshot.
    #[must_use]
    pub fn repo_corpus_snapshot_key(&self) -> String {
        format!("{}:repo-corpus:snapshot", self.namespace)
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

/// Materialized publication row for one published repo-backed corpus table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchRepoPublicationRecord {
    /// Repo-backed corpus this manifest row belongs to.
    pub corpus: SearchCorpusKind,
    /// Stable repository identifier.
    pub repo_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Stable epoch token for the currently readable repo-backed publication.
    pub active_epoch: Option<u64>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SearchRepoPublicationInput {
    pub(crate) table_name: String,
    pub(crate) schema_version: u32,
    pub(crate) source_revision: Option<String>,
    pub(crate) table_version_id: u64,
    pub(crate) row_count: u64,
    pub(crate) fragment_count: u64,
    pub(crate) published_at: String,
}

impl SearchRepoPublicationRecord {
    /// Construct repo publication metadata from one published table snapshot.
    #[must_use]
    pub(crate) fn new(
        corpus: SearchCorpusKind,
        repo_id: impl Into<String>,
        input: SearchRepoPublicationInput,
    ) -> Self {
        let repo_id = repo_id.into();
        let publication_id = build_repo_publication_id(corpus, repo_id.as_str(), &input);
        Self {
            corpus,
            active_epoch: Some(build_repo_publication_epoch(publication_id.as_str())),
            publication_id,
            repo_id,
            table_name: input.table_name,
            table_version_id: input.table_version_id,
            schema_version: input.schema_version,
            source_revision: input.source_revision,
            row_count: input.row_count,
            fragment_count: input.fragment_count,
            published_at: input.published_at,
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

    /// Stable epoch token for the readable repo-backed publication.
    #[must_use]
    pub fn active_epoch_value(&self) -> u64 {
        self.active_epoch
            .unwrap_or_else(|| build_repo_publication_epoch(self.publication_id.as_str()))
    }
}

/// Materialized runtime row for one repository's indexing/search readiness state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchRepoRuntimeRecord {
    /// Stable repository identifier.
    pub repo_id: String,
    /// Latest repo indexing phase observed by the producer-side coordinator.
    pub phase: RepoIndexPhase,
    /// Latest source revision observed by repo indexing, when known.
    pub last_revision: Option<String>,
    /// Latest repo indexing error surfaced by the producer, when known.
    pub last_error: Option<String>,
    /// RFC3339 timestamp associated with the latest runtime snapshot, when known.
    pub updated_at: Option<String>,
}

impl SearchRepoRuntimeRecord {
    /// Project a persisted runtime row from one repo-index status entry.
    #[must_use]
    pub fn from_status(status: &RepoIndexEntryStatus) -> Self {
        Self {
            repo_id: status.repo_id.clone(),
            phase: status.phase,
            last_revision: status.last_revision.clone(),
            last_error: status.last_error.clone(),
            updated_at: status.updated_at.clone(),
        }
    }
}

/// Combined repo-backed corpus record that folds runtime and publication into one row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchRepoCorpusRecord {
    /// Repo-backed corpus this row belongs to.
    pub corpus: SearchCorpusKind,
    /// Stable repository identifier.
    pub repo_id: String,
    /// Latest repo runtime state known to the search plane, when available.
    pub runtime: Option<SearchRepoRuntimeRecord>,
    /// Latest readable publication for this repo-backed corpus, when available.
    pub publication: Option<SearchRepoPublicationRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Latest repo-backed maintenance metadata known to the search plane.
    pub maintenance: Option<SearchMaintenanceStatus>,
}

impl SearchRepoCorpusRecord {
    /// Construct one combined repo-backed corpus record.
    #[must_use]
    pub fn new(
        corpus: SearchCorpusKind,
        repo_id: impl Into<String>,
        runtime: Option<SearchRepoRuntimeRecord>,
        publication: Option<SearchRepoPublicationRecord>,
    ) -> Self {
        Self {
            corpus,
            repo_id: repo_id.into(),
            runtime,
            publication,
            maintenance: None,
        }
    }

    /// Attach the latest repo-backed maintenance metadata to this combined row.
    #[must_use]
    pub fn with_maintenance(mut self, maintenance: Option<SearchMaintenanceStatus>) -> Self {
        self.maintenance = maintenance;
        self
    }
}

/// Full combined repo-backed corpus snapshot owned by the search plane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchRepoCorpusSnapshotRecord {
    /// Combined repo-backed corpus rows across all tracked repos and corpora.
    pub records: Vec<SearchRepoCorpusRecord>,
}

fn build_repo_publication_id(
    corpus: SearchCorpusKind,
    repo_id: &str,
    input: &SearchRepoPublicationInput,
) -> String {
    let payload = format!(
        "{corpus}|{}|{}|{schema_version}|{}|{table_version_id}|{row_count}|{fragment_count}|{}",
        repo_id.trim().to_ascii_lowercase(),
        input.table_name.trim().to_ascii_lowercase(),
        input
            .source_revision
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_ascii_lowercase(),
        input.published_at.trim().to_ascii_lowercase(),
        schema_version = input.schema_version,
        table_version_id = input.table_version_id,
        row_count = input.row_count,
        fragment_count = input.fragment_count,
    );
    blake3::hash(payload.as_bytes()).to_hex().to_string()
}

fn build_repo_publication_epoch(publication_id: &str) -> u64 {
    let hash = blake3::hash(publication_id.trim().as_bytes());
    let mut bytes = [0_u8; 8];
    bytes.copy_from_slice(&hash.as_bytes()[..8]);
    u64::from_be_bytes(bytes)
}

/// Stable file-level fingerprint payload for incremental manifest updates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchFileFingerprint {
    /// Repo-relative path for the source file.
    pub relative_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Optional local partition identifier used to route incremental updates.
    pub partition_id: Option<String>,
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
    fn repo_publication_id_changes_when_table_version_changes() {
        let first = SearchRepoPublicationRecord::new(
            SearchCorpusKind::RepoEntity,
            "alpha/repo",
            SearchRepoPublicationInput {
                table_name: "repo_entity_repo_alpha".to_string(),
                schema_version: 1,
                source_revision: Some("rev-1".to_string()),
                table_version_id: 7,
                row_count: 10,
                fragment_count: 2,
                published_at: "2026-03-23T12:00:00Z".to_string(),
            },
        );
        let second = SearchRepoPublicationRecord::new(
            SearchCorpusKind::RepoEntity,
            "alpha/repo",
            SearchRepoPublicationInput {
                table_name: "repo_entity_repo_alpha".to_string(),
                schema_version: 1,
                source_revision: Some("rev-1".to_string()),
                table_version_id: 8,
                row_count: 10,
                fragment_count: 2,
                published_at: "2026-03-23T12:00:00Z".to_string(),
            },
        );

        assert_ne!(first.publication_id, second.publication_id);
        assert_ne!(first.active_epoch_value(), second.active_epoch_value());
        assert_ne!(first.cache_version(), second.cache_version());
    }

    #[test]
    fn repo_publication_cache_version_is_stable_for_same_publication() {
        let first = SearchRepoPublicationRecord::new(
            SearchCorpusKind::RepoContentChunk,
            "alpha/repo",
            SearchRepoPublicationInput {
                table_name: "repo_content_chunk_repo_alpha".to_string(),
                schema_version: 1,
                source_revision: Some("rev-7".to_string()),
                table_version_id: 3,
                row_count: 42,
                fragment_count: 1,
                published_at: "2026-03-23T12:00:00Z".to_string(),
            },
        );
        let second = SearchRepoPublicationRecord::new(
            SearchCorpusKind::RepoContentChunk,
            "alpha/repo",
            SearchRepoPublicationInput {
                table_name: "repo_content_chunk_repo_alpha".to_string(),
                schema_version: 1,
                source_revision: Some("rev-7".to_string()),
                table_version_id: 3,
                row_count: 42,
                fragment_count: 1,
                published_at: "2026-03-23T12:00:00Z".to_string(),
            },
        );

        assert_eq!(first.publication_id, second.publication_id);
        assert_eq!(first.active_epoch_value(), second.active_epoch_value());
        assert_eq!(first.cache_version(), second.cache_version());
    }

    #[test]
    fn repo_publication_id_changes_when_source_revision_changes() {
        let first = SearchRepoPublicationRecord::new(
            SearchCorpusKind::RepoContentChunk,
            "alpha/repo",
            SearchRepoPublicationInput {
                table_name: "repo_content_chunk_repo_alpha".to_string(),
                schema_version: 1,
                source_revision: Some("rev-1".to_string()),
                table_version_id: 3,
                row_count: 42,
                fragment_count: 1,
                published_at: "2026-03-23T12:00:00Z".to_string(),
            },
        );
        let second = SearchRepoPublicationRecord::new(
            SearchCorpusKind::RepoContentChunk,
            "alpha/repo",
            SearchRepoPublicationInput {
                table_name: "repo_content_chunk_repo_alpha".to_string(),
                schema_version: 1,
                source_revision: Some("rev-2".to_string()),
                table_version_id: 3,
                row_count: 42,
                fragment_count: 1,
                published_at: "2026-03-23T12:00:00Z".to_string(),
            },
        );

        assert_ne!(first.publication_id, second.publication_id);
        assert_ne!(first.active_epoch_value(), second.active_epoch_value());
        assert_ne!(first.cache_version(), second.cache_version());
    }

    #[test]
    fn repo_publication_active_epoch_falls_back_for_legacy_payloads() {
        let legacy = SearchRepoPublicationRecord {
            corpus: SearchCorpusKind::RepoEntity,
            repo_id: "alpha/repo".to_string(),
            active_epoch: None,
            publication_id: "legacy-publication".to_string(),
            table_name: "repo_entity_repo_alpha".to_string(),
            table_version_id: 7,
            schema_version: 1,
            source_revision: Some("rev-1".to_string()),
            row_count: 10,
            fragment_count: 2,
            published_at: "2026-03-23T12:00:00Z".to_string(),
        };

        assert_eq!(
            legacy.active_epoch_value(),
            build_repo_publication_epoch("legacy-publication")
        );
    }
}
