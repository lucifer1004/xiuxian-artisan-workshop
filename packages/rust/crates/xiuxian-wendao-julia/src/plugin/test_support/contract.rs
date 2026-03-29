use std::sync::Arc;

use arrow::array::{FixedSizeListArray, Float32Array, Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use xiuxian_vector::{
    ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION, ARROW_TRANSPORT_SCHEMA_VERSION_METADATA_KEY,
    attach_record_batch_metadata, attach_record_batch_trace_id,
};
use xiuxian_wendao_core::repo_intelligence::{
    JULIA_ARROW_ANALYZER_SCORE_COLUMN, JULIA_ARROW_DOC_ID_COLUMN, JULIA_ARROW_FINAL_SCORE_COLUMN,
    julia_arrow_request_schema, julia_arrow_response_schema,
};
pub(crate) fn response_batch_without_trace_id() -> RecordBatch {
    RecordBatch::try_new(
        julia_arrow_response_schema(false),
        vec![
            Arc::new(StringArray::from(vec!["doc-a", "doc-b"])),
            Arc::new(Float64Array::from(vec![0.2_f64, 0.7_f64])),
            Arc::new(Float64Array::from(vec![0.5_f64, 0.9_f64])),
        ],
    )
    .expect("valid response batch")
}

pub(crate) fn response_batch_with_duplicates() -> RecordBatch {
    RecordBatch::try_new(
        julia_arrow_response_schema(false),
        vec![
            Arc::new(StringArray::from(vec!["doc-a", "doc-a"])),
            Arc::new(Float64Array::from(vec![0.2_f64, 0.7_f64])),
            Arc::new(Float64Array::from(vec![0.5_f64, 0.9_f64])),
        ],
    )
    .expect("duplicate response batch")
}

pub(crate) fn response_batch() -> RecordBatch {
    RecordBatch::try_new(
        julia_arrow_response_schema(false),
        vec![
            Arc::new(StringArray::from(vec!["doc-a"])),
            Arc::new(Float64Array::from(vec![0.6_f64])),
            Arc::new(Float64Array::from(vec![0.9_f64])),
        ],
    )
    .expect("response batch")
}

pub(crate) fn request_batch() -> RecordBatch {
    let vector_dim = 2;
    RecordBatch::try_new(
        julia_arrow_request_schema(vector_dim),
        vec![
            Arc::new(StringArray::from(vec!["doc-a", "doc-b"])),
            Arc::new(Float64Array::from(vec![0.4_f64, 0.7_f64])),
            Arc::new(fixed_size_vector_array(
                vector_dim,
                vec![0.1_f32, 0.2_f32, 0.3_f32, 0.4_f32],
            )),
            Arc::new(fixed_size_vector_array(
                vector_dim,
                vec![0.9_f32, 0.8_f32, 0.9_f32, 0.8_f32],
            )),
        ],
    )
    .expect("request batch")
}

pub(crate) fn request_batch_with_trace_id(trace_id: &str) -> RecordBatch {
    let batch = request_batch();
    attach_record_batch_metadata(
        &attach_record_batch_trace_id(&batch, trace_id).expect("attach trace metadata"),
        [(
            ARROW_TRANSPORT_SCHEMA_VERSION_METADATA_KEY,
            ARROW_TRANSPORT_DEFAULT_SCHEMA_VERSION,
        )],
    )
    .expect("attach request metadata")
}

pub(crate) fn invalid_response_missing_final_batch() -> RecordBatch {
    RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new(JULIA_ARROW_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(JULIA_ARROW_ANALYZER_SCORE_COLUMN, DataType::Float64, false),
        ])),
        vec![
            Arc::new(StringArray::from(vec!["doc-a"])),
            Arc::new(Float64Array::from(vec![0.6_f64])),
        ],
    )
    .expect("invalid response batch")
}

pub(crate) fn invalid_response_missing_analyzer_score_batch() -> RecordBatch {
    RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new(JULIA_ARROW_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(JULIA_ARROW_FINAL_SCORE_COLUMN, DataType::Float64, false),
        ])),
        vec![
            Arc::new(StringArray::from(vec!["doc-a"])),
            Arc::new(Float64Array::from(vec![0.5_f64])),
        ],
    )
    .expect("missing analyzer_score batch")
}

fn fixed_size_vector_array(vector_dim: i32, values: Vec<f32>) -> FixedSizeListArray {
    FixedSizeListArray::try_new(
        Arc::new(Field::new("item", DataType::Float32, true)),
        vector_dim,
        Arc::new(Float32Array::from(values)),
        None,
    )
    .expect("fixed-size vector array")
}
