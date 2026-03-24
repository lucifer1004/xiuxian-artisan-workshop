use std::sync::Arc;

use futures::TryStreamExt;
use lance::dataset::{MergeInsertBuilder, WhenMatched, WhenNotMatched, WriteParams};
use lance::deps::arrow_array::{RecordBatch, RecordBatchIterator};
use lance::deps::arrow_schema::{ArrowError, Schema};
use lance_file::version::LanceFileVersion;
use lance_index::IndexType;
use lance_index::scalar::FullTextSearchQuery;
use lance_index::scalar::inverted::tokenizer::InvertedIndexParams;
use lance_index::scalar::{BuiltinIndexType, ScalarIndexParams};
use lance_index::traits::DatasetIndexExt;

use crate::{VectorStore, VectorStoreError};

use crate::ScalarIndexType;

/// Generic scanner options for columnar tables backed by Lance datasets.
#[derive(Debug, Clone, Default)]
pub struct ColumnarScanOptions {
    /// Optional SQL-like filter pushed down to Lance.
    pub where_filter: Option<String>,
    /// Optional projected columns. Empty means all columns.
    pub projected_columns: Vec<String>,
    /// Optional scanner batch size.
    pub batch_size: Option<usize>,
    /// Optional fragment read-ahead.
    pub fragment_readahead: Option<usize>,
    /// Optional batch read-ahead.
    pub batch_readahead: Option<usize>,
    /// Optional scan limit.
    pub limit: Option<usize>,
}

fn default_columnar_write_params() -> WriteParams {
    WriteParams {
        data_storage_version: Some(LanceFileVersion::V2_1),
        ..WriteParams::default()
    }
}

fn reader_from_batches(
    schema: Arc<Schema>,
    batches: Vec<RecordBatch>,
) -> RecordBatchIterator<std::vec::IntoIter<Result<RecordBatch, ArrowError>>> {
    let rows = if batches.is_empty() {
        vec![Ok(RecordBatch::new_empty(schema.clone()))]
    } else {
        batches.into_iter().map(Ok).collect()
    };
    RecordBatchIterator::new(rows.into_iter(), schema)
}

async fn append_batches(
    dataset: &mut lance::dataset::Dataset,
    schema: Arc<Schema>,
    batches: Vec<RecordBatch>,
) -> Result<(), VectorStoreError> {
    if batches.is_empty() {
        return Ok(());
    }
    dataset
        .append(
            reader_from_batches(schema, batches),
            Some(default_columnar_write_params()),
        )
        .await
        .map_err(VectorStoreError::LanceDB)
}

impl VectorStore {
    /// Replace a columnar table with the provided Arrow batches.
    ///
    /// # Errors
    ///
    /// Returns an error when dataset creation or batch append fails.
    pub async fn replace_record_batches(
        &self,
        table_name: &str,
        schema: Arc<Schema>,
        mut batches: Vec<RecordBatch>,
    ) -> Result<(), VectorStoreError> {
        let initial = batches
            .is_empty()
            .then(|| RecordBatch::new_empty(schema.clone()))
            .or_else(|| Some(batches.remove(0)));
        let Some(initial_batch) = initial else {
            return Err(VectorStoreError::General(
                "replace_record_batches requires an initial batch".to_string(),
            ));
        };

        let (mut dataset, _) = self
            .get_or_create_dataset(table_name, true, Some((schema.clone(), initial_batch)))
            .await?;
        append_batches(&mut dataset, schema, batches).await?;
        {
            let mut cache = self.datasets.write().await;
            cache.insert(table_name.to_string(), dataset);
        }
        Ok(())
    }

    /// Append Arrow batches to a columnar table, creating it when needed.
    ///
    /// # Errors
    ///
    /// Returns an error when dataset creation or append fails.
    pub async fn append_record_batches(
        &self,
        table_name: &str,
        schema: Arc<Schema>,
        mut batches: Vec<RecordBatch>,
    ) -> Result<(), VectorStoreError> {
        if batches.is_empty() {
            return Ok(());
        }
        let first = batches.remove(0);
        let (mut dataset, created) = self
            .get_or_create_dataset(table_name, false, Some((schema.clone(), first.clone())))
            .await?;
        if created {
            append_batches(&mut dataset, schema, batches).await?;
        } else {
            let mut all_batches = Vec::with_capacity(batches.len() + 1);
            all_batches.push(first);
            all_batches.extend(batches);
            append_batches(&mut dataset, schema, all_batches).await?;
        }
        {
            let mut cache = self.datasets.write().await;
            cache.insert(table_name.to_string(), dataset);
        }
        Ok(())
    }

