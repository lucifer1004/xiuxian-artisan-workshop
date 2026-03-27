use std::io::Cursor;
use std::sync::Arc;

use arrow::array::{StringArray, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::ipc::writer::StreamWriter;
use arrow::record_batch::RecordBatch;
use axum::http::{
    HeaderMap, HeaderValue,
    header::{CONTENT_TYPE, HeaderName},
};
use axum::response::{IntoResponse, Response};

use crate::gateway::studio::types::{RetrievalChunk, RetrievalChunkSurface};

pub(crate) const RETRIEVAL_ARROW_CONTENT_TYPE: &str = "application/vnd.apache.arrow.stream";
pub(crate) const RETRIEVAL_ARROW_SCHEMA_VERSION: &str = "v1";
pub(crate) const RETRIEVAL_ARROW_SCHEMA_VERSION_HEADER: &str = "x-wendao-schema-version";

pub(crate) fn encode_retrieval_chunks_ipc(chunks: &[RetrievalChunk]) -> Result<Vec<u8>, String> {
    let owner_ids: Vec<&str> = chunks.iter().map(|chunk| chunk.owner_id.as_str()).collect();
    let chunk_ids: Vec<&str> = chunks.iter().map(|chunk| chunk.chunk_id.as_str()).collect();
    let semantic_types: Vec<&str> = chunks
        .iter()
        .map(|chunk| chunk.semantic_type.as_str())
        .collect();
    let fingerprints: Vec<&str> = chunks
        .iter()
        .map(|chunk| chunk.fingerprint.as_str())
        .collect();
    let token_estimates: Vec<u64> = chunks
        .iter()
        .map(|chunk| {
            u64::try_from(chunk.token_estimate)
                .map_err(|_| "tokenEstimate exceeds u64 range".to_string())
        })
        .collect::<Result<_, _>>()?;
    let display_labels: Vec<Option<&str>> = chunks
        .iter()
        .map(|chunk| chunk.display_label.as_deref())
        .collect();
    let excerpts: Vec<Option<&str>> = chunks
        .iter()
        .map(|chunk| chunk.excerpt.as_deref())
        .collect();
    let line_starts: Vec<Option<u64>> = chunks
        .iter()
        .map(|chunk| {
            chunk
                .line_start
                .map(|value| {
                    u64::try_from(value).map_err(|_| "lineStart exceeds u64 range".to_string())
                })
                .transpose()
        })
        .collect::<Result<_, _>>()?;
    let line_ends: Vec<Option<u64>> = chunks
        .iter()
        .map(|chunk| {
            chunk
                .line_end
                .map(|value| {
                    u64::try_from(value).map_err(|_| "lineEnd exceeds u64 range".to_string())
                })
                .transpose()
        })
        .collect::<Result<_, _>>()?;
    let surfaces: Vec<Option<&str>> = chunks
        .iter()
        .map(|chunk| chunk.surface.map(retrieval_surface_label))
        .collect();

    let schema = Schema::new(vec![
        Field::new("ownerId", DataType::Utf8, false),
        Field::new("chunkId", DataType::Utf8, false),
        Field::new("semanticType", DataType::Utf8, false),
        Field::new("fingerprint", DataType::Utf8, false),
        Field::new("tokenEstimate", DataType::UInt64, false),
        Field::new("displayLabel", DataType::Utf8, true),
        Field::new("excerpt", DataType::Utf8, true),
        Field::new("lineStart", DataType::UInt64, true),
        Field::new("lineEnd", DataType::UInt64, true),
        Field::new("surface", DataType::Utf8, true),
    ]);

    let batch = RecordBatch::try_new(
        Arc::new(schema.clone()),
        vec![
            Arc::new(StringArray::from(owner_ids)),
            Arc::new(StringArray::from(chunk_ids)),
            Arc::new(StringArray::from(semantic_types)),
            Arc::new(StringArray::from(fingerprints)),
            Arc::new(UInt64Array::from(token_estimates)),
            Arc::new(StringArray::from(display_labels)),
            Arc::new(StringArray::from(excerpts)),
            Arc::new(UInt64Array::from(line_starts)),
            Arc::new(UInt64Array::from(line_ends)),
            Arc::new(StringArray::from(surfaces)),
        ],
    )
    .map_err(|error| error.to_string())?;

    let mut buffer = Cursor::new(Vec::new());
    {
        let mut writer =
            StreamWriter::try_new(&mut buffer, &schema).map_err(|error| error.to_string())?;
        writer.write(&batch).map_err(|error| error.to_string())?;
        writer.finish().map_err(|error| error.to_string())?;
    }
    Ok(buffer.into_inner())
}

pub(crate) fn retrieval_chunks_arrow_response(
    chunks: &[RetrievalChunk],
) -> Result<Response, String> {
    let payload = encode_retrieval_chunks_ipc(chunks)?;
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(RETRIEVAL_ARROW_CONTENT_TYPE),
    );
    headers.insert(
        HeaderName::from_static(RETRIEVAL_ARROW_SCHEMA_VERSION_HEADER),
        HeaderValue::from_static(RETRIEVAL_ARROW_SCHEMA_VERSION),
    );
    Ok((headers, payload).into_response())
}

