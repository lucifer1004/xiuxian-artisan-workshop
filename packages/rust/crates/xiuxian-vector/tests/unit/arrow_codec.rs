use super::{
    attach_record_batch_metadata, attach_record_batch_trace_id, decode_record_batches_ipc,
    encode_record_batches_ipc,
};
use anyhow::{Result, anyhow};
use arrow::array::{Array, Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use std::sync::Arc;

#[test]
fn encode_and_decode_record_batches_roundtrip() -> Result<()> {
    let batch = sample_batch()?;
    let payload = encode_record_batches_ipc(std::slice::from_ref(&batch))?;
    let decoded = decode_record_batches_ipc(payload.as_slice())?;

    assert_eq!(decoded.len(), 1);
    assert_string_column_eq(&batch, &decoded[0], "doc_id")?;
    assert_float_column_eq(&batch, &decoded[0], "score")?;

    Ok(())
}

#[test]
fn attach_record_batch_metadata_merges_existing_entries() -> Result<()> {
    let batch = attach_record_batch_metadata(
        &sample_batch()?,
        [("wendao.schema_version", "v1"), ("trace_id", "trace-123")],
    )?;
    let updated =
        attach_record_batch_metadata(&batch, [("trace_id", "trace-456"), ("request_id", "req-1")])?;

    assert_eq!(
        updated.schema().metadata().get("wendao.schema_version"),
        Some(&"v1".to_string())
    );
    assert_eq!(
        updated.schema().metadata().get("trace_id"),
        Some(&"trace-456".to_string())
    );
    assert_eq!(
        updated.schema().metadata().get("request_id"),
        Some(&"req-1".to_string())
    );

    Ok(())
}

#[test]
fn attach_record_batch_trace_id_sets_canonical_trace_metadata() -> Result<()> {
    let batch = attach_record_batch_trace_id(&sample_batch()?, "trace-123")?;

    assert_eq!(
        batch.schema().metadata().get("trace_id"),
        Some(&"trace-123".to_string())
    );

    Ok(())
}

fn sample_batch() -> Result<RecordBatch> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("doc_id", DataType::Utf8, false),
        Field::new("score", DataType::Float64, false),
    ]));
    Ok(RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(vec!["doc-1", "doc-2"])),
            Arc::new(Float64Array::from(vec![0.9, 0.4])),
        ],
    )?)
}

fn assert_string_column_eq(
    expected: &RecordBatch,
    actual: &RecordBatch,
    column: &str,
) -> Result<()> {
    let expected = expected
        .column_by_name(column)
        .and_then(|array| array.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| anyhow!("expected Utf8 column '{column}'"))?;
    let actual = actual
        .column_by_name(column)
        .and_then(|array| array.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| anyhow!("actual Utf8 column '{column}'"))?;

    assert_eq!(expected.len(), actual.len());
    for row in 0..expected.len() {
        assert_eq!(expected.value(row), actual.value(row));
    }

    Ok(())
}

fn assert_float_column_eq(
    expected: &RecordBatch,
    actual: &RecordBatch,
    column: &str,
) -> Result<()> {
    let expected = expected
        .column_by_name(column)
        .and_then(|array| array.as_any().downcast_ref::<Float64Array>())
        .ok_or_else(|| anyhow!("expected Float64 column '{column}'"))?;
    let actual = actual
        .column_by_name(column)
        .and_then(|array| array.as_any().downcast_ref::<Float64Array>())
        .ok_or_else(|| anyhow!("actual Float64 column '{column}'"))?;

    assert_eq!(expected.len(), actual.len());
    for row in 0..expected.len() {
        assert_eq!(expected.value(row), actual.value(row));
    }

    Ok(())
}
