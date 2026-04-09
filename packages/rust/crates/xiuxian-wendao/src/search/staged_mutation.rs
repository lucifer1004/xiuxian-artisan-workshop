use std::collections::BTreeSet;

use xiuxian_vector::{VectorStore, VectorStoreError};

const DELETE_PATH_FILTER_BATCH_SIZE: usize = 100;

pub(crate) async fn delete_paths_from_table(
    store: &VectorStore,
    table_name: &str,
    column: &str,
    paths: &BTreeSet<String>,
) -> Result<(), VectorStoreError> {
    for filter in path_delete_filters(column, paths) {
        store.delete_where(table_name, filter.as_str()).await?;
    }
    Ok(())
}

#[must_use]
pub(crate) fn path_delete_filters(column: &str, paths: &BTreeSet<String>) -> Vec<String> {
    if paths.is_empty() {
        return Vec::new();
    }

    let escaped = paths
        .iter()
        .map(|path| format!("'{}'", path.replace('\'', "''")))
        .collect::<Vec<_>>();
    escaped
        .chunks(DELETE_PATH_FILTER_BATCH_SIZE)
        .map(|chunk| format!("{column} IN ({})", chunk.join(",")))
        .collect()
}

#[cfg(test)]
#[path = "../../tests/unit/search/staged_mutation.rs"]
mod tests;
