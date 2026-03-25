use xiuxian_vector::{ScalarIndexType, VectorStoreError};

use crate::search_plane::attachment::build::{AttachmentBuildPlan, AttachmentWriteResult};
use crate::search_plane::attachment::schema::{
    attachment_batches, attachment_ext_column, attachment_schema, kind_column, search_text_column,
    source_path_column,
};
use crate::search_plane::{
    SearchBuildLease, SearchCorpusKind, SearchPlaneService, delete_paths_from_table,
};

pub(crate) async fn write_attachment_epoch(
    service: &SearchPlaneService,
    lease: &SearchBuildLease,
    plan: &AttachmentBuildPlan,
) -> Result<AttachmentWriteResult, VectorStoreError> {
    let store = service.open_store(SearchCorpusKind::Attachment).await?;
    let table_name = SearchPlaneService::table_name(SearchCorpusKind::Attachment, lease.epoch);
    let schema = attachment_schema();
    let changed_batches = attachment_batches(plan.changed_hits.as_slice())?;
    if let Some(base_epoch) = plan.base_epoch {
        let base_table_name =
            SearchPlaneService::table_name(SearchCorpusKind::Attachment, base_epoch);
        store
            .clone_table(base_table_name.as_str(), table_name.as_str(), true)
            .await?;
        delete_paths_from_table(
            &store,
            table_name.as_str(),
            source_path_column(),
            &plan.replaced_paths,
        )
        .await?;
        if !changed_batches.is_empty() {
            store
                .merge_insert_record_batches(
                    table_name.as_str(),
                    schema.clone(),
                    changed_batches,
                    &["id".to_string()],
                )
                .await?;
        }
    } else {
        store
            .replace_record_batches(table_name.as_str(), schema.clone(), changed_batches)
            .await?;
    }
    store
        .create_inverted_index(table_name.as_str(), search_text_column(), None)
        .await?;
    store
        .create_column_scalar_index(
            table_name.as_str(),
            attachment_ext_column(),
            None,
            ScalarIndexType::Bitmap,
        )
        .await?;
    store
        .create_column_scalar_index(
            table_name.as_str(),
            kind_column(),
            None,
            ScalarIndexType::Bitmap,
        )
        .await?;
    let table_info = store.get_table_info(table_name.as_str()).await?;
    Ok(AttachmentWriteResult {
        row_count: table_info.num_rows,
        fragment_count: u64::try_from(table_info.fragment_count).unwrap_or(u64::MAX),
    })
}