    /// Merge-insert Arrow batches into a columnar table using one or more key columns.
    ///
    /// # Errors
    ///
    /// Returns an error when table creation, batch preparation, or merge-insert execution fails.
    pub async fn merge_insert_record_batches(
        &self,
        table_name: &str,
        schema: Arc<Schema>,
        batches: Vec<RecordBatch>,
        match_on: &[String],
    ) -> Result<(), VectorStoreError> {
        if batches.is_empty() {
            return Ok(());
        }
        if match_on.is_empty() {
            return Err(VectorStoreError::General(
                "merge_insert_record_batches requires at least one match key".to_string(),
            ));
        }

        let table_path = self.table_path(table_name);
        if !table_path.exists() {
            return self
                .replace_record_batches(table_name, schema, batches)
                .await;
        }

        let dataset = self
            .open_dataset_at_uri(table_path.to_string_lossy().as_ref())
            .await?;
        let mut builder = MergeInsertBuilder::try_new(Arc::new(dataset), match_on.to_vec())?;
        builder
            .when_matched(WhenMatched::UpdateAll)
            .when_not_matched(WhenNotMatched::InsertAll);
        let job = builder.try_build()?;
        let (updated_dataset, _) = job
            .execute_reader(reader_from_batches(schema, batches))
            .await?;
        {
            let mut cache = self.datasets.write().await;
            cache.insert(table_name.to_string(), updated_dataset.as_ref().clone());
        }
        Ok(())
    }

    /// Delete rows from a columnar table using a Lance filter predicate.
    ///
    /// # Errors
    ///
    /// Returns an error when the table cannot be opened or the delete operation fails.
    pub async fn delete_where(
        &self,
        table_name: &str,
        predicate: &str,
    ) -> Result<(), VectorStoreError> {
        let trimmed = predicate.trim();
        if trimmed.is_empty() {
            return Ok(());
        }
        let mut dataset = self.open_table_or_err(table_name).await?;
        dataset
            .delete(trimmed)
            .await
            .map_err(VectorStoreError::LanceDB)?;
        {
            let mut cache = self.datasets.write().await;
            cache.insert(table_name.to_string(), dataset);
        }
        Ok(())
    }

    /// Scan a columnar table and return Arrow batches.
    ///
    /// # Errors
    ///
    /// Returns an error when the table cannot be opened or the scan fails.
    pub async fn scan_record_batches_streaming<E, F>(
        &self,
        table_name: &str,
        options: ColumnarScanOptions,
        mut on_batch: F,
    ) -> Result<(), E>
    where
        E: From<VectorStoreError>,
        F: FnMut(RecordBatch) -> Result<(), E>,
    {
        let dataset = self.open_table_or_err(table_name).await.map_err(E::from)?;
        let mut scanner = dataset.scan();
        if !options.projected_columns.is_empty() {
            let columns = options
                .projected_columns
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            scanner
                .project(&columns)
                .map_err(VectorStoreError::from)
                .map_err(E::from)?;
        }
        if let Some(filter) = options
            .where_filter
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            scanner
                .filter(filter)
                .map_err(VectorStoreError::from)
                .map_err(E::from)?;
        }
        if let Some(batch_size) = options.batch_size {
            scanner.batch_size(batch_size);
        }
        if let Some(fragment_readahead) = options.fragment_readahead {
            scanner.fragment_readahead(fragment_readahead);
        }
        if let Some(batch_readahead) = options.batch_readahead {
            scanner.batch_readahead(batch_readahead);
        }
        if let Some(limit) = options.limit {
            scanner
                .limit(Some(i64::try_from(limit).unwrap_or(i64::MAX)), None)
                .map_err(VectorStoreError::from)
                .map_err(E::from)?;
        }

