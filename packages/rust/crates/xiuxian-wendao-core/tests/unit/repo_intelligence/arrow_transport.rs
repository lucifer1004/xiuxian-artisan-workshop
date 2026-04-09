use arrow::datatypes::DataType;

use super::{
    JULIA_ARROW_ANALYZER_SCORE_COLUMN, JULIA_ARROW_DOC_ID_COLUMN, JULIA_ARROW_EMBEDDING_COLUMN,
    JULIA_ARROW_FINAL_SCORE_COLUMN, JULIA_ARROW_QUERY_EMBEDDING_COLUMN,
    JULIA_ARROW_TRACE_ID_COLUMN, JULIA_ARROW_VECTOR_SCORE_COLUMN, julia_arrow_request_schema,
    julia_arrow_response_schema,
};

#[test]
fn julia_arrow_request_schema_uses_contract_columns() {
    let schema = julia_arrow_request_schema(3);

    assert_eq!(schema.field(0).name(), JULIA_ARROW_DOC_ID_COLUMN);
    assert_eq!(schema.field(1).name(), JULIA_ARROW_VECTOR_SCORE_COLUMN);
    assert_eq!(schema.field(2).name(), JULIA_ARROW_EMBEDDING_COLUMN);
    assert_eq!(schema.field(3).name(), JULIA_ARROW_QUERY_EMBEDDING_COLUMN);
    match schema.field(2).data_type() {
        DataType::FixedSizeList(_, size) => assert_eq!(*size, 3),
        other => panic!("expected FixedSizeList embedding field, found {other:?}"),
    }
}

#[test]
fn julia_arrow_response_schema_optionally_includes_trace_id() {
    let base = julia_arrow_response_schema(false);
    let traced = julia_arrow_response_schema(true);

    assert_eq!(base.fields().len(), 3);
    assert_eq!(base.field(0).name(), JULIA_ARROW_DOC_ID_COLUMN);
    assert_eq!(base.field(1).name(), JULIA_ARROW_ANALYZER_SCORE_COLUMN);
    assert_eq!(base.field(2).name(), JULIA_ARROW_FINAL_SCORE_COLUMN);
    assert_eq!(traced.fields().len(), 4);
    assert_eq!(traced.field(3).name(), JULIA_ARROW_TRACE_ID_COLUMN);
}
