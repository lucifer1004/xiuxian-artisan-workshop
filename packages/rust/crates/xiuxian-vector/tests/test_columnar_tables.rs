//! Integration coverage for generic columnar table operations.

use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Result;
use xiuxian_vector::{
    ColumnarScanOptions, LanceDataType, LanceField, LanceRecordBatch, LanceSchema,
    LanceStringArray, LanceUInt64Array, VectorStore, VectorStoreError,
};

fn local_symbol_schema() -> Arc<LanceSchema> {
    Arc::new(LanceSchema::new(vec![
        LanceField::new("id", LanceDataType::Utf8, false),
        LanceField::new("name", LanceDataType::Utf8, false),
        LanceField::new("line", LanceDataType::UInt64, false),
    ]))
}

fn local_symbol_batch(
    rows: &[(&str, &str, u64)],
    schema: Arc<LanceSchema>,
) -> Result<LanceRecordBatch> {
    let ids = rows.iter().map(|(id, _, _)| *id).collect::<Vec<_>>();
    let names = rows.iter().map(|(_, name, _)| *name).collect::<Vec<_>>();
    let lines = rows.iter().map(|(_, _, line)| *line).collect::<Vec<_>>();
    Ok(LanceRecordBatch::try_new(
        schema,
        vec![
            Arc::new(LanceStringArray::from(ids)),
            Arc::new(LanceStringArray::from(names)),
            Arc::new(LanceUInt64Array::from(lines)),
        ],
    )?)
}

