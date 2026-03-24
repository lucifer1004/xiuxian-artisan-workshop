use xiuxian_vector::{ScalarIndexType, VectorStoreError};

use crate::analyzers::RepositoryAnalysisOutput;
use crate::search_plane::{SearchCorpusKind, SearchPlaneService};

use super::schema::{
    entity_kind_column, language_column, repo_entity_batches, repo_entity_schema,
    rows_from_analysis, search_text_column, symbol_kind_column,
};

pub(crate) async fn publish_repo_entities(
    service: &SearchPlaneService,
    repo_id: &str,
    analysis: &RepositoryAnalysisOutput,
    source_revision: Option<&str>,
) -> Result<(), VectorStoreError> {
    let store = service.open_store(SearchCorpusKind::RepoEntity).await?;
    let table_name = SearchPlaneService::repo_entity_table_name(repo_id);
    let rows = rows_from_analysis(repo_id, analysis)?;
    store
        .replace_record_batches(
            table_name.as_str(),
            repo_entity_schema(),
            repo_entity_batches(&rows)?,
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
    store
        .create_column_scalar_index(
            table_name.as_str(),
            entity_kind_column(),
            None,
            ScalarIndexType::Bitmap,
        )
        .await?;
    store
        .create_column_scalar_index(
            table_name.as_str(),
            symbol_kind_column(),
            None,
            ScalarIndexType::Bitmap,
        )
        .await?;
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
    Ok(())
}
