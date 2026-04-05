use std::sync::Arc;

use arrow_array::types::Float32Type;
use arrow_array::{
    FixedSizeListArray, Float32Array, Float64Array, Int32Array, RecordBatch, StringArray,
};
use arrow_schema::{DataType, Field, Schema};

use super::{
    RERANK_REQUEST_DOC_ID_COLUMN, RERANK_REQUEST_EMBEDDING_COLUMN,
    RERANK_REQUEST_QUERY_EMBEDDING_COLUMN, RERANK_REQUEST_VECTOR_SCORE_COLUMN,
    RERANK_RESPONSE_DOC_ID_COLUMN, RERANK_RESPONSE_FINAL_SCORE_COLUMN, RERANK_RESPONSE_RANK_COLUMN,
    RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN, RERANK_RESPONSE_VECTOR_SCORE_COLUMN, RerankScoreWeights,
    must_err, must_ok, score_rerank_request_batch, score_rerank_request_batch_with_weights,
    validate_rerank_request_batch, validate_rerank_request_schema, validate_rerank_response_batch,
    validate_rerank_response_schema,
};

#[test]
fn rerank_request_schema_validation_accepts_stable_shape() {
    let schema = Schema::new(vec![
        Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
        Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
        Field::new(
            RERANK_REQUEST_EMBEDDING_COLUMN,
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
            false,
        ),
        Field::new(
            RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
            false,
        ),
    ]);

    assert!(validate_rerank_request_schema(&schema, 3).is_ok());
}

#[test]
fn rerank_request_schema_validation_rejects_wrong_scalar_type() {
    let schema = Schema::new(vec![
        Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
        Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float64, false),
        Field::new(
            RERANK_REQUEST_EMBEDDING_COLUMN,
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
            false,
        ),
        Field::new(
            RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
            false,
        ),
    ]);

    assert_eq!(
        validate_rerank_request_schema(&schema, 3),
        Err("rerank request column `vector_score` must be Float32".to_string())
    );
}

#[test]
fn rerank_request_schema_validation_rejects_dimension_drift() {
    let schema = Schema::new(vec![
        Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
        Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
        Field::new(
            RERANK_REQUEST_EMBEDDING_COLUMN,
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 2),
            false,
        ),
        Field::new(
            RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 2),
            false,
        ),
    ]);

    assert_eq!(
        validate_rerank_request_schema(&schema, 3),
        Err("rerank request column `embedding` must use dimension 3, got 2".to_string())
    );
}

fn build_rerank_request_batch(
    doc_ids: Vec<&str>,
    vector_scores: Vec<f32>,
    embeddings: Vec<Vec<f32>>,
    query_embeddings: Vec<Vec<f32>>,
) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
        Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
        Field::new(
            RERANK_REQUEST_EMBEDDING_COLUMN,
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
            false,
        ),
        Field::new(
            RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
            DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
            false,
        ),
    ]));

    let embedding_values = embeddings
        .into_iter()
        .map(|row| Some(row.into_iter().map(Some).collect::<Vec<Option<f32>>>()));
    let query_embedding_values = query_embeddings
        .into_iter()
        .map(|row| Some(row.into_iter().map(Some).collect::<Vec<Option<f32>>>()));

    must_ok(
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(doc_ids)),
                Arc::new(Float32Array::from(vector_scores)),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        embedding_values,
                        3,
                    ),
                ),
                Arc::new(
                    FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                        query_embedding_values,
                        3,
                    ),
                ),
            ],
        ),
        "record batch should build",
    )
}

#[test]
fn rerank_request_batch_validation_accepts_stable_semantics() {
    let batch = build_rerank_request_batch(
        vec!["doc-1", "doc-2"],
        vec![0.9_f32, 0.8_f32],
        vec![
            vec![0.1_f32, 0.2_f32, 0.3_f32],
            vec![0.4_f32, 0.5_f32, 0.6_f32],
        ],
        vec![
            vec![0.7_f32, 0.8_f32, 0.9_f32],
            vec![0.7_f32, 0.8_f32, 0.9_f32],
        ],
    );

    assert!(validate_rerank_request_batch(&batch, 3).is_ok());
}

#[test]
fn rerank_request_batch_validation_rejects_blank_doc_id() {
    let batch = build_rerank_request_batch(
        vec![" "],
        vec![0.9_f32],
        vec![vec![0.1_f32, 0.2_f32, 0.3_f32]],
        vec![vec![0.7_f32, 0.8_f32, 0.9_f32]],
    );

    assert_eq!(
        validate_rerank_request_batch(&batch, 3),
        Err(
            "rerank request column `doc_id` must not contain blank values; row 0 is blank"
                .to_string()
        )
    );
}

