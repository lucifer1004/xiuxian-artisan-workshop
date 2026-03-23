//! Integration coverage for generic columnar table operations.

use std::sync::Arc;

use anyhow::Result;
use xiuxian_vector::{
    ColumnarScanOptions, LanceDataType, LanceField, LanceRecordBatch, LanceSchema,
    LanceStringArray, LanceUInt64Array, VectorStore,
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
