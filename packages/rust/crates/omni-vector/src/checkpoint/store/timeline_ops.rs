use super::{
    Array, CHECKPOINT_PARENT_ID_COLUMN, CHECKPOINT_STEP_COLUMN, CHECKPOINT_TIMESTAMP_COLUMN,
    CONTENT_COLUMN, CheckpointStore, ID_COLUMN, METADATA_COLUMN, PREVIEW_MAX_LEN, RecordBatch,
    Result, THREAD_ID_COLUMN, TryStreamExt, VectorStoreError,
};

impl CheckpointStore {
    /// Get timeline records for time-travel visualization.
    ///
    /// Returns structured timeline events with previews, suitable for UI display.
    ///
    /// # Errors
    ///
    /// Returns an error if the checkpoint table cannot be opened or scanned.
    pub async fn get_timeline_records(
        &mut self,
        table_name: &str,
        thread_id: &str,
        limit: usize,
    ) -> Result<Vec<crate::checkpoint::TimelineRecord>, VectorStoreError> {
        if limit == 0 {
            return Ok(Vec::new());
        }

        let table_path = self.table_path(table_name);
        if !table_path.exists() {
            return Ok(Vec::new());
        }

        let dataset = self.open_or_recover(table_name, false).await?;

        let mut scanner = dataset.scan();
        scanner.project(&[
            ID_COLUMN,
            CONTENT_COLUMN,
            METADATA_COLUMN,
            CHECKPOINT_TIMESTAMP_COLUMN,
            CHECKPOINT_PARENT_ID_COLUMN,
            CHECKPOINT_STEP_COLUMN,
        ])?;
        let filter_expr = format!("{} = '{}'", THREAD_ID_COLUMN, thread_id.replace('\'', "''"));
        scanner.filter(&filter_expr)?;

        let mut stream = scanner
            .try_into_stream()
            .await
            .map_err(VectorStoreError::LanceDB)?;
        let mut checkpoints: Vec<(f64, Option<i32>, crate::checkpoint::TimelineRecord)> =
            Vec::new();

        while let Some(batch) = stream.try_next().await.map_err(VectorStoreError::LanceDB)? {
            Self::extend_timeline_from_batch(&batch, thread_id, &mut checkpoints);
        }

        Self::sort_and_limit_timeline(&mut checkpoints, limit);
        Ok(Self::finalize_timeline_steps(checkpoints))
    }

    fn extend_timeline_from_batch(
        batch: &RecordBatch,
        thread_id: &str,
        checkpoints: &mut Vec<(f64, Option<i32>, crate::checkpoint::TimelineRecord)>,
    ) {
        let Some(id_col) = batch.column_by_name(ID_COLUMN) else {
            return;
        };
        let Some(content_col) = batch.column_by_name(CONTENT_COLUMN) else {
            return;
        };
        let Some(metadata_col) = batch.column_by_name(METADATA_COLUMN) else {
            return;
        };
        let Some(ts_col) = batch.column_by_name(CHECKPOINT_TIMESTAMP_COLUMN) else {
            return;
        };
        let Some(parent_col) = batch.column_by_name(CHECKPOINT_PARENT_ID_COLUMN) else {
            return;
        };
        let Some(step_col) = batch.column_by_name(CHECKPOINT_STEP_COLUMN) else {
            return;
        };

        let Some(id_strs) = id_col
            .as_any()
            .downcast_ref::<lance::deps::arrow_array::StringArray>()
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
        let Some(ts_vals) = ts_col
            .as_any()
            .downcast_ref::<lance::deps::arrow_array::Float64Array>()
        else {
            return;
        };
        let Some(parent_vals) = parent_col
            .as_any()
            .downcast_ref::<lance::deps::arrow_array::StringArray>()
        else {
            return;
        };
        let Some(step_vals) = step_col
            .as_any()
            .downcast_ref::<lance::deps::arrow_array::Int32Array>()
        else {
            return;
        };

        for i in 0..batch.num_rows() {
            if id_strs.is_null(i) || content_strs.is_null(i) || ts_vals.is_null(i) {
                continue;
            }

            let id = id_strs.value(i).to_string();
            let content = content_strs.value(i);
            let timestamp = ts_vals.value(i);
            let parent_checkpoint_id = if parent_vals.is_null(i) {
                None
            } else {
                Some(parent_vals.value(i).to_string())
            };
            let explicit_step = if step_vals.is_null(i) {
                None
            } else {
                Some(step_vals.value(i))
            };
            let reason = if metadata_strs.is_null(i) {
                None
            } else {
                Self::extract_reason(metadata_strs.value(i))
            };

            let record = crate::checkpoint::TimelineRecord {
                checkpoint_id: id,
                thread_id: thread_id.to_string(),
                step: explicit_step.unwrap_or(0),
                timestamp,
                preview: Self::build_preview(content),
                parent_checkpoint_id,
                reason,
            };
            checkpoints.push((timestamp, explicit_step, record));
        }
    }

    fn extract_reason(metadata: &str) -> Option<String> {
        serde_json::from_str::<serde_json::Value>(metadata)
            .ok()
            .and_then(|meta| {
                meta.get("reason")
                    .and_then(serde_json::Value::as_str)
                    .map(ToString::to_string)
            })
    }

    fn build_preview(content: &str) -> String {
        if content.chars().count() > PREVIEW_MAX_LEN {
            let truncated: String = content.chars().take(PREVIEW_MAX_LEN).collect();
            format!("{truncated}...")
        } else {
            content.to_string()
        }
    }

    fn sort_and_limit_timeline(
        checkpoints: &mut Vec<(f64, Option<i32>, crate::checkpoint::TimelineRecord)>,
        limit: usize,
    ) {
        checkpoints.sort_by(|left, right| right.0.total_cmp(&left.0));
        checkpoints.truncate(limit);
    }

    fn finalize_timeline_steps(
        checkpoints: Vec<(f64, Option<i32>, crate::checkpoint::TimelineRecord)>,
    ) -> Vec<crate::checkpoint::TimelineRecord> {
        checkpoints
            .into_iter()
            .enumerate()
            .map(|(index, (_, explicit_step, mut record))| {
                if explicit_step.is_none() {
                    record.step = if let Ok(step) = i32::try_from(index) {
                        step
                    } else {
                        log::warn!(
                            "timeline index {index} exceeds i32 range; clamping step to i32::MAX"
                        );
                        i32::MAX
                    };
                }
                record
            })
            .collect()
    }
}