#[test]
fn rerank_request_batch_validation_rejects_duplicate_doc_id() {
    let batch = build_rerank_request_batch(
        vec!["doc-1", "doc-1"],
        vec![0.9_f32, 0.8_f32],
        vec![
            vec![0.1_f32, 0.2_f32, 0.3_f32],
            vec![0.4_f32, 0.5_f32, 0.6_f32],
        ],
        vec![
            vec![0.7_f32, 0.8_f32, 0.9_f32],
            vec![0.7_f32, 0.8_f32, 0.9_f32],
        ],
    );

    assert_eq!(
        validate_rerank_request_batch(&batch, 3),
        Err(
            "rerank request column `doc_id` must be unique across one batch; row 1 duplicates `doc-1`"
                .to_string()
        )
    );
}

#[test]
fn rerank_request_batch_validation_rejects_out_of_range_vector_score() {
    let batch = build_rerank_request_batch(
        vec!["doc-1"],
        vec![1.2_f32],
        vec![vec![0.1_f32, 0.2_f32, 0.3_f32]],
        vec![vec![0.7_f32, 0.8_f32, 0.9_f32]],
    );

    assert_eq!(
        validate_rerank_request_batch(&batch, 3),
        Err(
            "rerank request column `vector_score` must stay within inclusive range [0.0, 1.0]; row 0 is 1.2"
                .to_string()
        )
    );
}

#[test]
fn rerank_request_batch_validation_rejects_query_embedding_drift() {
    let batch = build_rerank_request_batch(
        vec!["doc-1", "doc-2"],
        vec![0.9_f32, 0.8_f32],
        vec![
            vec![0.1_f32, 0.2_f32, 0.3_f32],
            vec![0.4_f32, 0.5_f32, 0.6_f32],
        ],
        vec![
            vec![0.7_f32, 0.8_f32, 0.9_f32],
            vec![1.0_f32, 1.1_f32, 1.2_f32],
        ],
    );

    assert_eq!(
        validate_rerank_request_batch(&batch, 3),
        Err(
            "rerank request column `query_embedding` must remain stable across all rows; row 1 differs from row 0"
                .to_string()
        )
    );
}

#[test]
fn rerank_request_batch_scoring_blends_vector_and_semantic_similarity() {
    let batch = build_rerank_request_batch(
        vec!["doc-0", "doc-1"],
        vec![0.5_f32, 0.8_f32],
        vec![
            vec![1.0_f32, 0.0_f32, 0.0_f32],
            vec![0.0_f32, 1.0_f32, 0.0_f32],
        ],
        vec![
            vec![1.0_f32, 0.0_f32, 0.0_f32],
            vec![1.0_f32, 0.0_f32, 0.0_f32],
        ],
    );

    let scored = must_ok(
        score_rerank_request_batch(&batch, 3),
        "rerank scoring should succeed",
    );

    assert_eq!(scored.len(), 2);
    assert_eq!(scored[0].doc_id, "doc-0");
    assert!((scored[0].vector_score - 0.5).abs() < 1e-6);
    assert!((scored[0].semantic_score - 1.0).abs() < 1e-6);
    assert!((scored[0].final_score - 0.8).abs() < 1e-6);
    assert_eq!(scored[1].doc_id, "doc-1");
    assert!((scored[1].vector_score - 0.8).abs() < 1e-6);
    assert!((scored[1].semantic_score - 0.5).abs() < 1e-6);
    assert!((scored[1].final_score - 0.62).abs() < 1e-6);
}

#[test]
fn rerank_score_weights_normalize_runtime_policy() {
    let weights = must_ok(RerankScoreWeights::new(2.0, 3.0), "weights should validate");
    let normalized = weights.normalized();

    assert!((normalized.vector_weight - 0.4).abs() < 1e-6);
    assert!((normalized.semantic_weight - 0.6).abs() < 1e-6);
}

#[test]
fn rerank_score_weights_reject_zero_sum_policy() {
    let error = must_err(
        RerankScoreWeights::new(0.0, 0.0),
        "zero-sum weights should fail",
    );
    assert_eq!(error, "rerank score weights must sum to greater than zero");
}

#[test]
fn score_rerank_request_batch_with_weights_respects_runtime_policy() {
    let batch = build_rerank_request_batch(
        vec!["doc-0", "doc-1"],
        vec![0.5_f32, 0.8_f32],
        vec![
            vec![1.0_f32, 0.0_f32, 0.0_f32],
            vec![0.0_f32, 1.0_f32, 0.0_f32],
        ],
        vec![
            vec![1.0_f32, 0.0_f32, 0.0_f32],
            vec![1.0_f32, 0.0_f32, 0.0_f32],
        ],
    );

    let scored = must_ok(
        score_rerank_request_batch_with_weights(
            &batch,
            3,
            must_ok(RerankScoreWeights::new(0.9, 0.1), "weights should validate"),
        ),
        "rerank scoring should succeed",
    );

    assert!((scored[0].final_score - 0.55).abs() < 1e-6);
    assert!((scored[1].final_score - 0.77).abs() < 1e-6);
    assert!(scored[1].final_score > scored[0].final_score);
}

