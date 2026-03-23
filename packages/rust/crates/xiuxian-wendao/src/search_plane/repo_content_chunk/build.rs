use xiuxian_vector::{ScalarIndexType, VectorStoreError};

use crate::gateway::studio::repo_index::RepoCodeDocument;
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

use super::schema::{
    language_column, repo_content_chunk_batches, repo_content_chunk_schema, rows_from_documents,
    search_text_column,
};

pub(crate) async fn publish_repo_content_chunks(
    service: &SearchPlaneService,
    repo_id: &str,
    documents: &[RepoCodeDocument],
    source_revision: Option<&str>,
) -> Result<(), VectorStoreError> {
    let store = service
        .open_store(SearchCorpusKind::RepoContentChunk)
        .await?;
    let table_name = service.repo_content_chunk_table_name(repo_id);
    let rows = rows_from_documents(documents);
    store
        .replace_record_batches(
            table_name.as_str(),
            repo_content_chunk_schema(),
            repo_content_chunk_batches(&rows)?,
        )
        .await?;
    store
        .create_inverted_index(table_name.as_str(), search_text_column(), None)
        .await?;
    store
        .create_column_scalar_index(
            table_name.as_str(),
            language_column(),
            None,
            ScalarIndexType::Bitmap,
        )
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
    Ok(())
}
