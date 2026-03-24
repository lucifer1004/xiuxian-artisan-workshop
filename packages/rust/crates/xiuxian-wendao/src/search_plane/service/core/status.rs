use super::types::{RepoRuntimeState, SearchPlaneService};
#[cfg(test)]
use crate::gateway::studio::repo_index::RepoIndexStatusResponse;
use crate::gateway::studio::repo_index::{RepoIndexEntryStatus, RepoIndexPhase};
use crate::search_plane::service::helpers::{
    annotate_status_reason, join_issue_messages, repo_content_phase, repo_corpus_active_epoch,
    repo_corpus_fingerprint_part, repo_corpus_staging_epoch, repo_index_failure_issue,
    repo_manifest_missing_issue, repo_publication_consistency_issue, summarize_issues,
    update_latest_timestamp,
};
use crate::search_plane::{
    SearchBuildLease, SearchCorpusIssue, SearchCorpusKind, SearchCorpusStatus,
    SearchPlaneStatusSnapshot, SearchRepoCorpusRecord, SearchRepoPublicationRecord,
    coordinator::SearchCompactionTask,
};

type PublishedRepoTable = (Option<RepoIndexEntryStatus>, SearchRepoPublicationRecord);

struct RepoTableStatusSynthesis {
    status: SearchCorpusStatus,
    published_repos: Vec<PublishedRepoTable>,
    issues: Vec<SearchCorpusIssue>,
    has_active_work: bool,
    runtime_statuses: Vec<RepoIndexEntryStatus>,
}

#[derive(Default)]
struct RepoTableSummary {
    has_ready_tables: bool,
    row_count: u64,
    fragment_count: u64,
    publication_epochs: Vec<u64>,
    fingerprint_parts: Vec<String>,
    fingerprint: String,
    build_finished_at: Option<String>,
    updated_at: Option<String>,
}

impl SearchPlaneService {
    /// Snapshot current multi-corpus status.
    #[must_use]
    pub fn status(&self) -> SearchPlaneStatusSnapshot {
        let mut snapshot = self.coordinator.status();
        self.annotate_runtime_status_snapshot(&mut snapshot);
        snapshot
    }

    pub(crate) async fn status_with_repo_runtime(&self) -> SearchPlaneStatusSnapshot {
        self.synchronize_repo_corpus_statuses_from_runtime().await;
        self.status()
    }

    #[cfg(test)]
    pub(crate) async fn status_with_repo_content(
        &self,
        repo_status: &RepoIndexStatusResponse,
    ) -> SearchPlaneStatusSnapshot {
        self.synchronize_repo_runtime(repo_status);
        self.status_with_repo_runtime().await
    }

    fn annotate_runtime_status_snapshot(&self, snapshot: &mut SearchPlaneStatusSnapshot) {
        for status in &mut snapshot.corpora {
            self.annotate_runtime_status(status);
        }
    }

    fn annotate_runtime_status(&self, status: &mut SearchCorpusStatus) {
        status.maintenance.compaction_running = self.is_compaction_running(status.corpus);
        status.last_query_telemetry = self.query_telemetry_for(status.corpus);
        annotate_status_reason(status);
    }

    fn is_compaction_running(&self, corpus: SearchCorpusKind) -> bool {
        self.maintenance_tasks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .contains(&corpus)
    }

    pub(crate) fn publish_ready_and_maintain(
        &self,
        lease: &SearchBuildLease,
        row_count: u64,
        fragment_count: u64,
    ) -> bool {
        if !self
            .coordinator
            .publish_ready(lease, row_count, fragment_count)
        {
            return false;
        }
        self.schedule_pending_compaction(lease.corpus);
        true
    }

