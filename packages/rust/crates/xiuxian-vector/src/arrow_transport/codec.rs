use std::io::Cursor;

use arrow::error::ArrowError;
use arrow::record_batch::RecordBatch;
use arrow_ipc::reader::StreamReader;
use arrow_ipc::writer::StreamWriter;

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
