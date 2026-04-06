use arrow::array::{Float64Array, StringArray};

use super::{
    GraphStructuralCandidateSubgraph, GraphStructuralFilterConstraint,
    GraphStructuralGenericTopologyCandidateInputs,
    GraphStructuralGenericTopologyCandidateMetadataInputs,
    GraphStructuralKeywordOverlapCandidateInputs, GraphStructuralKeywordOverlapPairInputs,
    GraphStructuralKeywordOverlapPairRequestInputs, GraphStructuralKeywordOverlapPairRerankInputs,
    GraphStructuralKeywordOverlapQueryInputs, GraphStructuralKeywordOverlapRawCandidateInputs,
    GraphStructuralKeywordTagQueryInputs, GraphStructuralNodeMetadataInputs,
    GraphStructuralPairCandidateInputs, GraphStructuralQueryAnchor, GraphStructuralQueryContext,
    GraphStructuralRerankSignals, build_graph_structural_filter_request_row,
    build_graph_structural_generic_topology_candidate_inputs,
    build_graph_structural_generic_topology_candidate_inputs_from_pair_collection,
    build_graph_structural_generic_topology_candidate_inputs_from_raw_connected_pairs,
    build_graph_structural_generic_topology_candidate_inputs_from_scored_pair_collection,
    build_graph_structural_generic_topology_candidate_metadata_inputs,
    build_graph_structural_generic_topology_candidate_metadata_inputs_from_pair_collection,
    build_graph_structural_generic_topology_candidate_subgraph,
    build_graph_structural_generic_topology_rerank_request_batch,
    build_graph_structural_generic_topology_rerank_request_batch_from_raw_connected_pair_collections,
    build_graph_structural_generic_topology_rerank_request_row,
    build_graph_structural_keyword_overlap_candidate_inputs,
    build_graph_structural_keyword_overlap_pair_candidate_inputs,
    build_graph_structural_keyword_overlap_pair_candidate_inputs_from_raw,
    build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs,
    build_graph_structural_keyword_overlap_pair_request_input,
    build_graph_structural_keyword_overlap_pair_rerank_request_batch,
    build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_inputs,
    build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_metadata,
    build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_raw_candidates,
    build_graph_structural_keyword_overlap_pair_rerank_request_row,
    build_graph_structural_keyword_overlap_pair_rerank_request_row_from_metadata,
    build_graph_structural_keyword_overlap_query_inputs,
    build_graph_structural_keyword_overlap_raw_candidate_inputs,
    build_graph_structural_keyword_tag_pair_rerank_request_row,
    build_graph_structural_keyword_tag_query_context,
    build_graph_structural_keyword_tag_rerank_signals,
    build_graph_structural_pair_candidate_inputs, build_graph_structural_pair_candidate_subgraph,
    build_graph_structural_pair_filter_request_row, build_graph_structural_pair_rerank_request_row,
    build_graph_structural_raw_connected_pair_collection_candidate_inputs_from_raw_tuples,
    build_graph_structural_raw_connected_pair_inputs, build_graph_structural_rerank_request_row,
    build_graph_structural_scored_pair_candidate_inputs, graph_structural_pair_candidate_id,
    graph_structural_shared_tag_anchors,
};
use crate::{
    build_graph_structural_filter_request_batch, build_graph_structural_rerank_request_batch,
};

#[test]
fn build_graph_structural_rerank_request_row_projects_semantic_dtos() {
    let query = GraphStructuralQueryContext::new(
        "query-1",
        1,
        3,
        vec![
            GraphStructuralQueryAnchor::new("semantic", "symbol:entry").expect("semantic anchor"),
            GraphStructuralQueryAnchor::new("tag", "core").expect("tag anchor"),
        ],
        vec!["depends_on".to_string()],
    )
    .expect("query context");
    let candidate = GraphStructuralCandidateSubgraph::new(
        "pair:node-1:node-2",
        vec!["node-1".to_string(), "node-2".to_string()],
        vec!["node-1".to_string()],
        vec!["node-2".to_string()],
        vec!["related".to_string()],
    )
    .expect("candidate");
    let signals = GraphStructuralRerankSignals::new(0.7, 0.4, 0.2, 0.3).expect("rerank signals");

    let row = build_graph_structural_rerank_request_row(&query, &candidate, &signals);
    let batch = build_graph_structural_rerank_request_batch(&[row.clone()])
        .expect("rerank batch should validate");

    assert_eq!(row.query_id, "query-1");
    assert_eq!(row.candidate_id, "pair:node-1:node-2");
    assert_eq!(row.anchor_planes, vec!["semantic", "tag"]);
    assert_eq!(row.anchor_values, vec!["symbol:entry", "core"]);
    assert_eq!(row.edge_constraint_kinds, vec!["depends_on"]);
    assert_eq!(row.candidate_node_ids, vec!["node-1", "node-2"]);
    assert_eq!(row.candidate_edge_sources, vec!["node-1"]);
    assert_eq!(row.candidate_edge_destinations, vec!["node-2"]);
    assert_eq!(batch.num_rows(), 1);
}

