//! Unit tests for topology module.

use std::collections::HashMap;
use std::sync::Arc;

use super::*;

fn make_test_node_with_path(title: &str, path: Vec<&str>, hash: Option<&str>) -> PageIndexNode {
    PageIndexNode {
        node_id: format!("doc#{}", title),
        parent_id: None,
        title: title.to_string(),
        level: path.len(),
        text: Arc::from("content"),
        summary: None,
        children: Vec::new(),
        blocks: Vec::new(),
        metadata: crate::link_graph::PageIndexMeta {
            line_range: (1, 10),
            byte_range: Some((0, 100)),
            structural_path: path.iter().map(|s| s.to_string()).collect(),
            content_hash: hash.map(|s| s.to_string()),
            attributes: HashMap::new(),
            token_count: 10,
            is_thinned: false,
            logbook: Vec::new(),
            observations: Vec::new(),
        },
    }
}

#[test]
fn test_empty_index() {
    let index = TopologyIndex::new();
    assert_eq!(index.total_entries(), 0);
    assert!(index.doc_ids().is_empty());
}

#[test]
fn test_build_from_trees() {
    let mut trees = HashMap::new();
    trees.insert(
        "doc.md".to_string(),
        vec![
            make_test_node_with_path("Intro", vec!["Intro"], Some("hash1")),
            make_test_node_with_path("Storage", vec!["Architecture", "Storage"], Some("hash2")),
        ],
    );

    let index = TopologyIndex::build_from_trees(&trees);

    assert_eq!(index.total_entries(), 2);
    assert_eq!(index.doc_ids().len(), 1);
}

#[test]
fn test_exact_path_match() {
    let mut trees = HashMap::new();
    trees.insert(
        "doc.md".to_string(),
        vec![make_test_node_with_path(
            "Storage",
            vec!["Architecture", "Storage"],
            None,
        )],
    );

    let index = TopologyIndex::build_from_trees(&trees);

    let entry = index
        .exact_path(
            "doc.md",
            &["Architecture".to_string(), "Storage".to_string()],
        )
        .expect("should find exact path");
    assert_eq!(entry.title, "Storage");

    // Wrong path
    assert!(
        index
            .exact_path(
                "doc.md",
                &["Architecture".to_string(), "Network".to_string()]
            )
            .is_none()
    );
}

#[test]
fn test_find_by_hash() {
    let mut trees = HashMap::new();
    trees.insert(
        "doc.md".to_string(),
        vec![make_test_node_with_path(
            "Section",
            vec!["Section"],
            Some("abc123"),
        )],
    );

    let index = TopologyIndex::build_from_trees(&trees);

    let entry = index.find_by_hash("abc123").expect("should find by hash");
    assert_eq!(entry.title, "Section");

    assert!(index.find_by_hash("notfound").is_none());
}

#[test]
fn test_fuzzy_resolve_exact_title() {
    let mut trees = HashMap::new();
    trees.insert(
        "doc.md".to_string(),
        vec![make_test_node_with_path("Storage", vec!["Storage"], None)],
    );

    let index = TopologyIndex::build_from_trees(&trees);

    let matches = index.fuzzy_resolve("storage", 5);
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].similarity_score, 1.0);
    assert_eq!(matches[0].match_type, MatchType::Exact);
}

#[test]
fn test_fuzzy_resolve_suffix() {
    let mut trees = HashMap::new();
    trees.insert(
        "doc.md".to_string(),
        vec![make_test_node_with_path(
            "Storage",
            vec!["Architecture", "Storage"],
            None,
        )],
    );

    let index = TopologyIndex::build_from_trees(&trees);

    let matches = index.fuzzy_resolve("architecture/storage", 5);
    assert!(!matches.is_empty());
    assert!(matches.iter().any(|m| m.match_type == MatchType::Suffix));
}

#[test]
fn test_fuzzy_resolve_substring() {
    let mut trees = HashMap::new();
    trees.insert(
        "doc.md".to_string(),
        vec![make_test_node_with_path(
            "Configuration Settings",
            vec!["Configuration Settings"],
            None,
        )],
    );

    let index = TopologyIndex::build_from_trees(&trees);

    let matches = index.fuzzy_resolve("config", 5);
    assert!(!matches.is_empty());
    assert!(
        matches
            .iter()
            .any(|m| m.match_type == MatchType::TitleSubstring)
    );
}

#[test]
fn test_case_insensitive_path() {
    let mut trees = HashMap::new();
    trees.insert(
        "doc.md".to_string(),
        vec![make_test_node_with_path(
            "Storage",
            vec!["Architecture", "Storage"],
            None,
        )],
    );

    let index = TopologyIndex::build_from_trees(&trees);

    let result = index
        .path_case_insensitive(
            "doc.md",
            &["architecture".to_string(), "storage".to_string()],
        )
        .expect("should find case-insensitive");
    assert_eq!(result.match_type, MatchType::CaseInsensitive);
    assert_eq!(result.similarity_score, 0.95);
}

#[test]
fn test_path_match_suffix_function() {
    assert!(path_match_suffix(
        &["architecture".to_string(), "storage".to_string()],
        "storage"
    ));

    assert!(path_match_suffix(
        &["architecture".to_string(), "storage".to_string()],
        "architecture/storage"
    ));

    assert!(!path_match_suffix(
        &["architecture".to_string(), "storage".to_string()],
        "network"
    ));

    assert!(!path_match_suffix(&["a".to_string()], "a/b/c"));
}
