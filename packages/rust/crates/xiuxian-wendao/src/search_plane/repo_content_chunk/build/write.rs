use std::collections::BTreeSet;

use xiuxian_vector::{ScalarIndexType, VectorStore, VectorStoreError};

use crate::gateway::studio::repo_index::RepoCodeDocument;
use crate::search_plane::repo_content_chunk::schema::{
    language_column, path_column, repo_content_chunk_batches, repo_content_chunk_schema,
    rows_from_documents, search_text_column,
};
use crate::search_plane::{SearchCorpusKind, SearchPlaneService, delete_paths_from_table};

pub(crate) async fn write_replaced_table(
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

pub(crate) async fn write_mutated_table(
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
    store: &VectorStore,
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