fn build_rerank_response_batch(ranks: Vec<i32>, final_scores: Vec<f64>) -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        Field::new(RERANK_RESPONSE_DOC_ID_COLUMN, DataType::Utf8, false),
        Field::new(
            RERANK_RESPONSE_VECTOR_SCORE_COLUMN,
            DataType::Float64,
            false,
        ),
        Field::new(
            RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN,
            DataType::Float64,
            false,
        ),
        Field::new(RERANK_RESPONSE_FINAL_SCORE_COLUMN, DataType::Float64, false),
        Field::new(RERANK_RESPONSE_RANK_COLUMN, DataType::Int32, false),
    ]));

    must_ok(
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["doc-1", "doc-2"])),
                Arc::new(Float64Array::from(vec![0.91_f64, 0.82_f64])),
                Arc::new(Float64Array::from(vec![0.97_f64, 0.91_f64])),
                Arc::new(Float64Array::from(final_scores)),
                Arc::new(Int32Array::from(ranks)),
            ],
        ),
        "record batch should build",
    )
}

#[test]
fn rerank_response_schema_validation_accepts_stable_shape() {
    let schema = Schema::new(vec![
        Field::new(RERANK_RESPONSE_DOC_ID_COLUMN, DataType::Utf8, false),
        Field::new(
            RERANK_RESPONSE_VECTOR_SCORE_COLUMN,
            DataType::Float64,
            false,
        ),
        Field::new(
            RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN,
            DataType::Float64,
            false,
        ),
        Field::new(RERANK_RESPONSE_FINAL_SCORE_COLUMN, DataType::Float64, false),
        Field::new(RERANK_RESPONSE_RANK_COLUMN, DataType::Int32, false),
    ]);

    assert!(validate_rerank_response_schema(&schema).is_ok());
}

#[test]
fn rerank_response_schema_validation_rejects_wrong_rank_type() {
    let schema = Schema::new(vec![
        Field::new(RERANK_RESPONSE_DOC_ID_COLUMN, DataType::Utf8, false),
        Field::new(
            RERANK_RESPONSE_VECTOR_SCORE_COLUMN,
            DataType::Float64,
            false,
        ),
        Field::new(
            RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN,
            DataType::Float64,
            false,
        ),
        Field::new(RERANK_RESPONSE_FINAL_SCORE_COLUMN, DataType::Float64, false),
        Field::new(RERANK_RESPONSE_RANK_COLUMN, DataType::UInt32, false),
    ]);

    assert_eq!(
        validate_rerank_response_schema(&schema),
        Err("rerank response column `rank` must be Int32".to_string())
    );
}

#[test]
fn rerank_response_batch_validation_accepts_stable_semantics() {
    let batch = build_rerank_response_batch(vec![1_i32, 2_i32], vec![0.97_f64, 0.91_f64]);
    assert!(validate_rerank_response_batch(&batch).is_ok());
}

#[test]
fn rerank_response_batch_validation_rejects_duplicate_rank() {
    let batch = build_rerank_response_batch(vec![1_i32, 1_i32], vec![0.97_f64, 0.91_f64]);

    assert_eq!(
        validate_rerank_response_batch(&batch),
        Err(
            "rerank response column `rank` must be unique across one batch; row 1 duplicates `1`"
                .to_string()
        )
    );
}

#[test]
fn rerank_response_batch_validation_rejects_out_of_range_final_score() {
    let schema = Arc::new(Schema::new(vec![
        Field::new(RERANK_RESPONSE_DOC_ID_COLUMN, DataType::Utf8, false),
        Field::new(
            RERANK_RESPONSE_VECTOR_SCORE_COLUMN,
            DataType::Float64,
            false,
        ),
        Field::new(
            RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN,
            DataType::Float64,
            false,
        ),
        Field::new(RERANK_RESPONSE_FINAL_SCORE_COLUMN, DataType::Float64, false),
        Field::new(RERANK_RESPONSE_RANK_COLUMN, DataType::Int32, false),
    ]));
    let batch = must_ok(
        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(vec!["doc-1"])),
                Arc::new(Float64Array::from(vec![0.9_f64])),
                Arc::new(Float64Array::from(vec![0.95_f64])),
                Arc::new(Float64Array::from(vec![1.2_f64])),
                Arc::new(Int32Array::from(vec![1_i32])),
            ],
        ),
        "record batch should build",
    );

    assert_eq!(
        validate_rerank_response_batch(&batch),
        Err(
            "rerank response column `final_score` must stay within inclusive range [0.0, 1.0]; row 0 is 1.2"
                .to_string()
        )
    );
}
