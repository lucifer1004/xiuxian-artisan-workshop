use std::collections::{BTreeMap, BTreeSet};

use xiuxian_vector::{ScalarIndexType, VectorStoreError};

use crate::gateway::studio::repo_index::RepoCodeDocument;
#[cfg(test)]
use crate::search_plane::repo_staging::versioned_repo_table_name;
use crate::search_plane::{
    RepoStagedMutationAction, RepoStagedMutationPlan, SearchCorpusKind, SearchFileFingerprint,
    SearchPlaneService, delete_paths_from_table, plan_repo_staged_mutation,
};

use super::schema::{
    language_column, path_column, projected_columns, repo_content_chunk_batches,
    repo_content_chunk_schema, rows_from_documents, search_text_column,
};

const REPO_CONTENT_CHUNK_EXTRACTOR_VERSION: u32 = 1;

type RepoContentChunkBuildAction = RepoStagedMutationAction<Vec<RepoCodeDocument>>;
type RepoContentChunkBuildPlan = RepoStagedMutationPlan<Vec<RepoCodeDocument>>;

pub(crate) async fn publish_repo_content_chunks(
    service: &SearchPlaneService,
    repo_id: &str,
    documents: &[RepoCodeDocument],
    source_revision: Option<&str>,
) -> Result<(), VectorStoreError> {
    let previous_fingerprints = service
        .repo_corpus_file_fingerprints(SearchCorpusKind::RepoContentChunk, repo_id)
        .await;
    let current_record = service
        .repo_corpus_record_for_reads(SearchCorpusKind::RepoContentChunk, repo_id)
        .await;
    let plan = plan_repo_content_chunk_build(
        repo_id,
        documents,
        source_revision,
        current_record
            .as_ref()
            .and_then(|record| record.publication.as_ref()),
        previous_fingerprints,
    );

    match &plan.action {
        RepoContentChunkBuildAction::Noop => {
            service
                .set_repo_corpus_file_fingerprints(
                    SearchCorpusKind::RepoContentChunk,
                    repo_id,
                    &plan.file_fingerprints,
                )
                .await;
            Ok(())
        }
        RepoContentChunkBuildAction::RefreshPublication { table_name } => {
            let store = service
                .open_store(SearchCorpusKind::RepoContentChunk)
                .await?;
            let table_info = store.get_table_info(table_name.as_str()).await?;
            service
                .record_repo_publication(
                    SearchCorpusKind::RepoContentChunk,
                    repo_id,
                    table_name.as_str(),
                    source_revision,
                    &table_info,
                )
                .await;
            service
                .set_repo_corpus_file_fingerprints(
                    SearchCorpusKind::RepoContentChunk,
                    repo_id,
                    &plan.file_fingerprints,
                )
                .await;
            Ok(())
        }
        RepoContentChunkBuildAction::ReplaceAll {
            table_name,
            payload: documents,
        } => {
            write_replaced_table(service, table_name.as_str(), documents).await?;
            finalize_repo_content_publication(
                service,
                repo_id,
                table_name.as_str(),
                source_revision,
                &plan.file_fingerprints,
            )
            .await
        }
        RepoContentChunkBuildAction::CloneAndMutate {
            base_table_name,
            target_table_name,
            replaced_paths,
            changed_payload: changed_documents,
        } => {
            write_mutated_table(
                service,
                base_table_name.as_str(),
                target_table_name.as_str(),
                replaced_paths,
                changed_documents,
            )
            .await?;
            finalize_repo_content_publication(
                service,
                repo_id,
                target_table_name.as_str(),
                source_revision,
                &plan.file_fingerprints,
            )
            .await
        }
    }
}

async fn finalize_repo_content_publication(
    service: &SearchPlaneService,
    repo_id: &str,
    table_name: &str,
    source_revision: Option<&str>,
    file_fingerprints: &BTreeMap<String, SearchFileFingerprint>,
) -> Result<(), VectorStoreError> {
    let prewarm_columns = projected_columns();
    service
        .prewarm_repo_table(
            SearchCorpusKind::RepoContentChunk,
            repo_id,
            table_name,
            &prewarm_columns,
        )
        .await?;
    let store = service
        .open_store(SearchCorpusKind::RepoContentChunk)
        .await?;
    let table_info = store.get_table_info(table_name).await?;
    service
        .record_repo_publication(
            SearchCorpusKind::RepoContentChunk,
            repo_id,
            table_name,
            source_revision,
            &table_info,
        )
        .await;
    service
        .set_repo_corpus_file_fingerprints(
            SearchCorpusKind::RepoContentChunk,
            repo_id,
            file_fingerprints,
        )
        .await;
    Ok(())
}

