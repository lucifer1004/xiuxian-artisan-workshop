use xiuxian_vector::{ScalarIndexType, VectorStoreError};

use crate::search_plane::repo_entity::schema::{
    RepoEntityRow, entity_kind_column, language_column, path_column, repo_entity_batches,
    repo_entity_schema, search_text_column, symbol_kind_column,
};
use crate::search_plane::{SearchCorpusKind, SearchPlaneService, delete_paths_from_table};

pub(crate) async fn write_replaced_table(
    service: &SearchPlaneService,
    table_name: &str,
    rows: &[RepoEntityRow],
) -> Result<(), VectorStoreError> {
    let store = service.open_store(SearchCorpusKind::RepoEntity).await?;
    store
        .replace_record_batches(table_name, repo_entity_schema(), repo_entity_batches(rows)?)
        .await?;
    ensure_repo_entity_indexes(&store, table_name).await
}

pub(crate) async fn write_mutated_table(
    service: &SearchPlaneService,
    base_table_name: &str,
    target_table_name: &str,
    replaced_paths: &std::collections::BTreeSet<String>,
    changed_rows: &[RepoEntityRow],
) -> Result<(), VectorStoreError> {
    let store = service.open_store(SearchCorpusKind::RepoEntity).await?;
    store
        .clone_table(base_table_name, target_table_name, true)
        .await?;
    delete_paths_from_table(&store, target_table_name, path_column(), replaced_paths).await?;
    let changed_batches = repo_entity_batches(changed_rows)?;
    if !changed_batches.is_empty() {
        store
            .merge_insert_record_batches(
                target_table_name,
                repo_entity_schema(),
                changed_batches,
                &["id".to_string()],
            )
            .await?;
    }
    ensure_repo_entity_indexes(&store, target_table_name).await
}

pub(crate) async fn ensure_repo_entity_indexes(
    store: &xiuxian_vector::VectorStore,
    table_name: &str,
) -> Result<(), VectorStoreError> {
    store
        .create_inverted_index(table_name, search_text_column(), None)
        .await?;
    store
        .create_column_scalar_index(table_name, language_column(), None, ScalarIndexType::Bitmap)
        .await?;
    store
        .create_column_scalar_index(
            table_name,
            entity_kind_column(),
            None,
            ScalarIndexType::Bitmap,
        )
        .await?;
    store
        .create_column_scalar_index(
            table_name,
            symbol_kind_column(),
            None,
            ScalarIndexType::Bitmap,
        )
        .await?;
    Ok(())
}