#[test]
fn build_graph_structural_filter_request_row_allows_empty_edge_lists() {
    let query = GraphStructuralQueryContext::new(
        "query-2",
        0,
        2,
        vec![GraphStructuralQueryAnchor::new("keyword", "solver").expect("keyword anchor")],
        Vec::new(),
    )
    .expect("query context");
    let candidate = GraphStructuralCandidateSubgraph::new(
        "candidate-a",
        vec!["node-a".to_string()],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .expect("candidate");
    let constraint = GraphStructuralFilterConstraint::new("boundary-match", 1).expect("constraint");

    let row = build_graph_structural_filter_request_row(&query, &candidate, &constraint);
    let batch = build_graph_structural_filter_request_batch(&[row.clone()])
        .expect("filter batch should validate");

    assert_eq!(row.edge_constraint_kinds, Vec::<String>::new());
    assert_eq!(row.candidate_edge_sources, Vec::<String>::new());
    assert_eq!(row.candidate_edge_destinations, Vec::<String>::new());
    assert_eq!(row.candidate_edge_kinds, Vec::<String>::new());
    assert_eq!(row.constraint_kind, "boundary-match");
    assert_eq!(row.required_boundary_size, 1);
    assert_eq!(batch.num_rows(), 1);
}

#[test]
fn build_graph_structural_pair_candidate_subgraph_normalizes_stable_id() {
    let candidate = build_graph_structural_pair_candidate_subgraph(
        "node-z",
        "node-a",
        vec!["related".to_string()],
    )
    .expect("pair candidate should normalize");

    assert_eq!(candidate.candidate_id(), "pair:node-a:node-z");
    assert_eq!(
        candidate.node_ids(),
        &["node-z".to_string(), "node-a".to_string()]
    );
    assert_eq!(candidate.edge_kinds(), &["related".to_string()]);
}

#[test]
fn build_graph_structural_pair_rerank_request_row_projects_pair_inputs() {
    let query = GraphStructuralQueryContext::new(
        "query-3",
        0,
        2,
        vec![GraphStructuralQueryAnchor::new("keyword", "alpha").expect("keyword anchor")],
        Vec::new(),
    )
    .expect("query context");
    let signals = GraphStructuralRerankSignals::new(0.8, 0.1, 1.0, 0.6).expect("rerank signals");

    let row = build_graph_structural_pair_rerank_request_row(
        &query,
        "doc-b",
        "doc-a",
        vec!["semantic_similar".to_string()],
        &signals,
    )
    .expect("pair row should project");
    let batch = build_graph_structural_rerank_request_batch(&[row.clone()])
        .expect("pair rerank batch should validate");

    assert_eq!(row.candidate_id, "pair:doc-a:doc-b");
    assert_eq!(row.candidate_node_ids, vec!["doc-b", "doc-a"]);
    assert_eq!(row.candidate_edge_kinds, vec!["semantic_similar"]);
    assert_eq!(batch.num_rows(), 1);
}

#[test]
fn build_graph_structural_generic_topology_candidate_subgraph_projects_explicit_edges() {
    let candidate = build_graph_structural_generic_topology_candidate_subgraph(
        build_graph_structural_generic_topology_candidate_metadata_inputs(
            "candidate-chain",
            vec![
                "node-1".to_string(),
                "node-2".to_string(),
                "node-3".to_string(),
            ],
            vec!["node-1".to_string(), "node-2".to_string()],
            vec!["node-2".to_string(), "node-3".to_string()],
            vec!["depends_on".to_string(), "references".to_string()],
        ),
    )
    .expect("generic topology candidate should normalize");

    assert_eq!(candidate.candidate_id(), "candidate-chain");
    assert_eq!(
        candidate.node_ids(),
        &[
            "node-1".to_string(),
            "node-2".to_string(),
            "node-3".to_string()
        ]
    );
    assert_eq!(
        candidate.edge_sources(),
        &["node-1".to_string(), "node-2".to_string()]
    );
    assert_eq!(
        candidate.edge_destinations(),
        &["node-2".to_string(), "node-3".to_string()]
    );
    assert_eq!(
        candidate.edge_kinds(),
        &["depends_on".to_string(), "references".to_string()]
    );
}

#[test]
fn build_graph_structural_generic_topology_candidate_metadata_inputs_from_pair_collection_projects_edges()
 {
    let candidate = build_graph_structural_generic_topology_candidate_subgraph(
        build_graph_structural_generic_topology_candidate_metadata_inputs_from_pair_collection(
            "candidate-from-pairs",
            vec![
                build_graph_structural_pair_candidate_inputs("node-1", "node-2", Vec::new()),
                build_graph_structural_pair_candidate_inputs(
                    "node-2",
                    "node-3",
                    vec!["references".to_string()],
                ),
            ],
            "related",
        )
        .expect("pair collection metadata should normalize"),
    )
    .expect("pair collection candidate should normalize");

    assert_eq!(candidate.candidate_id(), "candidate-from-pairs");
    assert_eq!(
        candidate.node_ids(),
        &[
            "node-1".to_string(),
            "node-2".to_string(),
            "node-3".to_string()
        ]
    );
    assert_eq!(
        candidate.edge_sources(),
        &["node-1".to_string(), "node-2".to_string()]
    );
    assert_eq!(
        candidate.edge_destinations(),
        &["node-2".to_string(), "node-3".to_string()]
    );
    assert_eq!(
        candidate.edge_kinds(),
        &["related".to_string(), "references".to_string()]
    );
}

#[test]
fn build_graph_structural_generic_topology_candidate_inputs_from_pair_collection_preserves_scores()
{
    let row = build_graph_structural_generic_topology_rerank_request_row(
        &build_graph_structural_keyword_tag_query_context(
            "query-generic-pairs",
            0,
            2,
            vec!["alpha".to_string()],
            Vec::new(),
            vec!["depends_on".to_string()],
        )
        .expect("query context"),
        build_graph_structural_generic_topology_candidate_inputs_from_pair_collection(
            "candidate-from-pairs",
            vec![
                build_graph_structural_pair_candidate_inputs(
                    "node-1",
                    "node-2",
                    vec!["depends_on".to_string()],
                ),
                build_graph_structural_pair_candidate_inputs("node-2", "node-3", Vec::new()),
            ],
            "related",
            0.7,
            0.6,
            1.0,
            0.0,
        )
        .expect("pair collection candidate should normalize"),
    )
    .expect("pair collection rerank row should project");

    assert_eq!(row.semantic_score, 0.7);
    assert_eq!(row.dependency_score, 0.6);
    assert_eq!(row.keyword_score, 1.0);
    assert_eq!(row.tag_score, 0.0);
    assert_eq!(
        row.candidate_edge_kinds,
        vec!["depends_on".to_string(), "related".to_string()]
    );
}

#[test]
fn build_graph_structural_scored_pair_candidate_inputs_rejects_negative_score() {
    let error = build_graph_structural_scored_pair_candidate_inputs(
        "node-1",
        "node-2",
        vec!["depends_on".to_string()],
        -0.1,
    )
    .expect_err("negative pair score should fail");

    assert!(
        error
            .to_string()
            .contains("pair semantic score must be non-negative"),
        "unexpected error: {error}"
    );
}

#[test]
fn build_graph_structural_generic_topology_candidate_inputs_from_scored_pair_collection_averages_semantic_score()
 {
    let row = build_graph_structural_generic_topology_rerank_request_row(
        &build_graph_structural_keyword_tag_query_context(
            "query-generic-scored-pairs",
            0,
            2,
            vec!["alpha".to_string()],
            Vec::new(),
            vec!["depends_on".to_string()],
        )
        .expect("query context"),
        build_graph_structural_generic_topology_candidate_inputs_from_scored_pair_collection(
            "candidate-from-scored-pairs",
            vec![
                build_graph_structural_scored_pair_candidate_inputs(
                    "node-1",
                    "node-2",
                    vec!["depends_on".to_string()],
                    0.6,
                )
                .expect("scored pair candidate"),
                build_graph_structural_scored_pair_candidate_inputs(
                    "node-2",
                    "node-3",
                    Vec::new(),
                    0.8,
                )
                .expect("scored pair candidate"),
            ],
            "related",
            0.5,
            1.0,
            0.0,
        )
        .expect("scored pair collection candidate should normalize"),
    )
    .expect("scored pair collection rerank row should project");

    assert!((row.semantic_score - 0.7).abs() < f64::EPSILON);
    assert_eq!(row.dependency_score, 0.5);
    assert_eq!(row.keyword_score, 1.0);
    assert_eq!(row.tag_score, 0.0);
    assert_eq!(
        row.candidate_edge_kinds,
        vec!["depends_on".to_string(), "related".to_string()]
    );
}

#[test]
fn build_graph_structural_generic_topology_candidate_inputs_from_raw_connected_pairs_averages_semantic_score()
 {
    let row = build_graph_structural_generic_topology_rerank_request_row(
        &build_graph_structural_keyword_tag_query_context(
            "query-generic-raw-connected-pairs",
            0,
            2,
            vec!["alpha".to_string()],
            Vec::new(),
            vec!["related".to_string()],
        )
        .expect("query context"),
        build_graph_structural_generic_topology_candidate_inputs_from_raw_connected_pairs(
            "candidate-from-raw-connected-pairs",
            vec![
                build_graph_structural_raw_connected_pair_inputs("node-1", "node-2", 0.4)
                    .expect("raw connected pair"),
                build_graph_structural_raw_connected_pair_inputs("node-2", "node-3", 0.8)
                    .expect("raw connected pair"),
            ],
            "related",
            0.3,
            1.0,
            0.0,
        )
        .expect("raw connected pair collection candidate should normalize"),
    )
    .expect("raw connected pair collection rerank row should project");

    assert!((row.semantic_score - 0.6).abs() < f64::EPSILON);
    assert_eq!(row.dependency_score, 0.3);
    assert_eq!(row.keyword_score, 1.0);
    assert_eq!(row.tag_score, 0.0);
    assert_eq!(
        row.candidate_edge_kinds,
        vec!["related".to_string(), "related".to_string()]
    );
}

#[test]
fn build_graph_structural_generic_topology_rerank_request_batch_from_raw_connected_pair_collections_composes()
 {
    let batch =
            build_graph_structural_generic_topology_rerank_request_batch_from_raw_connected_pair_collections(
                &build_graph_structural_keyword_tag_query_context(
                    "query-generic-raw-connected-collections",
                    0,
                    2,
                    vec!["alpha".to_string()],
                    Vec::new(),
                    vec!["related".to_string()],
                )
                .expect("query context"),
                &[build_graph_structural_raw_connected_pair_collection_candidate_inputs_from_raw_tuples(
                    "candidate-from-raw-connected-collection",
                    vec![
                        ("node-1", "node-2", 0.4),
                        ("node-2", "node-3", 0.8),
                    ],
                    "related",
                    0.3,
                    1.0,
                    0.0,
                )
                .expect("raw connected pair collection candidate")],
            )
            .expect("raw connected pair collection batch should project");

    assert_eq!(batch.num_rows(), 1);
    let candidate_ids = batch
        .column_by_name("candidate_id")
        .expect("candidate_id column")
        .as_any()
        .downcast_ref::<StringArray>()
        .expect("candidate_id strings");
    let semantic_scores = batch
        .column_by_name("semantic_score")
        .expect("semantic_score column")
        .as_any()
        .downcast_ref::<Float64Array>()
        .expect("semantic_score floats");

    assert_eq!(
        candidate_ids.value(0),
        "candidate-from-raw-connected-collection"
    );
    assert!((semantic_scores.value(0) - 0.6).abs() < f64::EPSILON);
}

#[test]
fn build_graph_structural_raw_connected_pair_collection_candidate_inputs_from_raw_tuples_rejects_blank_endpoint()
 {
    let error =
        build_graph_structural_raw_connected_pair_collection_candidate_inputs_from_raw_tuples(
            "candidate-from-tuples",
            vec![("node-1", "", 0.4)],
            "related",
            0.3,
            1.0,
            0.0,
        )
        .expect_err("blank endpoint should fail");

    assert!(
        error
            .to_string()
            .contains("pair right id must not be blank"),
        "unexpected error: {error}"
    );
}

#[test]
fn build_graph_structural_generic_topology_rerank_request_row_projects_explicit_topology() {
    let query = build_graph_structural_keyword_tag_query_context(
        "query-generic-row",
        1,
        3,
        vec!["alpha".to_string()],
        vec!["core".to_string()],
        vec!["depends_on".to_string()],
    )
    .expect("query context");
    let row = build_graph_structural_generic_topology_rerank_request_row(
        &query,
        build_graph_structural_generic_topology_candidate_inputs(
            build_graph_structural_generic_topology_candidate_metadata_inputs(
                "candidate-chain",
                vec![
                    "node-1".to_string(),
                    "node-2".to_string(),
                    "node-3".to_string(),
                ],
                vec!["node-1".to_string(), "node-2".to_string()],
                vec!["node-2".to_string(), "node-3".to_string()],
                vec!["depends_on".to_string(), "depends_on".to_string()],
            ),
            0.7,
            0.5,
            1.0,
            0.0,
        ),
    )
    .expect("generic topology rerank row should normalize");

    assert_eq!(row.candidate_id, "candidate-chain");
    assert_eq!(row.candidate_node_ids.len(), 3);
    assert_eq!(row.candidate_edge_sources, vec!["node-1", "node-2"]);
    assert_eq!(row.candidate_edge_destinations, vec!["node-2", "node-3"]);
    assert_eq!(row.candidate_edge_kinds, vec!["depends_on", "depends_on"]);
    assert_eq!(row.keyword_score, 1.0);
}

#[test]
fn build_graph_structural_generic_topology_rerank_request_batch_composes() {
    let query = GraphStructuralQueryContext::new(
        "query-generic-batch",
        0,
        2,
        vec![GraphStructuralQueryAnchor::new("semantic", "symbol:entry").expect("semantic anchor")],
        vec!["depends_on".to_string()],
    )
    .expect("query context");
    let batch = build_graph_structural_generic_topology_rerank_request_batch(
        &query,
        &[build_graph_structural_generic_topology_candidate_inputs(
            build_graph_structural_generic_topology_candidate_metadata_inputs(
                "candidate-chain",
                vec![
                    "node-1".to_string(),
                    "node-2".to_string(),
                    "node-3".to_string(),
                ],
                vec!["node-1".to_string(), "node-2".to_string()],
                vec!["node-2".to_string(), "node-3".to_string()],
                vec!["depends_on".to_string(), "depends_on".to_string()],
            ),
            0.8,
            0.4,
            0.2,
            0.1,
        )],
    )
    .expect("generic topology batch helper should normalize");

    assert_eq!(batch.num_rows(), 1);
    assert_eq!(batch.num_columns(), 15);
}

#[test]
fn build_graph_structural_pair_filter_request_row_rejects_duplicate_endpoints() {
    let query = GraphStructuralQueryContext::new(
        "query-4",
        0,
        1,
        vec![GraphStructuralQueryAnchor::new("tag", "core").expect("tag anchor")],
        Vec::new(),
    )
    .expect("query context");
    let constraint = GraphStructuralFilterConstraint::new("boundary-match", 1).expect("constraint");

    let error = build_graph_structural_pair_filter_request_row(
        &query,
        "node-a",
        "node-a",
        Vec::new(),
        &constraint,
    )
    .expect_err("pair filter row should reject duplicate endpoints");
    assert!(
        error
            .to_string()
            .contains("pair endpoints must not resolve to the same id"),
        "unexpected error: {error}"
    );
}

#[test]
fn build_graph_structural_keyword_tag_query_context_orders_keyword_before_tag() {
    let query = build_graph_structural_keyword_tag_query_context(
        "query-5",
        0,
        2,
        vec![" alpha ".to_string()],
        vec![" core ".to_string(), " graph ".to_string()],
        vec!["depends_on".to_string()],
    )
    .expect("query context should normalize");

    assert_eq!(query.query_id(), "query-5");
    assert_eq!(query.anchors()[0].plane(), "keyword");
    assert_eq!(query.anchors()[0].value(), "alpha");
    assert_eq!(query.anchors()[1].plane(), "tag");
    assert_eq!(query.anchors()[1].value(), "core");
    assert_eq!(query.anchors()[2].plane(), "tag");
    assert_eq!(query.anchors()[2].value(), "graph");
    assert_eq!(query.edge_constraint_kinds(), &["depends_on".to_string()]);
}

#[test]
fn build_graph_structural_keyword_tag_query_context_rejects_empty_anchor_lists() {
    let error = build_graph_structural_keyword_tag_query_context(
        "query-6",
        0,
        1,
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .expect_err("query context should reject empty keyword and tag anchors");
    assert!(
        error
            .to_string()
            .contains("at least one query anchor is required"),
        "unexpected error: {error}"
    );
}

#[test]
fn build_graph_structural_keyword_tag_rerank_signals_maps_binary_matches() {
    let signals = build_graph_structural_keyword_tag_rerank_signals(0.6, 0.2, true, false)
        .expect("binary match signals should normalize");

    assert_eq!(signals.semantic_score(), 0.6);
    assert_eq!(signals.dependency_score(), 0.2);
    assert_eq!(signals.keyword_score(), 1.0);
    assert_eq!(signals.tag_score(), 0.0);
}

#[test]
fn build_graph_structural_keyword_tag_pair_rerank_request_row_composes_helper_layers() {
    let row = build_graph_structural_keyword_tag_pair_rerank_request_row(
        GraphStructuralKeywordTagQueryInputs::new(
            "query-7",
            1,
            3,
            vec!["alpha".to_string()],
            vec!["core".to_string()],
            vec!["depends_on".to_string()],
        ),
        GraphStructuralPairCandidateInputs::new(
            "node-b",
            "node-a",
            vec!["semantic_similar".to_string()],
        ),
        0.75,
        0.1,
        true,
        true,
    )
    .expect("combined helper should normalize");
    let batch = build_graph_structural_rerank_request_batch(&[row.clone()])
        .expect("combined helper batch should validate");

    assert_eq!(row.query_id, "query-7");
    assert_eq!(row.candidate_id, "pair:node-a:node-b");
    assert_eq!(row.anchor_planes, vec!["keyword", "tag"]);
    assert_eq!(row.anchor_values, vec!["alpha", "core"]);
    assert_eq!(row.candidate_edge_kinds, vec!["semantic_similar"]);
    assert_eq!(row.keyword_score, 1.0);
    assert_eq!(row.tag_score, 1.0);
    assert_eq!(batch.num_rows(), 1);
}

#[test]
fn graph_structural_shared_tag_anchors_preserve_left_order_and_uniqueness() {
    let shared = graph_structural_shared_tag_anchors(
        vec![
            "core".to_string(),
            "alpha".to_string(),
            "core".to_string(),
            "graph".to_string(),
        ],
        vec!["graph".to_string(), "core".to_string(), "delta".to_string()],
    )
    .expect("shared tag anchors should normalize");

    assert_eq!(shared, vec!["core".to_string(), "graph".to_string()]);
}

#[test]
fn build_graph_structural_keyword_overlap_pair_rerank_request_row_computes_tag_overlap() {
    let row = build_graph_structural_keyword_overlap_pair_rerank_request_row(
        GraphStructuralKeywordTagQueryInputs::new(
            "query-8",
            0,
            2,
            vec!["alpha".to_string()],
            Vec::new(),
            Vec::new(),
        ),
        vec!["alpha".to_string(), "core".to_string()],
        vec!["graph".to_string(), "core".to_string()],
        GraphStructuralPairCandidateInputs::new(
            "node-z",
            "node-a",
            vec!["semantic_similar".to_string()],
        ),
        0.8,
        0.0,
        true,
    )
    .expect("tag-overlap pair helper should normalize");
    let batch = build_graph_structural_rerank_request_batch(&[row.clone()])
        .expect("tag-overlap batch should validate");

    assert_eq!(row.candidate_id, "pair:node-a:node-z");
    assert_eq!(row.anchor_planes, vec!["keyword", "tag"]);
    assert_eq!(row.anchor_values, vec!["alpha", "core"]);
    assert_eq!(row.keyword_score, 1.0);
    assert_eq!(row.tag_score, 1.0);
    assert_eq!(batch.num_rows(), 1);
}

#[test]
fn build_graph_structural_keyword_overlap_pair_rerank_request_row_from_metadata_composes() {
    let row = build_graph_structural_keyword_overlap_pair_rerank_request_row_from_metadata(
        GraphStructuralKeywordOverlapPairInputs::new(
            GraphStructuralKeywordTagQueryInputs::new(
                "query-9",
                0,
                2,
                vec!["alpha".to_string()],
                Vec::new(),
                vec!["semantic_similar".to_string()],
            ),
            GraphStructuralNodeMetadataInputs::new(vec!["alpha".to_string(), "core".to_string()]),
            GraphStructuralNodeMetadataInputs::new(vec!["graph".to_string(), "core".to_string()]),
            GraphStructuralPairCandidateInputs::new(
                "node-k",
                "node-a",
                vec!["semantic_similar".to_string()],
            ),
        ),
        0.9,
        0.1,
        true,
    )
    .expect("metadata-aware overlap helper should normalize");
    let batch = build_graph_structural_rerank_request_batch(&[row.clone()])
        .expect("metadata-aware overlap batch should validate");

    assert_eq!(row.query_id, "query-9");
    assert_eq!(row.candidate_id, "pair:node-a:node-k");
    assert_eq!(row.anchor_planes, vec!["keyword", "tag"]);
    assert_eq!(row.anchor_values, vec!["alpha", "core"]);
    assert_eq!(row.edge_constraint_kinds, vec!["semantic_similar"]);
    assert_eq!(row.keyword_score, 1.0);
    assert_eq!(row.tag_score, 1.0);
    assert_eq!(batch.num_rows(), 1);
}

#[test]
fn build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_metadata_composes() {
    let batch = build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_metadata(&[
        GraphStructuralKeywordOverlapPairRerankInputs::new(
            GraphStructuralKeywordOverlapPairInputs::new(
                GraphStructuralKeywordTagQueryInputs::new(
                    "query-10",
                    1,
                    2,
                    vec!["alpha".to_string()],
                    Vec::new(),
                    Vec::new(),
                ),
                GraphStructuralNodeMetadataInputs::new(vec![
                    "alpha".to_string(),
                    "core".to_string(),
                ]),
                GraphStructuralNodeMetadataInputs::new(vec![
                    "graph".to_string(),
                    "core".to_string(),
                ]),
                GraphStructuralPairCandidateInputs::new("node-r", "node-a", Vec::new()),
            ),
            0.5,
            0.0,
            true,
        ),
    ])
    .expect("metadata-aware batch helper should normalize");

    assert_eq!(batch.num_rows(), 1);
    assert_eq!(batch.num_columns(), 15);
}

#[test]
fn build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_inputs_composes() {
    let batch = build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_inputs(&[
        GraphStructuralKeywordOverlapPairRequestInputs::new(
            GraphStructuralKeywordTagQueryInputs::new(
                "query-11",
                0,
                1,
                vec!["alpha".to_string()],
                Vec::new(),
                Vec::new(),
            ),
            GraphStructuralNodeMetadataInputs::new(vec!["alpha".to_string(), "core".to_string()]),
            GraphStructuralNodeMetadataInputs::new(vec!["core".to_string(), "graph".to_string()]),
            GraphStructuralPairCandidateInputs::new("node-left", "node-right", Vec::new()),
            0.7,
            0.0,
            true,
        ),
    ])
    .expect("higher-level candidate input helper should normalize");

    assert_eq!(batch.num_rows(), 1);
    assert_eq!(batch.num_columns(), 15);
}

#[test]
fn build_graph_structural_keyword_overlap_pair_request_input_composes() {
    let request = build_graph_structural_keyword_overlap_pair_request_input(
        &build_graph_structural_keyword_overlap_query_inputs(
            "query-12",
            1,
            2,
            vec!["alpha".to_string()],
            vec!["semantic_similar".to_string()],
        ),
        GraphStructuralKeywordOverlapCandidateInputs::new(
            GraphStructuralNodeMetadataInputs::new(vec!["alpha".to_string(), "core".to_string()]),
            GraphStructuralNodeMetadataInputs::new(vec!["core".to_string()]),
            GraphStructuralPairCandidateInputs::new(
                "node-left",
                "node-right",
                vec!["semantic_similar".to_string()],
            ),
            0.6,
            0.2,
            true,
        ),
    );

    assert_eq!(request.metadata_inputs.query_inputs.query_id, "query-12");
    assert_eq!(
        request.metadata_inputs.query_inputs.keyword_anchors,
        vec!["alpha"]
    );
    assert_eq!(request.metadata_inputs.pair_inputs.left_id, "node-left");
    assert_eq!(request.metadata_inputs.pair_inputs.right_id, "node-right");
    assert_eq!(request.semantic_score, 0.6);
}

#[test]
fn build_graph_structural_keyword_overlap_query_inputs_composes() {
    let query = build_graph_structural_keyword_overlap_query_inputs(
        "query-12b",
        1,
        2,
        vec!["alpha".to_string()],
        vec!["semantic_similar".to_string()],
    );

    assert_eq!(
        query,
        GraphStructuralKeywordOverlapQueryInputs::new(
            "query-12b",
            1,
            2,
            vec!["alpha".to_string()],
            vec!["semantic_similar".to_string()],
        )
    );
}

#[test]
fn build_graph_structural_pair_candidate_inputs_composes() {
    let pair_inputs = build_graph_structural_pair_candidate_inputs(
        "node-left",
        "node-right",
        vec!["semantic_similar".to_string()],
    );

    assert_eq!(
        pair_inputs,
        GraphStructuralPairCandidateInputs::new(
            "node-left",
            "node-right",
            vec!["semantic_similar".to_string()],
        )
    );
}

#[test]
fn build_graph_structural_keyword_overlap_pair_candidate_inputs_composes() {
    let candidate = build_graph_structural_keyword_overlap_pair_candidate_inputs(
        build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(
            "node-left",
            "node-right",
            vec!["semantic_similar".to_string()],
            vec!["alpha".to_string(), "core".to_string()],
            vec!["core".to_string()],
        ),
        0.6,
        0.2,
        true,
    );

    assert_eq!(
        candidate,
        GraphStructuralKeywordOverlapCandidateInputs::new(
            GraphStructuralNodeMetadataInputs::new(vec!["alpha".to_string(), "core".to_string(),]),
            GraphStructuralNodeMetadataInputs::new(vec!["core".to_string()]),
            GraphStructuralPairCandidateInputs::new(
                "node-left",
                "node-right",
                vec!["semantic_similar".to_string()],
            ),
            0.6,
            0.2,
            true,
        )
    );
}

#[test]
fn build_graph_structural_keyword_overlap_pair_candidate_inputs_from_raw_composes() {
    let candidate = build_graph_structural_keyword_overlap_pair_candidate_inputs_from_raw(
        build_graph_structural_keyword_overlap_raw_candidate_inputs(
            build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(
                "node-left",
                "node-right",
                vec!["semantic_similar".to_string()],
                vec!["alpha".to_string(), "core".to_string()],
                vec!["core".to_string()],
            ),
            0.6,
            0.2,
            true,
        ),
    );

    assert_eq!(
        candidate,
        GraphStructuralKeywordOverlapCandidateInputs::new(
            GraphStructuralNodeMetadataInputs::new(vec!["alpha".to_string(), "core".to_string(),]),
            GraphStructuralNodeMetadataInputs::new(vec!["core".to_string()]),
            GraphStructuralPairCandidateInputs::new(
                "node-left",
                "node-right",
                vec!["semantic_similar".to_string()],
            ),
            0.6,
            0.2,
            true,
        )
    );
}

#[test]
fn build_graph_structural_keyword_overlap_raw_candidate_inputs_composes() {
    let candidate = build_graph_structural_keyword_overlap_raw_candidate_inputs(
        build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(
            "node-left",
            "node-right",
            vec!["semantic_similar".to_string()],
            vec!["alpha".to_string(), "core".to_string()],
            vec!["core".to_string()],
        ),
        0.6,
        0.2,
        true,
    );

    assert_eq!(
        candidate,
        GraphStructuralKeywordOverlapRawCandidateInputs::new(
            build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(
                "node-left",
                "node-right",
                vec!["semantic_similar".to_string()],
                vec!["alpha".to_string(), "core".to_string()],
                vec!["core".to_string()],
            ),
            0.6,
            0.2,
            true,
        )
    );
}

#[test]
fn graph_structural_keyword_overlap_candidate_inputs_new_composes() {
    let candidate = build_graph_structural_keyword_overlap_candidate_inputs(
        build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(
            "node-left",
            "node-right",
            vec!["semantic_similar".to_string()],
            vec!["alpha".to_string(), "core".to_string()],
            vec!["core".to_string()],
        ),
        0.6,
        0.2,
        true,
    );

    assert_eq!(
        candidate,
        GraphStructuralKeywordOverlapCandidateInputs::new(
            GraphStructuralNodeMetadataInputs::new(vec!["alpha".to_string(), "core".to_string(),]),
            GraphStructuralNodeMetadataInputs::new(vec!["core".to_string()]),
            GraphStructuralPairCandidateInputs::new(
                "node-left",
                "node-right",
                vec!["semantic_similar".to_string()],
            ),
            0.6,
            0.2,
            true,
        )
    );
}

#[test]
fn build_graph_structural_keyword_overlap_candidate_inputs_composes() {
    let candidate = build_graph_structural_keyword_overlap_candidate_inputs(
        build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(
            "node-left",
            "node-right",
            vec!["semantic_similar".to_string()],
            vec!["alpha".to_string(), "core".to_string()],
            vec!["core".to_string()],
        ),
        0.6,
        0.2,
        true,
    );

    assert_eq!(
        candidate,
        GraphStructuralKeywordOverlapCandidateInputs::new(
            GraphStructuralNodeMetadataInputs::new(vec!["alpha".to_string(), "core".to_string(),]),
            GraphStructuralNodeMetadataInputs::new(vec!["core".to_string()]),
            GraphStructuralPairCandidateInputs::new(
                "node-left",
                "node-right",
                vec!["semantic_similar".to_string()],
            ),
            0.6,
            0.2,
            true,
        )
    );
}

#[test]
fn build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_raw_candidates_composes() {
    let batch =
        build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_raw_candidates(
            &build_graph_structural_keyword_overlap_query_inputs(
                "query-13a",
                0,
                1,
                vec!["alpha".to_string()],
                Vec::new(),
            ),
            &[build_graph_structural_keyword_overlap_raw_candidate_inputs(
                build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(
                    "node-left",
                    "node-right",
                    Vec::new(),
                    vec!["alpha".to_string(), "core".to_string()],
                    vec!["core".to_string(), "graph".to_string()],
                ),
                0.7,
                0.0,
                true,
            )],
        )
        .expect("raw candidate batch helper should normalize");

    assert_eq!(batch.num_rows(), 1);
    assert_eq!(batch.num_columns(), 15);
}

#[test]
fn build_graph_structural_keyword_overlap_pair_rerank_request_batch_composes() {
    let batch = build_graph_structural_keyword_overlap_pair_rerank_request_batch(
        &build_graph_structural_keyword_overlap_query_inputs(
            "query-13",
            0,
            1,
            vec!["alpha".to_string()],
            Vec::new(),
        ),
        &[
            build_graph_structural_keyword_overlap_pair_candidate_inputs(
                build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(
                    "node-left",
                    "node-right",
                    Vec::new(),
                    vec!["alpha".to_string(), "core".to_string()],
                    vec!["core".to_string(), "graph".to_string()],
                ),
                0.7,
                0.0,
                true,
            ),
        ],
    )
    .expect("query-candidate batch helper should normalize");

    assert_eq!(batch.num_rows(), 1);
    assert_eq!(batch.num_columns(), 15);
}

#[test]
fn build_graph_structural_generic_topology_candidate_inputs_composes() {
    let candidate = build_graph_structural_generic_topology_candidate_inputs(
        build_graph_structural_generic_topology_candidate_metadata_inputs(
            "candidate-chain",
            vec![
                "node-1".to_string(),
                "node-2".to_string(),
                "node-3".to_string(),
            ],
            vec!["node-1".to_string(), "node-2".to_string()],
            vec!["node-2".to_string(), "node-3".to_string()],
            vec!["depends_on".to_string(), "depends_on".to_string()],
        ),
        0.6,
        0.3,
        0.2,
        0.1,
    );

    assert_eq!(
        candidate,
        GraphStructuralGenericTopologyCandidateInputs::new(
            GraphStructuralGenericTopologyCandidateMetadataInputs::new(
                "candidate-chain",
                vec![
                    "node-1".to_string(),
                    "node-2".to_string(),
                    "node-3".to_string(),
                ],
                vec!["node-1".to_string(), "node-2".to_string()],
                vec!["node-2".to_string(), "node-3".to_string()],
                vec!["depends_on".to_string(), "depends_on".to_string()],
            ),
            0.6,
            0.3,
            0.2,
            0.1,
        )
    );
}

#[test]
fn graph_structural_query_context_rejects_empty_anchor_list() {
    let error = GraphStructuralQueryContext::new("query-1", 0, 1, Vec::new(), Vec::new())
        .expect_err("query context should reject empty anchors");
    assert!(
        error
            .to_string()
            .contains("at least one query anchor is required"),
        "unexpected error: {error}"
    );
}

#[test]
fn graph_structural_candidate_subgraph_rejects_blank_node_ids() {
    let error = GraphStructuralCandidateSubgraph::new(
        "candidate-a",
        vec!["node-a".to_string(), "  ".to_string()],
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .expect_err("candidate should reject blank node ids");
    assert!(
        error
            .to_string()
            .contains("candidate node ids item 1 must not be blank"),
        "unexpected error: {error}"
    );
}

#[test]
fn graph_structural_rerank_signals_reject_negative_scores() {
    let error = GraphStructuralRerankSignals::new(-0.1, 0.0, 0.0, 0.0)
        .expect_err("signals should reject negative scores");
    assert!(
        error
            .to_string()
            .contains("semantic score must be non-negative"),
        "unexpected error: {error}"
    );
}

#[test]
fn graph_structural_pair_candidate_id_rejects_duplicate_endpoints() {
    let error = graph_structural_pair_candidate_id("same-node", "same-node")
        .expect_err("pair candidate id should reject duplicate endpoints");
    assert!(
        error
            .to_string()
            .contains("pair endpoints must not resolve to the same id"),
        "unexpected error: {error}"
    );
}
