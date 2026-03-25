use crate::analyzers::RepositoryAnalysisOutput;
use crate::gateway::studio::repo_index::RepoCodeDocument;
use crate::search_plane::repo_entity::build::plan_repo_entity_build;
use crate::search_plane::repo_entity::schema::{
    hit_json_column, projected_columns, rows_from_analysis,
};
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

use crate::search_plane::repo_entity::build::RepoEntityBuildAction;
use crate::search_plane::repo_entity::build::write::{write_mutated_table, write_replaced_table};

use std::collections::BTreeMap;
use xiuxian_vector::VectorStoreError;

pub(crate) async fn publish_repo_entities(
    service: &SearchPlaneService,
    repo_id: &str,
    analysis: &RepositoryAnalysisOutput,
    documents: &[RepoCodeDocument],
    source_revision: Option<&str>,
) -> Result<(), VectorStoreError> {
    let previous_fingerprints = service
        .repo_corpus_file_fingerprints(SearchCorpusKind::RepoEntity, repo_id)
        .await;
    let current_record = service
        .repo_corpus_record_for_reads(SearchCorpusKind::RepoEntity, repo_id)
        .await;
    let rows = rows_from_analysis(repo_id, analysis)?;
    let plan = plan_repo_entity_build(
        repo_id,
        &rows,
        documents,
        source_revision,
        current_record
            .as_ref()
            .and_then(|record| record.publication.as_ref()),
        previous_fingerprints,
    );

    match &plan.action {
        RepoEntityBuildAction::Noop => {
            service
                .set_repo_corpus_file_fingerprints(
                    SearchCorpusKind::RepoEntity,
                    repo_id,
                    &plan.file_fingerprints,
                )
                .await;
            Ok(())
        }
        RepoEntityBuildAction::RefreshPublication { table_name } => {
            let store = service.open_store(SearchCorpusKind::RepoEntity).await?;
            let table_info = store.get_table_info(table_name.as_str()).await?;
            service
                .record_repo_publication(
                    SearchCorpusKind::RepoEntity,
                    repo_id,
                    table_name.as_str(),
                    source_revision,
                    &table_info,
                )
                .await;
            service
                .set_repo_corpus_file_fingerprints(
                    SearchCorpusKind::RepoEntity,
                    repo_id,
                    &plan.file_fingerprints,
                )
                .await;
            Ok(())
        }
        RepoEntityBuildAction::ReplaceAll {
            table_name,
            payload: rows,
        } => {
            write_replaced_table(service, table_name.as_str(), rows).await?;
            finalize_repo_entity_publication(
                service,
                repo_id,
                table_name.as_str(),
                source_revision,
                &plan.file_fingerprints,
            )
            .await
        }
        RepoEntityBuildAction::CloneAndMutate {
            base_table_name,
            target_table_name,
            replaced_paths,
            changed_payload: changed_rows,
        } => {
            write_mutated_table(
                service,
                base_table_name.as_str(),
                target_table_name.as_str(),
                replaced_paths,
                changed_rows,
            )
            .await?;
            finalize_repo_entity_publication(
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

async fn finalize_repo_entity_publication(
    service: &SearchPlaneService,
    repo_id: &str,
    table_name: &str,
    source_revision: Option<&str>,
    file_fingerprints: &BTreeMap<String, crate::search_plane::SearchFileFingerprint>,
) -> Result<(), VectorStoreError> {
    let mut prewarm_columns = projected_columns().to_vec();
    prewarm_columns.push(hit_json_column());
    service
        .prewarm_repo_table(
            SearchCorpusKind::RepoEntity,
            repo_id,
            table_name,
            &prewarm_columns,
        )
        .await?;
    let store = service.open_store(SearchCorpusKind::RepoEntity).await?;
    let table_info = store.get_table_info(table_name).await?;
    service
        .record_repo_publication(
            SearchCorpusKind::RepoEntity,
            repo_id,
            table_name,
            source_revision,
            &table_info,
        )
        .await;
    service
        .set_repo_corpus_file_fingerprints(SearchCorpusKind::RepoEntity, repo_id, file_fingerprints)
        .await;
    Ok(())
}
