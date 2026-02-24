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
fn test_search_entities_exact_name_ranks_highest() {
    let graph = KnowledgeGraph::new();

    let entities = vec![
        ("git.commit", EntityType::Tool, "Create a git commit"),
        ("git.status", EntityType::Tool, "Show git status"),
        ("knowledge.recall", EntityType::Tool, "Recall knowledge"),
    ];

    for (name, etype, desc) in &entities {
        let entity = Entity::new(
            format!("tool:{}", name),
            name.to_string(),
            etype.clone(),
            desc.to_string(),
        );
        graph.add_entity(entity).unwrap();
    }

    let results = graph.search_entities("git.commit", 10);
    assert!(!results.is_empty());
    assert_eq!(results[0].name, "git.commit", "Exact match should be first");
}

#[test]
fn test_search_entities_alias_match() {
    let graph = KnowledgeGraph::new();

    let mut entity = Entity::new(
        "tool:claude_code".to_string(),
        "Claude Code".to_string(),
        EntityType::Tool,
        "AI coding assistant".to_string(),
    );
    entity.aliases = vec!["claude-dev".to_string(), "cc".to_string()];
    graph.add_entity(entity).unwrap();

    let other = Entity::new(
        "concept:devtools".to_string(),
        "Developer Tools".to_string(),
        EntityType::Concept,
        "Development tools and utilities".to_string(),
    );
    graph.add_entity(other).unwrap();

    // Search by alias
    let results = graph.search_entities("claude-dev", 10);
    assert!(!results.is_empty());
    assert_eq!(
        results[0].name, "Claude Code",
        "Alias exact match should find Claude Code"
    );

    // Short alias
    let results = graph.search_entities("cc", 10);
    assert!(!results.is_empty());
    assert_eq!(results[0].name, "Claude Code");
}

#[test]
fn test_search_entities_token_overlap() {
    let graph = KnowledgeGraph::new();

    let entities = vec![
        ("git.smart_commit", EntityType::Tool, "Create smart commits"),
        ("git.status", EntityType::Tool, "Show git status"),
        ("knowledge.code_search", EntityType::Tool, "Search code"),
    ];

    for (name, etype, desc) in &entities {
        let entity = Entity::new(
            format!("tool:{}", name),
            name.to_string(),
            etype.clone(),
            desc.to_string(),
        );
        graph.add_entity(entity).unwrap();
    }

    // "smart commit" should match "git.smart_commit" via token overlap
    let results = graph.search_entities("smart commit", 10);
    assert!(!results.is_empty());
    assert_eq!(
        results[0].name, "git.smart_commit",
        "Token overlap should match 'smart commit' to 'git.smart_commit'"
    );
}

#[test]
fn test_search_entities_fuzzy_match() {
    let graph = KnowledgeGraph::new();

    let entity = Entity::new(
        "concept:zettelkasten".to_string(),
        "zettelkasten".to_string(),
        EntityType::Concept,
        "Note-taking method".to_string(),
    );
    graph.add_entity(entity).unwrap();

    // Typo: "zettelkastn" should still find "zettelkasten" via fuzzy match
    let results = graph.search_entities("zettelkastn", 10);
    assert!(
        !results.is_empty(),
        "Fuzzy match should find 'zettelkasten' when querying 'zettelkastn'"
    );
    assert_eq!(results[0].name, "zettelkasten");
}

#[test]
fn test_search_entities_description_fallback() {
    let graph = KnowledgeGraph::new();

    let entity = Entity::new(
        "tool:research_web".to_string(),
        "researcher.search".to_string(),
        EntityType::Tool,
        "Search the internet for information about any topic".to_string(),
    );
    graph.add_entity(entity).unwrap();

    // "internet" doesn't appear in name, aliases, or tokens — only description
    let results = graph.search_entities("internet", 10);
    assert!(!results.is_empty());
    assert_eq!(results[0].name, "researcher.search");
}

#[test]
fn test_search_entities_empty_query() {
    let graph = KnowledgeGraph::new();
    let entity = Entity::new(
        "tool:git".to_string(),
        "git".to_string(),
        EntityType::Tool,
        "Git".to_string(),
    );
    graph.add_entity(entity).unwrap();

    let results = graph.search_entities("", 10);
    assert!(results.is_empty(), "Empty query should return no results");
}

#[test]
fn test_search_entities_confidence_boost() {
    let graph = KnowledgeGraph::new();

    let mut high_conf = Entity::new(
        "tool:primary".to_string(),
        "primary_tool".to_string(),
        EntityType::Tool,
        "A primary tool for search".to_string(),
    );
    high_conf.confidence = 1.0;

    let mut low_conf = Entity::new(
        "tool:secondary".to_string(),
        "secondary_tool".to_string(),
        EntityType::Tool,
        "A secondary tool for search".to_string(),
    );
    low_conf.confidence = 0.3;

    graph.add_entity(high_conf).unwrap();
    graph.add_entity(low_conf).unwrap();

    // Both match via description ("search"), but high confidence should rank first
    let results = graph.search_entities("search", 10);
    assert!(results.len() >= 2);
    // High-confidence entity should have higher final score
    let names: Vec<String> = results.iter().map(|e| e.name.clone()).collect();
    assert_eq!(
        names[0], "primary_tool",
        "Higher confidence entity should rank first"
    );
}
