use super::{
    GRAPH_NEIGHBORS_DEFAULT_HOPS, GRAPH_NEIGHBORS_DEFAULT_LIMIT, validate_graph_neighbors_request,
};

#[test]
fn graph_neighbors_request_validation_accepts_canonical_request() {
    assert_eq!(
        validate_graph_neighbors_request(
            "kernel/docs/index.md",
            Some("outgoing"),
            Some(3),
            Some(25)
        ),
        Ok((
            "kernel/docs/index.md".to_string(),
            "outgoing".to_string(),
            3,
            25,
        ))
    );
}

#[test]
fn graph_neighbors_request_validation_normalizes_defaults_and_clamps_bounds() {
    assert_eq!(
        validate_graph_neighbors_request(
            "kernel/docs/index.md",
            Some("invalid"),
            Some(0),
            Some(999)
        ),
        Ok((
            "kernel/docs/index.md".to_string(),
            "both".to_string(),
            1,
            300,
        ))
    );
    assert_eq!(
        validate_graph_neighbors_request("kernel/docs/index.md", None, None, None),
        Ok((
            "kernel/docs/index.md".to_string(),
            "both".to_string(),
            GRAPH_NEIGHBORS_DEFAULT_HOPS,
            GRAPH_NEIGHBORS_DEFAULT_LIMIT,
        ))
    );
}

#[test]
fn graph_neighbors_request_validation_rejects_blank_node_id() {
    assert_eq!(
        validate_graph_neighbors_request("   ", Some("both"), Some(2), Some(20)),
        Err("graph neighbors requires a non-empty node id".to_string())
    );
}
