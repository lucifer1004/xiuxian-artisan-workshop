impl VectorStore {
    async fn open_table_or_err(&self, table_name: &str) -> Result<Dataset, VectorStoreError> {
        let table_path = self.table_path(table_name);
        if !table_path.exists() {
            return Err(VectorStoreError::TableNotFound(table_name.to_string()));
        }
        {
            let mut cache = self.datasets.write().await;
            if let Some(cached) = cache.get(table_name) {
                return Ok(cached);
            }
        }
        let uri = table_path.to_string_lossy();
        let dataset = self.open_dataset_at_uri(uri.as_ref()).await?;
        {
            let mut cache = self.datasets.write().await;
            if let Some(cached) = cache.get(table_name) {
                return Ok(cached);
            }
            cache.insert(table_name.to_string(), dataset.clone());
        }
        Ok(dataset)
    }

    fn ensure_non_reserved_column(column: &str) -> Result<(), VectorStoreError> {
        if Self::is_reserved_column(column) {
            return Err(VectorStoreError::General(format!(
                "Column '{column}' is reserved and cannot be altered or dropped"
            )));
        }
        Ok(())
    }

    fn is_reserved_column(column: &str) -> bool {
        matches!(
            column,
            ID_COLUMN
                | VECTOR_COLUMN
                | CONTENT_COLUMN
                | METADATA_COLUMN
                | THREAD_ID_COLUMN
                | SKILL_NAME_COLUMN
                | CATEGORY_COLUMN
                | crate::TOOL_NAME_COLUMN
                | crate::FILE_PATH_COLUMN
                | crate::ROUTING_KEYWORDS_COLUMN
                | crate::INTENTS_COLUMN
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::DatasetCacheConfig;

    #[tokio::test]
    async fn open_table_or_err_populates_dataset_cache_for_query_paths() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("query_cache");
        let db_path_str = db_path.to_string_lossy();
        let store = VectorStore::new_with_cache_options(
            db_path_str.as_ref(),
            Some(8),
            DatasetCacheConfig {
                max_cached_tables: Some(4),
            },
        )
        .await
        .expect("create vector store");
        let schema = store.create_schema();
        let empty = lance::deps::arrow_array::RecordBatch::new_empty(schema.clone());
        store
            .replace_record_batches("cached", schema, vec![empty])
            .await
            .expect("create cached table");
        {
            let mut cache = store.datasets.write().await;
            cache.remove("cached");
            assert!(!cache.contains_key("cached"));
        }

        store
            .open_table_or_err("cached")
            .await
            .expect("first open should succeed");
        {
            let cache = store.datasets.read().await;
            assert!(cache.contains_key("cached"));
            assert_eq!(cache.len(), 1);
        }

        store
            .open_table_or_err("cached")
            .await
            .expect("second open should reuse cache");
        {
            let cache = store.datasets.read().await;
            assert!(cache.contains_key("cached"));
            assert_eq!(cache.len(), 1);
        }
    }
}
