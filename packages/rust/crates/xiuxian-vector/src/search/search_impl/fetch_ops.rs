use std::collections::BTreeMap;

use futures::TryStreamExt;
use lance::deps::arrow_array::{Array, FixedSizeListArray, Float32Array, StringArray};

use crate::{ID_COLUMN, VECTOR_COLUMN, VectorStore, VectorStoreError};

impl VectorStore {
    /// Fetch stored embedding vectors by document id from one table.
    ///
    /// Missing ids are skipped. Returned keys are unique and sorted because the
    /// output uses a [`BTreeMap`].
    ///
    /// # Errors
    ///
    /// Returns an error when the table cannot be opened, the Lance scanner
    /// cannot be executed, or the table does not expose the expected `id` and
    /// `vector` columns.
    pub async fn fetch_embeddings_by_ids(
        &self,
        table_name: &str,
        ids: &[String],
    ) -> Result<BTreeMap<String, Vec<f32>>, VectorStoreError> {
        if ids.is_empty() {
            return Ok(BTreeMap::new());
        }

        let table_path = self.table_path(table_name);
        if !table_path.exists() {
            return Err(VectorStoreError::TableNotFound(table_name.to_string()));
        }

        let dataset = self
            .open_dataset_at_uri(table_path.to_string_lossy().as_ref())
            .await?;
        let mut scanner = dataset.scan();
        scanner.project(&[ID_COLUMN, VECTOR_COLUMN])?;
        scanner.filter(build_id_in_filter(ids).as_str())?;

        let mut stream = scanner.try_into_stream().await?;
        let mut embeddings = BTreeMap::new();

        while let Some(batch) = stream.try_next().await? {
            let id_column = batch
                .column_by_name(ID_COLUMN)
                .and_then(|column| column.as_any().downcast_ref::<StringArray>())
                .ok_or_else(|| {
                    VectorStoreError::General(format!(
                        "missing Utf8 id column `{ID_COLUMN}` while fetching embeddings"
                    ))
                })?;
            let vector_column = batch
                .column_by_name(VECTOR_COLUMN)
                .and_then(|column| column.as_any().downcast_ref::<FixedSizeListArray>())
                .ok_or_else(|| {
                    VectorStoreError::General(format!(
                        "missing FixedSizeList vector column `{VECTOR_COLUMN}` while fetching embeddings"
                    ))
                })?;
            let vector_values = vector_column
                .values()
                .as_any()
                .downcast_ref::<Float32Array>()
                .ok_or_else(|| {
                    VectorStoreError::General(format!(
                        "vector column `{VECTOR_COLUMN}` does not store Float32 values"
                    ))
                })?;
            let vector_len = usize::try_from(vector_column.value_length()).unwrap_or(0);

            for row in 0..batch.num_rows() {
                if id_column.is_null(row) || vector_column.is_null(row) || vector_len == 0 {
                    continue;
                }

                let start = row.saturating_mul(vector_len);
                let end = start.saturating_add(vector_len);
                if end > vector_values.len() {
                    continue;
                }

                let vector = (start..end)
                    .map(|index| vector_values.value(index))
                    .collect::<Vec<_>>();
                embeddings.insert(id_column.value(row).to_string(), vector);
            }
        }

        Ok(embeddings)
    }
}

fn build_id_in_filter(ids: &[String]) -> String {
    let escaped = ids
        .iter()
        .map(|id| id.replace('\'', "''"))
        .collect::<Vec<_>>();
    format!("{ID_COLUMN} IN ('{}')", escaped.join("','"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fetch_embeddings_by_ids_returns_requested_vectors() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("embedding_fetch");
        let db_path_str = db_path.to_string_lossy();
        let mut store = VectorStore::new(db_path_str.as_ref(), Some(3))
            .await
            .expect("create vector store");

        store
            .replace_documents(
                "docs",
                vec!["doc-a".to_string(), "doc-b".to_string()],
                vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]],
                vec!["alpha".to_string(), "beta".to_string()],
                vec!["{}".to_string(), "{}".to_string()],
            )
            .await
            .expect("seed table");

        let embeddings = store
            .fetch_embeddings_by_ids("docs", &["doc-b".to_string(), "missing".to_string()])
            .await
            .expect("fetch embeddings");

        assert_eq!(embeddings.len(), 1);
        assert_eq!(embeddings.get("doc-b"), Some(&vec![4.0, 5.0, 6.0]));
    }

    #[tokio::test]
    async fn fetch_embeddings_by_ids_returns_empty_map_for_empty_request() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("embedding_fetch_empty");
        let db_path_str = db_path.to_string_lossy();
        let store = VectorStore::new(db_path_str.as_ref(), Some(3))
            .await
            .expect("create vector store");

        let embeddings = store
            .fetch_embeddings_by_ids("docs", &[])
            .await
            .expect("empty requests should not fail");

        assert!(embeddings.is_empty());
    }
}
