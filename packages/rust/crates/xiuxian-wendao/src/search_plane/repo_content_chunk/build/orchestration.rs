use std::collections::BTreeMap;

use xiuxian_vector::VectorStoreError;

use crate::gateway::studio::repo_index::RepoCodeDocument;
use crate::search_plane::repo_content_chunk::build::plan::plan_repo_content_chunk_build;
use crate::search_plane::repo_content_chunk::build::types::RepoContentChunkBuildAction;
use crate::search_plane::repo_content_chunk::build::write::{
    write_mutated_table, write_replaced_table,
};
use crate::search_plane::repo_content_chunk::schema::projected_columns;
use crate::search_plane::{SearchCorpusKind, SearchFileFingerprint, SearchPlaneService};

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
