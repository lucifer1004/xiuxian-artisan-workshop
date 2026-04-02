use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

use arrow::datatypes::Schema;
use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use arrow_ipc::reader::StreamReader;
use arrow_ipc::writer::StreamWriter;

const TRACE_ID_METADATA_KEY: &str = "trace_id";

/// Encode a single Arrow `RecordBatch` into IPC stream bytes.
///
/// # Errors
///
/// Returns [`ArrowError`] when Arrow IPC stream construction fails.
pub fn encode_record_batch_ipc(batch: &RecordBatch) -> Result<Vec<u8>, ArrowError> {
    encode_record_batches_ipc(std::slice::from_ref(batch))
}

/// Encode one or more Arrow `RecordBatch` values into IPC stream bytes.
///
/// # Errors
///
/// Returns [`ArrowError`] when the batch list is empty or Arrow IPC stream
/// construction fails.
pub fn encode_record_batches_ipc(batches: &[RecordBatch]) -> Result<Vec<u8>, ArrowError> {
    let Some(first_batch) = batches.first() else {
        return Err(ArrowError::InvalidArgumentError(
            "Arrow IPC encoding requires at least one RecordBatch".to_string(),
        ));
    };

    let mut buffer = Cursor::new(Vec::new());
    {
        let mut writer = StreamWriter::try_new(&mut buffer, first_batch.schema().as_ref())?;
        for batch in batches {
            writer.write(batch)?;
        }
        writer.finish()?;
    }
    Ok(buffer.into_inner())
}

/// Attach or overwrite schema metadata on a `RecordBatch`.
///
/// Existing schema metadata is preserved unless a provided key overwrites it.
///
/// # Errors
///
/// Returns [`ArrowError`] when the batch cannot be rebuilt with the merged
/// schema metadata.
pub fn attach_record_batch_metadata<K, V, I>(
    batch: &RecordBatch,
    metadata: I,
) -> Result<RecordBatch, ArrowError>
where
    K: Into<String>,
    V: Into<String>,
    I: IntoIterator<Item = (K, V)>,
{
    let mut merged: HashMap<String, String> = batch.schema().metadata().clone();
    merged.extend(
        metadata
            .into_iter()
            .map(|(key, value)| (key.into(), value.into())),
    );

    let schema = Arc::new(Schema::new_with_metadata(
        batch.schema().fields().clone(),
        merged,
    ));
    RecordBatch::try_new(schema, batch.columns().to_vec())
}

/// Attach or overwrite the canonical `trace_id` schema metadata entry.
///
/// # Errors
///
/// Returns [`ArrowError`] when the batch cannot be rebuilt with the updated
/// schema metadata.
pub fn attach_record_batch_trace_id(
    batch: &RecordBatch,
    trace_id: impl Into<String>,
) -> Result<RecordBatch, ArrowError> {
    attach_record_batch_metadata(batch, [(TRACE_ID_METADATA_KEY, trace_id.into())])
}

/// Decode Arrow IPC stream bytes into one or more `RecordBatch` values.
///
/// # Errors
///
/// Returns [`ArrowError`] when the payload is not valid Arrow IPC stream data.
pub fn decode_record_batches_ipc(payload: &[u8]) -> Result<Vec<RecordBatch>, ArrowError> {
    let cursor = Cursor::new(payload);
    let reader = StreamReader::try_new(cursor, None)?;
    reader.collect()
}

#[cfg(test)]
mod tests {
    use super::{
        attach_record_batch_metadata, attach_record_batch_trace_id, decode_record_batches_ipc,
        encode_record_batches_ipc,
    };
    use arrow::array::{Array, Float64Array, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};
    use arrow::record_batch::RecordBatch;
    use std::sync::Arc;

    #[test]
    fn encode_and_decode_record_batches_roundtrip() {
        let batch = sample_batch();
        let payload = encode_record_batches_ipc(&[batch.clone()]).expect("encode sample batch");
        let decoded = decode_record_batches_ipc(payload.as_slice()).expect("decode sample batch");

        assert_eq!(decoded.len(), 1);
        assert_string_column_eq(&batch, &decoded[0], "doc_id");
        assert_float_column_eq(&batch, &decoded[0], "score");
    }

    #[test]
    fn attach_record_batch_metadata_merges_existing_entries() {
        let batch = attach_record_batch_metadata(
            &sample_batch(),
            [("wendao.schema_version", "v1"), ("trace_id", "trace-123")],
        )
        .expect("attach metadata");
        let updated = attach_record_batch_metadata(
            &batch,
            [("trace_id", "trace-456"), ("request_id", "req-1")],
        )
        .expect("merge metadata");

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
    }

    #[test]
    fn attach_record_batch_trace_id_sets_canonical_trace_metadata() {
        let batch =
            attach_record_batch_trace_id(&sample_batch(), "trace-123").expect("attach trace id");

        assert_eq!(
            batch.schema().metadata().get("trace_id"),
            Some(&"trace-123".to_string())
        );
    }

    fn sample_batch() -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![
            Field::new("doc_id", DataType::Utf8, false),
            Field::new("score", DataType::Float64, false),
        ]));
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["doc-1", "doc-2"])),
                Arc::new(Float64Array::from(vec![0.9, 0.4])),
            ],
        )
        .expect("build sample batch")
    }

    fn assert_string_column_eq(expected: &RecordBatch, actual: &RecordBatch, column: &str) {
        let expected = expected
            .column_by_name(column)
            .and_then(|array| array.as_any().downcast_ref::<StringArray>())
            .expect("expected Utf8 column");
        let actual = actual
            .column_by_name(column)
            .and_then(|array| array.as_any().downcast_ref::<StringArray>())
            .expect("actual Utf8 column");

        assert_eq!(expected.len(), actual.len());
        for row in 0..expected.len() {
            assert_eq!(expected.value(row), actual.value(row));
        }
    }

    fn assert_float_column_eq(expected: &RecordBatch, actual: &RecordBatch, column: &str) {
        let expected = expected
            .column_by_name(column)
            .and_then(|array| array.as_any().downcast_ref::<Float64Array>())
            .expect("expected Float64 column");
        let actual = actual
            .column_by_name(column)
            .and_then(|array| array.as_any().downcast_ref::<Float64Array>())
            .expect("actual Float64 column");

        assert_eq!(expected.len(), actual.len());
        for row in 0..expected.len() {
            assert_eq!(expected.value(row), actual.value(row));
        }
    }
}