const fn retrieval_surface_label(surface: RetrievalChunkSurface) -> &'static str {
    match surface {
        RetrievalChunkSurface::Document => "document",
        RetrievalChunkSurface::Section => "section",
        RetrievalChunkSurface::CodeBlock => "codeblock",
        RetrievalChunkSurface::Table => "table",
        RetrievalChunkSurface::Math => "math",
        RetrievalChunkSurface::Observation => "observation",
        RetrievalChunkSurface::Declaration => "declaration",
        RetrievalChunkSurface::Block => "block",
        RetrievalChunkSurface::Symbol => "symbol",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{StringArray, UInt64Array};
    use arrow::ipc::reader::StreamReader;

    #[test]
    fn retrieval_arrow_roundtrip_preserves_chunk_fields() {
        let chunks = vec![
            RetrievalChunk {
                owner_id: "section:intro".to_string(),
                chunk_id: "md:intro".to_string(),
                semantic_type: "section".to_string(),
                fingerprint: "fp:intro".to_string(),
                token_estimate: 18,
                display_label: Some("Intro".to_string()),
                excerpt: Some("Hello world".to_string()),
                line_start: Some(1),
                line_end: Some(4),
                surface: Some(RetrievalChunkSurface::Section),
            },
            RetrievalChunk {
                owner_id: "block:return:solve".to_string(),
                chunk_id: "ast:return:solve".to_string(),
                semantic_type: "return".to_string(),
                fingerprint: "fp:return".to_string(),
                token_estimate: 9,
                display_label: None,
                excerpt: None,
                line_start: Some(22),
                line_end: Some(24),
                surface: Some(RetrievalChunkSurface::Block),
            },
        ];

        let encoded = encode_retrieval_chunks_ipc(&chunks).expect("arrow encoding should succeed");
        let reader =
            StreamReader::try_new(Cursor::new(encoded), None).expect("stream reader should open");
        let batches = reader
            .collect::<Result<Vec<_>, _>>()
            .expect("batches should decode");
        assert_eq!(batches.len(), 1);
        let batch = &batches[0];
        assert_eq!(batch.num_rows(), 2);

        let owner_ids = batch
            .column_by_name("ownerId")
            .expect("ownerId column")
            .as_any()
            .downcast_ref::<StringArray>()
            .expect("ownerId should be utf8");
        assert_eq!(owner_ids.value(0), "section:intro");
        assert_eq!(owner_ids.value(1), "block:return:solve");

        let token_estimates = batch
            .column_by_name("tokenEstimate")
            .expect("tokenEstimate column")
            .as_any()
            .downcast_ref::<UInt64Array>()
            .expect("tokenEstimate should be u64");
        assert_eq!(token_estimates.value(0), 18);
        assert_eq!(token_estimates.value(1), 9);

        let surfaces = batch
            .column_by_name("surface")
            .expect("surface column")
            .as_any()
            .downcast_ref::<StringArray>()
            .expect("surface should be utf8");
        assert_eq!(surfaces.value(0), "section");
        assert_eq!(surfaces.value(1), "block");
    }
}
