use std::sync::Arc;

use arrow::array::{BooleanArray, NullArray, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;

use super::{
    MODELICA_PARSER_SUMMARY_BACKEND_COLUMN, MODELICA_PARSER_SUMMARY_CLASS_NAME_COLUMN,
    MODELICA_PARSER_SUMMARY_ERROR_MESSAGE_COLUMN, MODELICA_PARSER_SUMMARY_KIND_COLUMN,
    MODELICA_PARSER_SUMMARY_PRIMARY_NAME_COLUMN, MODELICA_PARSER_SUMMARY_REQUEST_ID_COLUMN,
    MODELICA_PARSER_SUMMARY_SOURCE_ID_COLUMN, MODELICA_PARSER_SUMMARY_SUCCESS_COLUMN,
    decode_modelica_parser_summary_response_rows,
};

#[test]
fn decode_modelica_parser_summary_rows_accepts_null_optional_columns() {
    let batch = RecordBatch::try_new(
        Arc::new(Schema::new(vec![
            Field::new(
                MODELICA_PARSER_SUMMARY_REQUEST_ID_COLUMN,
                DataType::Utf8,
                false,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_SOURCE_ID_COLUMN,
                DataType::Utf8,
                false,
            ),
            Field::new(MODELICA_PARSER_SUMMARY_KIND_COLUMN, DataType::Utf8, false),
            Field::new(
                MODELICA_PARSER_SUMMARY_BACKEND_COLUMN,
                DataType::Utf8,
                false,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_SUCCESS_COLUMN,
                DataType::Boolean,
                false,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_PRIMARY_NAME_COLUMN,
                DataType::Null,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ERROR_MESSAGE_COLUMN,
                DataType::Null,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_CLASS_NAME_COLUMN,
                DataType::Utf8,
                true,
            ),
        ])),
        vec![
            Arc::new(StringArray::from(vec![Some("req-1")])),
            Arc::new(StringArray::from(vec![Some("Demo.mo")])),
            Arc::new(StringArray::from(vec![Some("modelica_file_summary")])),
            Arc::new(StringArray::from(vec![Some("OMParser.jl")])),
            Arc::new(BooleanArray::from(vec![Some(true)])),
            Arc::new(NullArray::new(1)),
            Arc::new(NullArray::new(1)),
            Arc::new(StringArray::from(vec![Some("Demo")])),
        ],
    )
    .unwrap_or_else(|error| panic!("build sample batch: {error}"));

    let rows = decode_modelica_parser_summary_response_rows(&[batch])
        .unwrap_or_else(|error| panic!("decode modelica parser-summary response rows: {error}"));

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].request_id, "req-1");
    assert_eq!(rows[0].primary_name, None);
    assert_eq!(rows[0].error_message, None);
    assert_eq!(rows[0].class_name.as_deref(), Some("Demo"));
}
