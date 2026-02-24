use super::{
    Array, CONTENT_COLUMN, CheckpointStore, METADATA_COLUMN, RecordBatch, Result, THREAD_ID_COLUMN,
    TryStreamExt, VECTOR_COLUMN, VectorStoreError,
};

impl CheckpointStore {
    /// Search for similar checkpoints using vector similarity.
    ///
    /// Performs ANN-like scan over checkpoint vectors and returns nearest rows.
    /// Optionally filters by thread ID and/or metadata key-value conditions.
    ///
    /// # Returns
    /// Vector of tuples: `(content_json, metadata_json, distance_score)`.
    ///
    /// # Errors
    ///
    /// Returns an error if the checkpoint table cannot be opened or scanned.
    pub async fn search(
        &mut self,
        table_name: &str,
        query_vector: &[f32],
        limit: usize,
        thread_id: Option<&str>,
        filter_metadata: Option<serde_json::Value>,
    ) -> Result<Vec<(String, String, f32)>, VectorStoreError> {
        if limit == 0 || query_vector.is_empty() {
            return Ok(Vec::new());
        }

        let table_path = self.table_path(table_name);
        if !table_path.exists() {
            return Ok(Vec::new());
        }

        let dataset = self.open_or_recover(table_name, false).await?;

        let mut scanner = dataset.scan();
        scanner.project(&[
            THREAD_ID_COLUMN,
            VECTOR_COLUMN,
            CONTENT_COLUMN,
            METADATA_COLUMN,
        ])?;
        if let Some(tid) = thread_id {
            let filter_expr = format!("{} = '{}'", THREAD_ID_COLUMN, tid.replace('\'', "''"));
            scanner.filter(&filter_expr)?;
        }

        let mut stream = scanner
            .try_into_stream()
            .await
            .map_err(VectorStoreError::LanceDB)?;
        let mut results: Vec<(String, String, f32)> = Vec::new();

        while let Some(batch) = stream.try_next().await.map_err(VectorStoreError::LanceDB)? {
            Self::extend_search_results_from_batch(
                &batch,
                query_vector,
                filter_metadata.as_ref(),
                &mut results,
            );
        }

        Self::sort_and_limit_results(&mut results, limit);
        Ok(results)
    }

    fn extend_search_results_from_batch(
        batch: &RecordBatch,
        query_vector: &[f32],
        filter_metadata: Option<&serde_json::Value>,
        results: &mut Vec<(String, String, f32)>,
    ) {
        let Some(vector_col) = batch.column_by_name(VECTOR_COLUMN) else {
            return;
        };
        let Some(content_col) = batch.column_by_name(CONTENT_COLUMN) else {
            return;
        };
        let Some(metadata_col) = batch.column_by_name(METADATA_COLUMN) else {
            return;
        };

        let Some(vector_arr) = vector_col
            .as_any()
            .downcast_ref::<lance::deps::arrow_array::FixedSizeListArray>()
        else {
            return;
        };
        let Some(content_strs) = content_col
            .as_any()
            .downcast_ref::<lance::deps::arrow_array::StringArray>()
        else {
            return;
        };
        let Some(metadata_strs) = metadata_col
            .as_any()
            .downcast_ref::<lance::deps::arrow_array::StringArray>()
        else {
            return;
        };
        let Some(values_arr) = vector_arr
            .values()
            .as_any()
            .downcast_ref::<lance::deps::arrow_array::Float32Array>()
        else {
            return;
        };

        let row_dim = usize::try_from(vector_arr.value_length()).unwrap_or_default();
        if row_dim == 0 {
            return;
        }
        let compute_len = row_dim.min(query_vector.len());
        if compute_len == 0 {
            return;
        }
        let values_slice = values_arr.values();

        for i in 0..batch.num_rows() {
            if vector_arr.is_null(i) || content_strs.is_null(i) || metadata_strs.is_null(i) {
                continue;
            }

            let start = i.saturating_mul(row_dim);
            let end = start.saturating_add(row_dim);
            if end > values_slice.len() {
                continue;
            }

            let distance = Self::l2_distance(query_vector, &values_slice[start..end], compute_len);
            let metadata_str = metadata_strs.value(i);
            if !Self::metadata_matches_filter(metadata_str, filter_metadata) {
                continue;
            }

            results.push((
                content_strs.value(i).to_string(),
                metadata_str.to_string(),
                distance,
            ));
        }
    }

    fn l2_distance(query_vector: &[f32], row_vector: &[f32], compute_len: usize) -> f32 {
        query_vector
            .iter()
            .take(compute_len)
            .zip(row_vector.iter().take(compute_len))
            .map(|(query, row)| {
                let diff = query - row;
                diff * diff
            })
            .sum::<f32>()
            .sqrt()
    }

    fn metadata_matches_filter(
        metadata_str: &str,
        filter_metadata: Option<&serde_json::Value>,
    ) -> bool {
        let Some(meta_filter) = filter_metadata else {
            return true;
        };
        let Ok(metadata_json) = serde_json::from_str::<serde_json::Value>(metadata_str) else {
            return false;
        };
        let Some(filter_obj) = meta_filter.as_object() else {
            return false;
        };
        filter_obj
            .iter()
            .all(|(key, expected)| metadata_json.get(key) == Some(expected))
    }

    fn sort_and_limit_results(results: &mut Vec<(String, String, f32)>, limit: usize) {
        results.sort_by(|left, right| left.2.total_cmp(&right.2));
        results.truncate(limit);
    }
}