#[tokio::test]
async fn test_columnar_table_replace_merge_scan_and_delete() -> Result<()> {
    let temp_dir = tempfile::Builder::new()
        .prefix("xiuxian_vector_columnar_")
        .tempdir()?;
    let store_root = temp_dir.path().join("columnar_store");
    let store = VectorStore::new(store_root.to_string_lossy().as_ref(), None).await?;
    let schema = local_symbol_schema();

    store
        .replace_record_batches(
            "local_symbol",
            schema.clone(),
            vec![local_symbol_batch(
                &[("sym-1", "AlphaSymbol", 10), ("sym-2", "BetaSymbol", 20)],
                schema.clone(),
            )?],
        )
        .await?;

    store
        .merge_insert_record_batches(
            "local_symbol",
            schema.clone(),
            vec![local_symbol_batch(
                &[
                    ("sym-2", "BetaSymbolRenamed", 21),
                    ("sym-3", "GammaSymbol", 30),
                ],
                schema.clone(),
            )?],
            &["id".to_string()],
        )
        .await?;

    let batches = store
        .scan_record_batches(
            "local_symbol",
            ColumnarScanOptions {
                projected_columns: vec!["id".to_string(), "name".to_string(), "line".to_string()],
                batch_size: Some(16),
                ..ColumnarScanOptions::default()
            },
        )
        .await?;

    assert_eq!(
        batches
            .iter()
            .map(LanceRecordBatch::num_rows)
            .sum::<usize>(),
        3
    );

    let mut rows = batches
        .iter()
        .flat_map(|batch| {
            let ids = batch
                .column(0)
                .as_any()
                .downcast_ref::<LanceStringArray>()
                .expect("id column should be utf8");
            let names = batch
                .column(1)
                .as_any()
                .downcast_ref::<LanceStringArray>()
                .expect("name column should be utf8");
            let lines = batch
                .column(2)
                .as_any()
                .downcast_ref::<LanceUInt64Array>()
                .expect("line column should be u64");
            (0..batch.num_rows()).map(move |row| {
                (
                    ids.value(row).to_string(),
                    names.value(row).to_string(),
                    lines.value(row),
                )
            })
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.0.cmp(&right.0));

    assert_eq!(
        rows,
        vec![
            ("sym-1".to_string(), "AlphaSymbol".to_string(), 10),
            ("sym-2".to_string(), "BetaSymbolRenamed".to_string(), 21),
            ("sym-3".to_string(), "GammaSymbol".to_string(), 30),
        ]
    );

    store.delete_where("local_symbol", "id = 'sym-1'").await?;

    let remaining = store
        .scan_record_batches(
            "local_symbol",
            ColumnarScanOptions {
                projected_columns: vec!["id".to_string()],
                ..ColumnarScanOptions::default()
            },
        )
        .await?;

    let ids = remaining
        .iter()
        .flat_map(|batch| {
            let ids = batch
                .column(0)
                .as_any()
                .downcast_ref::<LanceStringArray>()
                .expect("id column should be utf8");
            (0..batch.num_rows()).map(move |row| ids.value(row).to_string())
        })
        .collect::<Vec<_>>();

    assert_eq!(ids, vec!["sym-2".to_string(), "sym-3".to_string()]);

    Ok(())
}

#[tokio::test]
async fn test_columnar_table_streaming_scan_respects_limit() -> Result<()> {
    let temp_dir = tempfile::Builder::new()
        .prefix("xiuxian_vector_columnar_stream_")
        .tempdir()?;
    let store_root = temp_dir.path().join("columnar_store");
    let store = VectorStore::new(store_root.to_string_lossy().as_ref(), None).await?;
    let schema = local_symbol_schema();

    store
        .replace_record_batches(
            "local_symbol",
            schema.clone(),
            vec![local_symbol_batch(
                &[
                    ("sym-1", "AlphaSymbol", 10),
                    ("sym-2", "BetaSymbol", 20),
                    ("sym-3", "GammaSymbol", 30),
                ],
                schema.clone(),
            )?],
        )
        .await?;

    let mut batches = Vec::new();
    store
        .scan_record_batches_streaming(
            "local_symbol",
            ColumnarScanOptions {
                projected_columns: vec!["id".to_string(), "line".to_string()],
                batch_size: Some(1),
                limit: Some(2),
                ..ColumnarScanOptions::default()
            },
            |batch| -> Result<(), VectorStoreError> {
                batches.push(batch);
                Ok(())
            },
        )
        .await?;

    assert_eq!(
        batches
            .iter()
            .map(LanceRecordBatch::num_rows)
            .sum::<usize>(),
        2
    );
    assert!(batches.iter().all(|batch| batch.num_columns() == 2));

    Ok(())
}

#[tokio::test]
async fn test_columnar_table_streaming_async_scan_can_append_batches() -> Result<()> {
    let temp_dir = tempfile::Builder::new()
        .prefix("xiuxian_vector_columnar_stream_async_")
        .tempdir()?;
    let store_root = temp_dir.path().join("columnar_store");
    let source = VectorStore::new(store_root.to_string_lossy().as_ref(), None).await?;
    let target = source.clone();
    let schema = local_symbol_schema();

    source
        .replace_record_batches(
            "source_symbols",
            schema.clone(),
            vec![local_symbol_batch(
                &[
                    ("sym-1", "AlphaSymbol", 10),
                    ("sym-2", "BetaSymbol", 20),
                    ("sym-3", "GammaSymbol", 30),
                ],
                schema.clone(),
            )?],
        )
        .await?;

    target
        .replace_record_batches("target_symbols", schema.clone(), Vec::new())
        .await?;

    source
        .scan_record_batches_streaming_async(
            "source_symbols",
            ColumnarScanOptions {
                projected_columns: vec!["id".to_string(), "name".to_string(), "line".to_string()],
                batch_size: Some(1),
                limit: Some(2),
                ..ColumnarScanOptions::default()
            },
            |batch| {
                let target = target.clone();
                let schema = schema.clone();
                async move {
                    target
                        .append_record_batches("target_symbols", schema, vec![batch])
                        .await
                }
            },
        )
        .await?;

    let row_count = target.count("target_symbols").await?;
    assert_eq!(row_count, 2);

    Ok(())
}

#[tokio::test]
async fn test_columnar_table_multi_table_streaming_tracks_source_and_global_limit() -> Result<()> {
    let temp_dir = tempfile::Builder::new()
        .prefix("xiuxian_vector_columnar_multi_")
        .tempdir()?;
    let store_root = temp_dir.path().join("columnar_store");
    let store = VectorStore::new(store_root.to_string_lossy().as_ref(), None).await?;
    let schema = local_symbol_schema();

    store
        .replace_record_batches(
            "symbols_a",
            schema.clone(),
            vec![local_symbol_batch(
                &[("sym-1", "AlphaSymbol", 10), ("sym-2", "BetaSymbol", 20)],
                schema.clone(),
            )?],
        )
        .await?;
    store
        .replace_record_batches(
            "symbols_b",
            schema.clone(),
            vec![local_symbol_batch(
                &[("sym-3", "GammaSymbol", 30), ("sym-4", "DeltaSymbol", 40)],
                schema.clone(),
            )?],
        )
        .await?;

    let mut seen = Vec::new();
    store
        .scan_record_batches_streaming_across_tables(
            &["symbols_a", "symbols_b"],
            ColumnarScanOptions {
                projected_columns: vec!["id".to_string()],
                batch_size: Some(1),
                limit: Some(3),
                ..ColumnarScanOptions::default()
            },
            |table_name, batch| -> Result<(), VectorStoreError> {
                let ids = batch
                    .column(0)
                    .as_any()
                    .downcast_ref::<LanceStringArray>()
                    .expect("id column should be utf8");
                for row in 0..batch.num_rows() {
                    seen.push((table_name.to_string(), ids.value(row).to_string()));
                }
                Ok(())
            },
        )
        .await?;

    assert_eq!(
        seen,
        vec![
            ("symbols_a".to_string(), "sym-1".to_string()),
            ("symbols_a".to_string(), "sym-2".to_string()),
            ("symbols_b".to_string(), "sym-3".to_string()),
        ]
    );

    Ok(())
}

#[tokio::test]
async fn test_columnar_table_multi_table_async_scan_can_append_batches() -> Result<()> {
    let temp_dir = tempfile::Builder::new()
        .prefix("xiuxian_vector_columnar_multi_async_")
        .tempdir()?;
    let store_root = temp_dir.path().join("columnar_store");
    let source = VectorStore::new(store_root.to_string_lossy().as_ref(), None).await?;
    let target = source.clone();
    let schema = local_symbol_schema();

    source
        .replace_record_batches(
            "source_symbols_a",
            schema.clone(),
            vec![local_symbol_batch(
                &[("sym-1", "AlphaSymbol", 10), ("sym-2", "BetaSymbol", 20)],
                schema.clone(),
            )?],
        )
        .await?;
    source
        .replace_record_batches(
            "source_symbols_b",
            schema.clone(),
            vec![local_symbol_batch(
                &[("sym-3", "GammaSymbol", 30)],
                schema.clone(),
            )?],
        )
        .await?;

    target
        .replace_record_batches("target_symbols", schema.clone(), Vec::new())
        .await?;

    let seen_tables = Arc::new(Mutex::new(Vec::new()));
    source
        .scan_record_batches_streaming_across_tables_async(
            &["source_symbols_a", "source_symbols_b"],
            ColumnarScanOptions {
                projected_columns: vec!["id".to_string(), "name".to_string(), "line".to_string()],
                batch_size: Some(1),
                ..ColumnarScanOptions::default()
            },
            |table_name, batch| {
                let target = target.clone();
                let schema = schema.clone();
                let seen_tables = seen_tables.clone();
                let table_name = table_name.to_string();
                async move {
                    seen_tables
                        .lock()
                        .expect("seen_tables mutex should not be poisoned")
                        .push(table_name);
                    target
                        .append_record_batches("target_symbols", schema, vec![batch])
                        .await
                }
            },
        )
        .await?;

    let row_count = target.count("target_symbols").await?;
    assert_eq!(row_count, 3);
    assert_eq!(
        seen_tables
            .lock()
            .expect("seen_tables mutex should not be poisoned")
            .as_slice(),
        &[
            "source_symbols_a".to_string(),
            "source_symbols_a".to_string(),
            "source_symbols_b".to_string(),
        ]
    );

    Ok(())
}

#[tokio::test]
async fn test_columnar_table_clone_delete_and_merge_insert() -> Result<()> {
    let temp_dir = tempfile::Builder::new()
        .prefix("xiuxian_vector_columnar_clone_")
        .tempdir()?;
    let store_root = temp_dir.path().join("columnar_store");
    let store = VectorStore::new(store_root.to_string_lossy().as_ref(), None).await?;
    let schema = local_symbol_schema();

    store
        .replace_record_batches(
            "source_symbols",
            schema.clone(),
            vec![local_symbol_batch(
                &[
                    ("sym-1", "AlphaSymbol", 10),
                    ("sym-2", "BetaSymbol", 20),
                    ("sym-3", "GammaSymbol", 30),
                ],
                schema.clone(),
            )?],
        )
        .await?;

    store
        .clone_table("source_symbols", "staging_symbols", true)
        .await?;
    store
        .delete_where("staging_symbols", "id IN ('sym-1','sym-2')")
        .await?;
    store
        .merge_insert_record_batches(
            "staging_symbols",
            schema.clone(),
            vec![local_symbol_batch(
                &[
                    ("sym-2", "BetaSymbolRenamed", 21),
                    ("sym-4", "DeltaSymbol", 40),
                ],
                schema.clone(),
            )?],
            &["id".to_string()],
        )
        .await?;

    let source_batches = store
        .scan_record_batches(
            "source_symbols",
            ColumnarScanOptions {
                projected_columns: vec!["id".to_string()],
                ..ColumnarScanOptions::default()
            },
        )
        .await?;
    let source_ids = source_batches
        .iter()
        .flat_map(|batch| {
            let ids = batch
                .column(0)
                .as_any()
                .downcast_ref::<LanceStringArray>()
                .expect("id column should be utf8");
            (0..batch.num_rows()).map(move |row| ids.value(row).to_string())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        source_ids,
        vec![
            "sym-1".to_string(),
            "sym-2".to_string(),
            "sym-3".to_string()
        ]
    );

    let staging_batches = store
        .scan_record_batches(
            "staging_symbols",
            ColumnarScanOptions {
                projected_columns: vec!["id".to_string(), "name".to_string(), "line".to_string()],
                ..ColumnarScanOptions::default()
            },
        )
        .await?;
    let mut staging_rows = staging_batches
        .iter()
        .flat_map(|batch| {
            let ids = batch
                .column(0)
                .as_any()
                .downcast_ref::<LanceStringArray>()
                .expect("id column should be utf8");
            let names = batch
                .column(1)
                .as_any()
                .downcast_ref::<LanceStringArray>()
                .expect("name column should be utf8");
            let lines = batch
                .column(2)
                .as_any()
                .downcast_ref::<LanceUInt64Array>()
                .expect("line column should be u64");
            (0..batch.num_rows()).map(move |row| {
                (
                    ids.value(row).to_string(),
                    names.value(row).to_string(),
                    lines.value(row),
                )
            })
        })
        .collect::<Vec<_>>();
    staging_rows.sort_by(|left, right| left.0.cmp(&right.0));

    assert_eq!(
        staging_rows,
        vec![
            ("sym-2".to_string(), "BetaSymbolRenamed".to_string(), 21),
            ("sym-3".to_string(), "GammaSymbol".to_string(), 30),
            ("sym-4".to_string(), "DeltaSymbol".to_string(), 40),
        ]
    );

    Ok(())
}