    fn schedule_pending_compaction(&self, corpus: SearchCorpusKind) {
        let Some(task) = self.coordinator.pending_compaction_task(corpus) else {
            return;
        };
        {
            let mut guard = self
                .maintenance_tasks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if !guard.insert(corpus) {
                return;
            }
        }

        let service = self.clone();
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                service.run_compaction_task(task).await;
                service
                    .maintenance_tasks
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .remove(&corpus);
            });
        } else {
            self.maintenance_tasks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(&corpus);
        }
    }

    async fn run_compaction_task(&self, task: SearchCompactionTask) {
        let table_name = Self::table_name(task.corpus, task.active_epoch);
        let store = match self.open_store(task.corpus).await {
            Ok(store) => store,
            Err(error) => {
                log::warn!(
                    "search-plane compaction failed to open store for {} epoch {}: {}",
                    task.corpus,
                    task.active_epoch,
                    error
                );
                return;
            }
        };
        match store.compact(table_name.as_str()).await {
            Ok(stats) => {
                let fragment_count = u64::try_from(stats.fragments_after).unwrap_or(u64::MAX);
                let _ = self.coordinator.mark_compaction_complete(
                    task.corpus,
                    task.active_epoch,
                    task.row_count,
                    fragment_count,
                    task.reason,
                );
            }
            Err(error) => {
                log::warn!(
                    "search-plane compaction failed for {} epoch {} table {}: {}",
                    task.corpus,
                    task.active_epoch,
                    table_name,
                    error
                );
            }
        }
    }

    fn synthesize_repo_table_status(
        repo_records: &[SearchRepoCorpusRecord],
        corpus: SearchCorpusKind,
    ) -> SearchCorpusStatus {
        let synthesis = Self::collect_repo_table_status_synthesis(repo_records, corpus);
        if synthesis.published_repos.is_empty() {
            return Self::finish_empty_repo_table_status(corpus, synthesis);
        }
        Self::finish_published_repo_table_status(corpus, synthesis)
    }

    fn collect_repo_table_status_synthesis(
        repo_records: &[SearchRepoCorpusRecord],
        corpus: SearchCorpusKind,
    ) -> RepoTableStatusSynthesis {
        let mut synthesis = RepoTableStatusSynthesis {
            status: SearchCorpusStatus::new(corpus),
            published_repos: Vec::new(),
            issues: Vec::new(),
            has_active_work: false,
            runtime_statuses: Vec::new(),
        };
        for record in repo_records.iter().filter(|record| record.corpus == corpus) {
            Self::accumulate_repo_table_status_record(&mut synthesis, corpus, record);
        }
        synthesis
    }

    fn accumulate_repo_table_status_record(
        synthesis: &mut RepoTableStatusSynthesis,
        corpus: SearchCorpusKind,
        record: &SearchRepoCorpusRecord,
    ) {
        let runtime_status = Self::repo_runtime_status_for_record(record);
        if let Some(repo) = runtime_status.as_ref() {
            synthesis.runtime_statuses.push(repo.clone());
            update_latest_timestamp(&mut synthesis.status.updated_at, repo.updated_at.as_deref());
            Self::accumulate_repo_runtime_issues(synthesis, repo, record.publication.as_ref());
        }
        Self::accumulate_repo_publication_status(synthesis, corpus, record, runtime_status);
    }

    fn repo_runtime_status_for_record(
        record: &SearchRepoCorpusRecord,
    ) -> Option<RepoIndexEntryStatus> {
        record
            .runtime
            .as_ref()
            .map(|runtime| {
                RepoRuntimeState::from_record(runtime).as_status(record.repo_id.as_str())
            })
            .or_else(|| {
                record
                    .publication
                    .as_ref()
                    .map(|publication| RepoIndexEntryStatus {
                        repo_id: record.repo_id.clone(),
                        phase: RepoIndexPhase::Idle,
                        queue_position: None,
                        last_error: None,
                        last_revision: publication.source_revision.clone(),
                        updated_at: Some(publication.published_at.clone()),
                        attempt_count: 0,
                    })
            })
    }

    fn accumulate_repo_runtime_issues(
        synthesis: &mut RepoTableStatusSynthesis,
        repo: &RepoIndexEntryStatus,
        publication: Option<&SearchRepoPublicationRecord>,
    ) {
        match repo.phase {
            RepoIndexPhase::Queued
            | RepoIndexPhase::Checking
            | RepoIndexPhase::Syncing
            | RepoIndexPhase::Indexing => {
                synthesis.has_active_work = true;
            }
            RepoIndexPhase::Failed => {
                if let Some(issue) = repo_index_failure_issue(repo, publication) {
                    synthesis.issues.push(issue);
                }
            }
            RepoIndexPhase::Idle | RepoIndexPhase::Unsupported | RepoIndexPhase::Ready => {}
        }
    }

    fn accumulate_repo_publication_status(
        synthesis: &mut RepoTableStatusSynthesis,
        corpus: SearchCorpusKind,
        record: &SearchRepoCorpusRecord,
        runtime_status: Option<RepoIndexEntryStatus>,
    ) {
        if let Some(publication) = record.publication.as_ref() {
            if let Some(repo) = runtime_status.as_ref()
                && let Some(issue) = repo_publication_consistency_issue(corpus, repo, publication)
            {
                synthesis.issues.push(issue);
            }
            synthesis
                .published_repos
                .push((runtime_status, publication.clone()));
        } else if let Some(repo) = runtime_status.as_ref()
            && matches!(repo.phase, RepoIndexPhase::Ready)
        {
            synthesis
                .issues
                .push(repo_manifest_missing_issue(corpus, repo));
        }
    }

    fn finish_empty_repo_table_status(
        corpus: SearchCorpusKind,
        mut synthesis: RepoTableStatusSynthesis,
    ) -> SearchCorpusStatus {
        synthesis.status.phase = repo_content_phase(
            false,
            synthesis.has_active_work,
            !synthesis.issues.is_empty(),
        );
        synthesis.status.staging_epoch =
            repo_corpus_staging_epoch(corpus, &synthesis.runtime_statuses, None);
        Self::finalize_repo_table_status(&mut synthesis.status, synthesis.issues);
        synthesis.status
    }

    fn finish_published_repo_table_status(
        corpus: SearchCorpusKind,
        mut synthesis: RepoTableStatusSynthesis,
    ) -> SearchCorpusStatus {
        let summary = Self::published_repo_table_summary(synthesis.published_repos.as_slice());
        synthesis.status.phase = repo_content_phase(
            summary.has_ready_tables,
            synthesis.has_active_work,
            !synthesis.issues.is_empty(),
        );
        if summary.has_ready_tables {
            synthesis.status.active_epoch = Some(repo_corpus_active_epoch(
                corpus,
                summary.publication_epochs.as_slice(),
            ));
            synthesis.status.staging_epoch = repo_corpus_staging_epoch(
                corpus,
                &synthesis.runtime_statuses,
                synthesis.status.active_epoch,
            );
            synthesis.status.row_count = Some(summary.row_count);
            synthesis.status.fragment_count = Some(summary.fragment_count);
            synthesis.status.fingerprint = Some(
                blake3::hash(summary.fingerprint.as_bytes())
                    .to_hex()
                    .to_string(),
            );
        }
        update_latest_timestamp(
            &mut synthesis.status.build_finished_at,
            summary.build_finished_at.as_deref(),
        );
        update_latest_timestamp(
            &mut synthesis.status.updated_at,
            summary.updated_at.as_deref(),
        );
        Self::finalize_repo_table_status(&mut synthesis.status, synthesis.issues);
        synthesis.status
    }

    fn finalize_repo_table_status(status: &mut SearchCorpusStatus, issues: Vec<SearchCorpusIssue>) {
        status.last_error = join_issue_messages(&issues);
        status.issue_summary = summarize_issues(&issues);
        status.issues = issues;
        annotate_status_reason(status);
    }

    fn published_repo_table_summary(published_repos: &[PublishedRepoTable]) -> RepoTableSummary {
        let mut summary = RepoTableSummary::default();
        for (runtime_status, publication) in published_repos {
            summary.has_ready_tables = true;
            summary.row_count = summary.row_count.saturating_add(publication.row_count);
            summary.fragment_count = summary
                .fragment_count
                .saturating_add(publication.fragment_count);
            summary
                .publication_epochs
                .push(publication.active_epoch_value());
            update_latest_timestamp(
                &mut summary.build_finished_at,
                Some(publication.published_at.as_str()),
            );
            update_latest_timestamp(
                &mut summary.updated_at,
                Some(publication.published_at.as_str()),
            );
            if let Some(runtime_status) = runtime_status.as_ref() {
                summary
                    .fingerprint_parts
                    .push(repo_corpus_fingerprint_part(runtime_status, publication));
            }
        }
        summary.publication_epochs.sort_unstable();
        summary.fingerprint_parts.sort_unstable();
        summary.fingerprint = summary.fingerprint_parts.join("|");
        summary
    }

    pub(crate) async fn synchronize_repo_corpus_statuses_from_runtime(&self) {
        let repo_records = self
            .repo_corpus_snapshot_for_reads()
            .await
            .into_values()
            .collect::<Vec<_>>();
        for corpus in [
            SearchCorpusKind::RepoEntity,
            SearchCorpusKind::RepoContentChunk,
        ] {
            let mut status = Self::synthesize_repo_table_status(&repo_records, corpus);
            self.annotate_runtime_status(&mut status);
            self.coordinator.replace_status(status);
        }
    }
}
