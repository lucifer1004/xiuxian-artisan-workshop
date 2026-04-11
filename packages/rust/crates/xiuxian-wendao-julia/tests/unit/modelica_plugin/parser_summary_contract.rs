use std::sync::Arc;

use arrow::array::{BooleanArray, Int32Array, NullArray, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;

use super::{
    MODELICA_PARSER_SUMMARY_BACKEND_COLUMN, MODELICA_PARSER_SUMMARY_CLASS_NAME_COLUMN,
    MODELICA_PARSER_SUMMARY_ERROR_MESSAGE_COLUMN, MODELICA_PARSER_SUMMARY_ITEM_CLASS_PATH_COLUMN,
    MODELICA_PARSER_SUMMARY_ITEM_COMPONENT_KIND_COLUMN,
    MODELICA_PARSER_SUMMARY_ITEM_DEFAULT_VALUE_COLUMN, MODELICA_PARSER_SUMMARY_ITEM_GROUP_COLUMN,
    MODELICA_PARSER_SUMMARY_ITEM_IS_ENCAPSULATED_COLUMN,
    MODELICA_PARSER_SUMMARY_ITEM_IS_FINAL_COLUMN, MODELICA_PARSER_SUMMARY_ITEM_IS_PARTIAL_COLUMN,
    MODELICA_PARSER_SUMMARY_ITEM_KIND_COLUMN, MODELICA_PARSER_SUMMARY_ITEM_LINE_END_COLUMN,
    MODELICA_PARSER_SUMMARY_ITEM_LINE_START_COLUMN, MODELICA_PARSER_SUMMARY_ITEM_NAME_COLUMN,
    MODELICA_PARSER_SUMMARY_ITEM_OWNER_NAME_COLUMN, MODELICA_PARSER_SUMMARY_ITEM_OWNER_PATH_COLUMN,
    MODELICA_PARSER_SUMMARY_ITEM_SIGNATURE_COLUMN, MODELICA_PARSER_SUMMARY_ITEM_TEXT_COLUMN,
    MODELICA_PARSER_SUMMARY_ITEM_TOP_LEVEL_COLUMN, MODELICA_PARSER_SUMMARY_ITEM_TYPE_NAME_COLUMN,
    MODELICA_PARSER_SUMMARY_ITEM_UNIT_COLUMN, MODELICA_PARSER_SUMMARY_ITEM_VARIABILITY_COLUMN,
    MODELICA_PARSER_SUMMARY_ITEM_VISIBILITY_COLUMN, MODELICA_PARSER_SUMMARY_KIND_COLUMN,
    MODELICA_PARSER_SUMMARY_PRIMARY_NAME_COLUMN, MODELICA_PARSER_SUMMARY_REQUEST_ID_COLUMN,
    MODELICA_PARSER_SUMMARY_RESTRICTION_COLUMN, MODELICA_PARSER_SUMMARY_SOURCE_ID_COLUMN,
    MODELICA_PARSER_SUMMARY_SUCCESS_COLUMN, decode_modelica_parser_file_summary,
    decode_modelica_parser_summary_response_rows,
};
use crate::modelica_plugin::parser_summary::transport::ParserSummaryRouteKind;

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

#[test]
fn decode_modelica_parser_file_summary_preserves_declaration_attributes() {
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
            Field::new(
                MODELICA_PARSER_SUMMARY_RESTRICTION_COLUMN,
                DataType::Utf8,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_GROUP_COLUMN,
                DataType::Utf8,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_NAME_COLUMN,
                DataType::Utf8,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_KIND_COLUMN,
                DataType::Utf8,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_SIGNATURE_COLUMN,
                DataType::Utf8,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_TEXT_COLUMN,
                DataType::Utf8,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_LINE_START_COLUMN,
                DataType::Int32,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_LINE_END_COLUMN,
                DataType::Int32,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_OWNER_NAME_COLUMN,
                DataType::Utf8,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_OWNER_PATH_COLUMN,
                DataType::Utf8,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_VISIBILITY_COLUMN,
                DataType::Utf8,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_TYPE_NAME_COLUMN,
                DataType::Utf8,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_VARIABILITY_COLUMN,
                DataType::Utf8,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_COMPONENT_KIND_COLUMN,
                DataType::Utf8,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_DEFAULT_VALUE_COLUMN,
                DataType::Utf8,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_UNIT_COLUMN,
                DataType::Utf8,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_CLASS_PATH_COLUMN,
                DataType::Utf8,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_TOP_LEVEL_COLUMN,
                DataType::Boolean,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_IS_PARTIAL_COLUMN,
                DataType::Boolean,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_IS_FINAL_COLUMN,
                DataType::Boolean,
                true,
            ),
            Field::new(
                MODELICA_PARSER_SUMMARY_ITEM_IS_ENCAPSULATED_COLUMN,
                DataType::Boolean,
                true,
            ),
        ])),
        vec![
            Arc::new(StringArray::from(vec![Some("req-1"), Some("req-1")])),
            Arc::new(StringArray::from(vec![Some("Demo.mo"), Some("Demo.mo")])),
            Arc::new(StringArray::from(vec![
                Some("modelica_file_summary"),
                Some("modelica_file_summary"),
            ])),
            Arc::new(StringArray::from(vec![
                Some("OMParser.jl"),
                Some("OMParser.jl"),
            ])),
            Arc::new(BooleanArray::from(vec![Some(true), Some(true)])),
            Arc::new(NullArray::new(2)),
            Arc::new(NullArray::new(2)),
            Arc::new(StringArray::from(vec![Some("PI"), Some("PI")])),
            Arc::new(StringArray::from(vec![Some("model"), Some("model")])),
            Arc::new(StringArray::from(vec![Some("symbol"), Some("equation")])),
            Arc::new(StringArray::from(vec![Some("k"), Some("PI")])),
            Arc::new(StringArray::from(vec![Some("parameter"), None])),
            Arc::new(StringArray::from(vec![Some("parameter Real k = 1;"), None])),
            Arc::new(StringArray::from(vec![None, Some("y = k;")])),
            Arc::new(Int32Array::from(vec![Some(2), Some(4)])),
            Arc::new(Int32Array::from(vec![Some(2), Some(4)])),
            Arc::new(StringArray::from(vec![Some("PI"), Some("PI")])),
            Arc::new(StringArray::from(vec![Some("PI"), Some("PI")])),
            Arc::new(StringArray::from(vec![Some("public"), None])),
            Arc::new(StringArray::from(vec![Some("Real"), None])),
            Arc::new(StringArray::from(vec![Some("parameter"), None])),
            Arc::new(StringArray::from(vec![Some("component"), None])),
            Arc::new(StringArray::from(vec![Some("1"), None])),
            Arc::new(StringArray::from(vec![Some("kg"), None])),
            Arc::new(StringArray::from(vec![Some("PI"), Some("PI")])),
            Arc::new(BooleanArray::from(vec![Some(false), None])),
            Arc::new(BooleanArray::from(vec![Some(false), None])),
            Arc::new(BooleanArray::from(vec![Some(true), None])),
            Arc::new(BooleanArray::from(vec![Some(false), None])),
        ],
    )
    .unwrap_or_else(|error| panic!("build sample batch: {error}"));

    let rows = decode_modelica_parser_summary_response_rows(&[batch])
        .unwrap_or_else(|error| panic!("decode modelica parser-summary response rows: {error}"));
    let summary = decode_modelica_parser_file_summary(ParserSummaryRouteKind::FileSummary, &rows)
        .unwrap_or_else(|error| panic!("decode modelica parser file summary: {error}"));

    assert_eq!(summary.declarations.len(), 1);
    let declaration = &summary.declarations[0];
    assert_eq!(declaration.name, "k");
    assert_eq!(
        declaration.attributes.get("visibility").map(String::as_str),
        Some("public"),
    );
    assert_eq!(
        declaration
            .attributes
            .get("variability")
            .map(String::as_str),
        Some("parameter"),
    );
    assert_eq!(
        declaration.attributes.get("type_name").map(String::as_str),
        Some("Real"),
    );
    assert_eq!(
        declaration.attributes.get("unit").map(String::as_str),
        Some("kg"),
    );
    assert_eq!(
        declaration.attributes.get("owner_path").map(String::as_str),
        Some("PI"),
    );
    assert_eq!(
        declaration
            .attributes
            .get("equation_latex")
            .map(String::as_str),
        Some("y = k;"),
    );
}
