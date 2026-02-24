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
//! Topology regression tests for mixed outbound link structures.

use std::collections::HashMap;
use xiuxian_wendao::LinkGraphIndex;

#[tokio::test]
async fn test_mixed_graph_topology_related_from_weighted_seed() {
    let temp = tempfile::tempdir().expect("tempdir should be created");
    let root = temp.path();

    std::fs::write(
        root.join("note.md"),
        r#"
# Section 1
This talks about [[EntityA]].

# Section 2
This links to [[EntityB]].
"#,
    )
    .expect("note should be written");
    std::fs::write(root.join("EntityA.md"), "Entity A canonical node.")
        .expect("EntityA should be written");
    std::fs::write(root.join("EntityB.md"), "Entity B canonical node.")
        .expect("EntityB should be written");

    let index = LinkGraphIndex::build(root).expect("index should build");

    let mut seeds = HashMap::new();
    seeds.insert("note".to_string(), 1.0);
    let (related, _) = index.related_from_weighted_seeds_with_diagnostics(&seeds, 1, 10, None);

    let stems: Vec<String> = related.iter().map(|n| n.stem.clone()).collect();
    assert!(
        stems.contains(&"EntityA".to_string()) && stems.contains(&"EntityB".to_string()),
        "seed note should expose both linked entities, got: {stems:?}"
    );
}
