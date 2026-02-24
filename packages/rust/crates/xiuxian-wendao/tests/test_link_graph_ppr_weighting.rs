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
//! Weighted-seed PPR behavior checks.

use std::collections::HashMap;
use xiuxian_wendao::LinkGraphIndex;

#[tokio::test]
async fn test_ppr_non_uniform_seed_bias() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let root = temp.path();

    std::fs::write(root.join("A.md"), "A links to [[B]]").expect("A should be written");
    std::fs::write(root.join("B.md"), "B node").expect("B should be written");
    std::fs::write(root.join("C.md"), "C links to [[D]]").expect("C should be written");
    std::fs::write(root.join("D.md"), "D node").expect("D should be written");

    let index = LinkGraphIndex::build(root).expect("index should build");

    let mut seeds = HashMap::new();
    seeds.insert("A".to_string(), 0.9);
    seeds.insert("C".to_string(), 0.1);

    let (related, _) = index.related_from_weighted_seeds_with_diagnostics(&seeds, 2, 10, None);
    let stems: Vec<String> = related.iter().map(|node| node.stem.clone()).collect();

    let pos_b = stems.iter().position(|stem| stem == "B");
    let pos_d = stems.iter().position(|stem| stem == "D");
    match (pos_b, pos_d) {
        (Some(b), Some(d)) => assert!(
            b < d,
            "expected B to outrank D under non-uniform seeds, got stems: {stems:?}"
        ),
        _ => panic!("expected both B and D in results, got stems: {stems:?}"),
    }
}
