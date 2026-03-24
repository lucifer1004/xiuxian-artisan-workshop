use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use crate::gateway::studio::repo_index::{RepoIndexEntryStatus, RepoIndexPhase};
use crate::search_plane::{
    SearchCorpusKind, SearchManifestKeyspace, SearchPlaneCoordinator, SearchQueryTelemetry,
    SearchRepoCorpusRecord, SearchRepoRuntimeRecord,
};

/// Project-scoped entrypoint for the search-plane domain.
#[derive(Clone)]
pub struct SearchPlaneService {
    pub(super) project_root: PathBuf,
    pub(super) storage_root: PathBuf,
    pub(super) manifest_keyspace: SearchManifestKeyspace,
    pub(super) coordinator: Arc<SearchPlaneCoordinator>,
    pub(super) cache: crate::search_plane::cache::SearchPlaneCache,
    pub(crate) maintenance_tasks: Arc<Mutex<std::collections::BTreeSet<SearchCorpusKind>>>,
    pub(crate) query_telemetry:
        Arc<RwLock<std::collections::BTreeMap<SearchCorpusKind, SearchQueryTelemetry>>>,
    pub(super) repo_corpus_records:
        Arc<RwLock<std::collections::BTreeMap<(SearchCorpusKind, String), SearchRepoCorpusRecord>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepoRuntimeState {
    pub(crate) phase: RepoIndexPhase,
    pub(crate) last_revision: Option<String>,
    pub(crate) last_error: Option<String>,
    pub(crate) updated_at: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepoSearchAvailability {
    Searchable,
    Pending,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepoSearchPublicationState {
    pub(crate) entity_published: bool,
    pub(crate) content_published: bool,
    pub(crate) availability: RepoSearchAvailability,
}

pub(crate) struct RepoSearchQueryCacheKeyInput<'a> {
    pub(crate) scope: &'a str,
    pub(crate) corpora: &'a [SearchCorpusKind],
    pub(crate) repo_corpora: &'a [SearchCorpusKind],
    pub(crate) repo_ids: &'a [String],
    pub(crate) query: &'a str,
    pub(crate) limit: usize,
    pub(crate) intent: Option<&'a str>,
    pub(crate) repo_hint: Option<&'a str>,
}

impl RepoSearchPublicationState {
    #[must_use]
    pub(crate) fn is_searchable(self) -> bool {
        matches!(self.availability, RepoSearchAvailability::Searchable)
    }
}

impl RepoRuntimeState {
    pub(super) fn from_status(status: &RepoIndexEntryStatus) -> Self {
        Self {
            phase: status.phase,
            last_revision: status.last_revision.clone(),
            last_error: status.last_error.clone(),
            updated_at: status.updated_at.clone(),
        }
    }

    pub(super) fn from_record(record: &SearchRepoRuntimeRecord) -> Self {
        Self {
            phase: record.phase,
            last_revision: record.last_revision.clone(),
            last_error: record.last_error.clone(),
            updated_at: record.updated_at.clone(),
        }
    }

    pub(super) fn as_status(&self, repo_id: &str) -> RepoIndexEntryStatus {
        RepoIndexEntryStatus {
            repo_id: repo_id.to_string(),
            phase: self.phase,
            queue_position: None,
            last_error: self.last_error.clone(),
            last_revision: self.last_revision.clone(),
            updated_at: self.updated_at.clone(),
            attempt_count: 0,
        }
    }
}
