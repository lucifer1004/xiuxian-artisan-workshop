#![allow(
    missing_docs,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::doc_markdown,
    clippy::implicit_clone,
    clippy::uninlined_format_args,
    clippy::float_cmp,
    clippy::cast_lossless,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation,
    clippy::manual_string_new,
    clippy::needless_raw_string_hashes,
    clippy::format_push_string,
    clippy::map_unwrap_or,
    clippy::unnecessary_to_owned,
    clippy::too_many_lines
)]
use super::*;

#[test]
fn test_compute_link_graph_saliency_activation_boosts_score() {
    let policy = LinkGraphSaliencyPolicy::default();
    let without_activation = compute_link_graph_saliency(5.0, 0.02, 0, 2.0, policy);
    let with_activation = compute_link_graph_saliency(5.0, 0.02, 8, 2.0, policy);
    assert!(with_activation > without_activation);
}