        let mut stream = scanner
            .try_into_stream()
            .await
            .map_err(VectorStoreError::from)
            .map_err(E::from)?;
        while let Some(batch) = stream
            .try_next()
            .await
            .map_err(VectorStoreError::from)
            .map_err(E::from)?
        {
            on_batch(batch)?;
        }
        Ok(())
    }

    /// Scan a columnar table and return Arrow batches.
    ///
    /// # Errors
    ///
    /// Returns an error when the table cannot be opened or the scan fails.
    pub async fn scan_record_batches(
        &self,
        table_name: &str,
        options: ColumnarScanOptions,
    ) -> Result<Vec<RecordBatch>, VectorStoreError> {
        let mut batches = Vec::new();
        self.scan_record_batches_streaming(
            table_name,
            options,
            |batch| -> Result<(), VectorStoreError> {
                batches.push(batch);
                Ok(())
            },
        )
        .await?;
        Ok(batches)
    }

    /// Run a native Lance full-text search and return projected Arrow batches.
    ///
    /// # Errors
    ///
    /// Returns an error when the table cannot be opened or the FTS scan fails.
    pub async fn search_fts_batches_streaming<E, F>(
        &self,
        table_name: &str,
        query: &str,
        options: ColumnarScanOptions,
        mut on_batch: F,
    ) -> Result<(), E>
    where
        E: From<VectorStoreError>,
        F: FnMut(RecordBatch) -> Result<(), E>,
    {
        if query.trim().is_empty() {
            return Ok(());
        }
        let dataset = self.open_table_or_err(table_name).await.map_err(E::from)?;
        let mut scanner = dataset.scan();
        if !options.projected_columns.is_empty() {
            let columns = options
                .projected_columns
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            scanner
                .project(&columns)
                .map_err(VectorStoreError::from)
                .map_err(E::from)?;
        }
        scanner
            .full_text_search(FullTextSearchQuery::new(query.trim().to_string()))
            .map_err(VectorStoreError::from)
            .map_err(E::from)?;
        if let Some(filter) = options
            .where_filter
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            scanner
                .filter(filter)
                .map_err(VectorStoreError::from)
                .map_err(E::from)?;
        }
        if let Some(batch_size) = options.batch_size {
            scanner.batch_size(batch_size);
        }
        if let Some(fragment_readahead) = options.fragment_readahead {
            scanner.fragment_readahead(fragment_readahead);
        }
        if let Some(batch_readahead) = options.batch_readahead {
            scanner.batch_readahead(batch_readahead);
        }
        if let Some(limit) = options.limit {
            scanner
                .limit(Some(i64::try_from(limit).unwrap_or(i64::MAX)), None)
                .map_err(VectorStoreError::from)
                .map_err(E::from)?;
        }

        let mut stream = scanner
            .try_into_stream()
            .await
            .map_err(VectorStoreError::from)
            .map_err(E::from)?;
        while let Some(batch) = stream
            .try_next()
            .await
            .map_err(VectorStoreError::from)
            .map_err(E::from)?
        {
            on_batch(batch)?;
        }
        Ok(())
    }

    /// Run a native Lance full-text search and return projected Arrow batches.
    ///
    /// # Errors
    ///
    /// Returns an error when the table cannot be opened or the FTS scan fails.
    pub async fn search_fts_batches(
        &self,
        table_name: &str,
        query: &str,
        options: ColumnarScanOptions,
    ) -> Result<Vec<RecordBatch>, VectorStoreError> {
        let mut batches = Vec::new();
        self.search_fts_batches_streaming(
            table_name,
            query,
            options,
            |batch| -> Result<(), VectorStoreError> {
                batches.push(batch);
                Ok(())
            },
        )
        .await?;
        Ok(batches)
    }

    /// Create a Lance inverted index over an arbitrary text column.
    ///
    /// # Errors
    ///
    /// Returns an error when the table cannot be opened or the index build fails.
    pub async fn create_inverted_index(
        &self,
        table_name: &str,
        column: &str,
        index_name: Option<&str>,
    ) -> Result<(), VectorStoreError> {
        if !self.table_path(table_name).exists() {
            return Ok(());
        }
        let mut dataset = self.open_table_or_err(table_name).await?;
        dataset
            .create_index(
                &[column],
                IndexType::Inverted,
                index_name.map(str::to_string),
                &InvertedIndexParams::default(),
                true,
            )
            .await
            .map_err(VectorStoreError::LanceDB)?;
        Ok(())
    }

    /// Create a scalar index over an arbitrary column.
    ///
    /// # Errors
    ///
    /// Returns an error when the table cannot be opened or the index build fails.
    pub async fn create_column_scalar_index(
        &self,
        table_name: &str,
        column: &str,
        index_name: Option<&str>,
        index_type: ScalarIndexType,
    ) -> Result<(), VectorStoreError> {
        if !self.table_path(table_name).exists() {
            return Ok(());
        }
        let mut dataset = self.open_table_or_err(table_name).await?;
        match index_type {
            ScalarIndexType::BTree => {
                dataset
                    .create_index(
                        &[column],
                        IndexType::BTree,
                        index_name.map(str::to_string),
                        &ScalarIndexParams::for_builtin(BuiltinIndexType::BTree),
                        true,
                    )
                    .await
            }
            ScalarIndexType::Bitmap => {
                dataset
                    .create_index(
                        &[column],
                        IndexType::Bitmap,
                        index_name.map(str::to_string),
                        &ScalarIndexParams::for_builtin(BuiltinIndexType::Bitmap),
                        true,
                    )
                    .await
            }
            ScalarIndexType::Inverted => {
                dataset
                    .create_index(
                        &[column],
                        IndexType::Inverted,
                        index_name.map(str::to_string),
                        &InvertedIndexParams::default(),
                        true,
                    )
                    .await
            }
        }
        .map_err(VectorStoreError::LanceDB)?;
        Ok(())
    }
}