async fn write_replaced_table(
    service: &SearchPlaneService,
    table_name: &str,
    documents: &[RepoCodeDocument],
) -> Result<(), VectorStoreError> {
    let store = service
        .open_store(SearchCorpusKind::RepoContentChunk)
        .await?;
    let rows = rows_from_documents(documents);
    store
        .replace_record_batches(
            table_name,
            repo_content_chunk_schema(),
            repo_content_chunk_batches(&rows)?,
        )
        .await?;
    ensure_repo_content_indexes(&store, table_name).await
}

async fn write_mutated_table(
    service: &SearchPlaneService,
    base_table_name: &str,
    target_table_name: &str,
    replaced_paths: &BTreeSet<String>,
    changed_documents: &[RepoCodeDocument],
) -> Result<(), VectorStoreError> {
    let store = service
        .open_store(SearchCorpusKind::RepoContentChunk)
        .await?;
    let schema = repo_content_chunk_schema();
    let changed_rows = rows_from_documents(changed_documents);
    let changed_batches = repo_content_chunk_batches(&changed_rows)?;
    store
        .clone_table(base_table_name, target_table_name, true)
        .await?;
    delete_paths_from_table(&store, target_table_name, path_column(), replaced_paths).await?;
    if !changed_batches.is_empty() {
        store
            .merge_insert_record_batches(
                target_table_name,
                schema,
                changed_batches,
                &["id".to_string()],
            )
            .await?;
    }
    ensure_repo_content_indexes(&store, target_table_name).await
}

async fn ensure_repo_content_indexes(
    store: &xiuxian_vector::VectorStore,
    table_name: &str,
) -> Result<(), VectorStoreError> {
    store
        .create_inverted_index(table_name, search_text_column(), None)
        .await?;
    store
        .create_column_scalar_index(table_name, language_column(), None, ScalarIndexType::Bitmap)
        .await?;
    Ok(())
}

