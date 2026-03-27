use std::io::Cursor;
use std::sync::Arc;

use arrow::array::ArrayRef;
use arrow::datatypes::Schema;
use arrow::ipc::writer::StreamWriter;
use arrow::record_batch::RecordBatch;
use axum::response::{IntoResponse, Response};

use crate::gateway::studio::router::retrieval_arrow::{
    RETRIEVAL_ARROW_CONTENT_TYPE, RETRIEVAL_ARROW_SCHEMA_VERSION,
    RETRIEVAL_ARROW_SCHEMA_VERSION_HEADER,
};

pub(super) fn build_arrow_search_ipc(
    schema: Schema,
    columns: Vec<ArrayRef>,
) -> Result<Vec<u8>, String> {
    let batch = RecordBatch::try_new(Arc::new(schema.clone()), columns)
        .map_err(|error| error.to_string())?;
    encode_record_batch_ipc(&schema, &batch)
}

pub(super) fn encode_optional_json<T: serde::Serialize>(
    value: Option<&T>,
) -> Result<Option<String>, String> {
    value
        .map(|value| serde_json::to_string(value).map_err(|error| error.to_string()))
        .transpose()
}

pub(super) fn encode_json<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| error.to_string())
}

pub(super) fn arrow_payload_response(payload: Vec<u8>) -> Response {
    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        axum::http::HeaderValue::from_static(RETRIEVAL_ARROW_CONTENT_TYPE),
    );
    headers.insert(
        axum::http::HeaderName::from_static(RETRIEVAL_ARROW_SCHEMA_VERSION_HEADER),
        axum::http::HeaderValue::from_static(RETRIEVAL_ARROW_SCHEMA_VERSION),
    );
    (headers, payload).into_response()
}

fn encode_record_batch_ipc(schema: &Schema, batch: &RecordBatch) -> Result<Vec<u8>, String> {
    let mut buffer = Cursor::new(Vec::new());
    {
        let mut writer =
            StreamWriter::try_new(&mut buffer, schema).map_err(|error| error.to_string())?;
        writer.write(batch).map_err(|error| error.to_string())?;
        writer.finish().map_err(|error| error.to_string())?;
    }
    Ok(buffer.into_inner())
}