fn plan_repo_content_chunk_build(
    repo_id: &str,
    documents: &[RepoCodeDocument],
    source_revision: Option<&str>,
    previous_publication: Option<&crate::search_plane::SearchRepoPublicationRecord>,
    previous_fingerprints: BTreeMap<String, SearchFileFingerprint>,
) -> RepoContentChunkBuildPlan {
    let file_fingerprints = documents
        .iter()
        .map(|document| {
            (
                document.path.clone(),
                document.to_file_fingerprint(
                    REPO_CONTENT_CHUNK_EXTRACTOR_VERSION,
                    SearchCorpusKind::RepoContentChunk.schema_version(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let changed_documents = documents
        .iter()
        .filter(|document| {
            previous_fingerprints.get(document.path.as_str())
                != file_fingerprints.get(document.path.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    let changed_paths = changed_documents
        .iter()
        .map(|document| document.path.clone())
        .collect::<BTreeSet<_>>();
    let deleted_paths = previous_fingerprints
        .keys()
        .filter(|path| !file_fingerprints.contains_key(*path))
        .cloned()
        .collect::<BTreeSet<_>>();

    plan_repo_staged_mutation(
        repo_id,
        SearchPlaneService::repo_content_chunk_table_name(repo_id).as_str(),
        SearchCorpusKind::RepoContentChunk,
        REPO_CONTENT_CHUNK_EXTRACTOR_VERSION,
        source_revision,
        previous_publication,
        previous_fingerprints,
        file_fingerprints,
        documents.to_vec(),
        changed_documents,
        changed_paths,
        deleted_paths,
    )
}

#[cfg(test)]
fn versioned_repo_content_table_name(
    repo_id: &str,
    file_fingerprints: &BTreeMap<String, SearchFileFingerprint>,
    source_revision: Option<&str>,
) -> String {
    versioned_repo_table_name(
        SearchPlaneService::repo_content_chunk_table_name(repo_id).as_str(),
        repo_id,
        file_fingerprints,
        source_revision,
        SearchCorpusKind::RepoContentChunk,
        REPO_CONTENT_CHUNK_EXTRACTOR_VERSION,
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::PathBuf;
    use std::sync::Arc;

    use super::{
        REPO_CONTENT_CHUNK_EXTRACTOR_VERSION, RepoContentChunkBuildAction,
        plan_repo_content_chunk_build, publish_repo_content_chunks,
        versioned_repo_content_table_name,
    };
    use crate::gateway::studio::repo_index::RepoCodeDocument;
    use crate::search_plane::{
        SearchCorpusKind, SearchMaintenancePolicy, SearchManifestKeyspace, SearchPlaneService,
        SearchRepoPublicationInput, SearchRepoPublicationRecord,
    };

    fn repo_document(
        path: &str,
        contents: &str,
        size_bytes: u64,
        modified_unix_ms: u64,
    ) -> RepoCodeDocument {
        RepoCodeDocument {
            path: path.to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from(contents),
            size_bytes,
            modified_unix_ms,
        }
    }

    #[test]
    fn plan_repo_content_chunk_build_only_rewrites_changed_files() {
        let first_documents = vec![
            repo_document("src/lib.rs", "fn alpha() {}\n", 14, 10),
            repo_document("src/util.rs", "fn beta() {}\n", 13, 10),
        ];
        let first_plan = plan_repo_content_chunk_build(
            "alpha/repo",
            &first_documents,
            Some("rev-1"),
            None,
            BTreeMap::new(),
        );
        let previous_publication = match first_plan.action {
            RepoContentChunkBuildAction::ReplaceAll { ref table_name, .. } => {
                SearchRepoPublicationRecord::new(
                    SearchCorpusKind::RepoContentChunk,
                    "alpha/repo",
                    SearchRepoPublicationInput {
                        table_name: table_name.clone(),
                        schema_version: SearchCorpusKind::RepoContentChunk.schema_version(),
                        source_revision: Some("rev-1".to_string()),
                        table_version_id: 1,
                        row_count: 2,
                        fragment_count: 1,
                        published_at: "2026-03-24T12:00:00Z".to_string(),
                    },
                )
            }
            other => panic!("unexpected first build action: {other:?}"),
        };

        let second_documents = vec![
            repo_document("src/lib.rs", "fn gamma() {}\n", 14, 20),
            repo_document("src/util.rs", "fn beta() {}\n", 13, 10),
        ];
        let second_plan = plan_repo_content_chunk_build(
            "alpha/repo",
            &second_documents,
            Some("rev-2"),
            Some(&previous_publication),
            first_plan.file_fingerprints.clone(),
        );

        match second_plan.action {
            RepoContentChunkBuildAction::CloneAndMutate {
                base_table_name,
                target_table_name,
                replaced_paths,
                changed_payload: changed_documents,
            } => {
                assert_eq!(base_table_name, previous_publication.table_name);
                assert_ne!(target_table_name, previous_publication.table_name);
                assert_eq!(
                    replaced_paths.into_iter().collect::<Vec<_>>(),
                    vec!["src/lib.rs".to_string()]
                );
                assert_eq!(changed_documents.len(), 1);
                assert_eq!(changed_documents[0].path, "src/lib.rs");
            }
            other => panic!("unexpected second build action: {other:?}"),
        }
    }

    #[test]
    fn plan_repo_content_chunk_build_reuses_table_for_revision_only_refresh() {
        let documents = vec![repo_document("src/lib.rs", "fn alpha() {}\n", 14, 10)];
        let table_name = versioned_repo_content_table_name(
            "alpha/repo",
            &documents
                .iter()
                .map(|document| {
                    (
                        document.path.clone(),
                        document.to_file_fingerprint(
                            REPO_CONTENT_CHUNK_EXTRACTOR_VERSION,
                            SearchCorpusKind::RepoContentChunk.schema_version(),
                        ),
                    )
                })
                .collect::<BTreeMap<_, _>>(),
            Some("rev-1"),
        );
        let publication = SearchRepoPublicationRecord::new(
            SearchCorpusKind::RepoContentChunk,
            "alpha/repo",
            SearchRepoPublicationInput {
                table_name: table_name.clone(),
                schema_version: SearchCorpusKind::RepoContentChunk.schema_version(),
                source_revision: Some("rev-1".to_string()),
                table_version_id: 1,
                row_count: 1,
                fragment_count: 1,
                published_at: "2026-03-24T12:00:00Z".to_string(),
            },
        );
        let plan = plan_repo_content_chunk_build(
            "alpha/repo",
            &documents,
            Some("rev-2"),
            Some(&publication),
            documents
                .iter()
                .map(|document| {
                    (
                        document.path.clone(),
                        document.to_file_fingerprint(
                            REPO_CONTENT_CHUNK_EXTRACTOR_VERSION,
                            SearchCorpusKind::RepoContentChunk.schema_version(),
                        ),
                    )
                })
                .collect::<BTreeMap<_, _>>(),
        );

        match plan.action {
            RepoContentChunkBuildAction::RefreshPublication { table_name } => {
                assert_eq!(table_name, publication.table_name);
            }
            other => panic!("unexpected build action: {other:?}"),
        }
    }

    #[tokio::test]
    async fn repo_content_chunk_incremental_refresh_reuses_unchanged_rows() {
        let temp_dir = tempfile::tempdir().unwrap_or_else(|error| panic!("temp dir: {error}"));
        let service = SearchPlaneService::with_paths(
            PathBuf::from("/tmp/project"),
            temp_dir.path().join("search_plane"),
            SearchManifestKeyspace::new("xiuxian:test:repo-content-build"),
            SearchMaintenancePolicy::default(),
        );
        let first_documents = vec![
            repo_document("src/lib.rs", "fn alpha() {}\n", 14, 10),
            repo_document("src/util.rs", "fn beta() {}\n", 13, 10),
        ];
        publish_repo_content_chunks(&service, "alpha/repo", &first_documents, Some("rev-1"))
            .await
            .unwrap_or_else(|error| panic!("first publish: {error}"));

        let first_record = service
            .repo_corpus_record_for_reads(SearchCorpusKind::RepoContentChunk, "alpha/repo")
            .await
            .unwrap_or_else(|| panic!("first repo content record"));
        let first_table_name = first_record
            .publication
            .as_ref()
            .unwrap_or_else(|| panic!("first publication"))
            .table_name
            .clone();
        assert!(
            first_record
                .maintenance
                .as_ref()
                .and_then(|maintenance| maintenance.last_prewarmed_at.as_ref())
                .is_some()
        );

        let second_documents = vec![
            repo_document("src/lib.rs", "fn gamma() {}\n", 14, 20),
            repo_document("src/util.rs", "fn beta() {}\n", 13, 10),
        ];
        publish_repo_content_chunks(&service, "alpha/repo", &second_documents, Some("rev-2"))
            .await
            .unwrap_or_else(|error| panic!("second publish: {error}"));

        let second_record = service
            .repo_corpus_record_for_reads(SearchCorpusKind::RepoContentChunk, "alpha/repo")
            .await
            .unwrap_or_else(|| panic!("second repo content record"));
        let second_publication = second_record
            .publication
            .as_ref()
            .unwrap_or_else(|| panic!("second publication"));
        assert_ne!(second_publication.table_name, first_table_name);
        assert_eq!(second_publication.source_revision.as_deref(), Some("rev-2"));
        assert!(
            second_record
                .maintenance
                .as_ref()
                .and_then(|maintenance| maintenance.last_prewarmed_at.as_ref())
                .is_some()
        );

        let beta_hits = service
            .search_repo_content_chunks("alpha/repo", "beta", &Default::default(), 5)
            .await
            .unwrap_or_else(|error| panic!("query beta: {error}"));
        assert_eq!(beta_hits.len(), 1);
        assert_eq!(beta_hits[0].path, "src/util.rs");

        let gamma_hits = service
            .search_repo_content_chunks("alpha/repo", "gamma", &Default::default(), 5)
            .await
            .unwrap_or_else(|error| panic!("query gamma: {error}"));
        assert_eq!(gamma_hits.len(), 1);
        assert_eq!(gamma_hits[0].path, "src/lib.rs");

        let alpha_hits = service
            .search_repo_content_chunks("alpha/repo", "alpha", &Default::default(), 5)
            .await
            .unwrap_or_else(|error| panic!("query alpha: {error}"));
        assert!(alpha_hits.is_empty());

        let fingerprints = service
            .repo_corpus_file_fingerprints(SearchCorpusKind::RepoContentChunk, "alpha/repo")
            .await;
        assert_eq!(fingerprints.len(), 2);
        assert_eq!(
            fingerprints
                .get("src/lib.rs")
                .map(|fingerprint| fingerprint.modified_unix_ms),
            Some(20)
        );
    }
}
