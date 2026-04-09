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
use crate::graph::{GraphError, KnowledgeGraph, SkillDoc};
use crate::{Entity, EntityType, Relation, RelationType};
use serde_yaml::Value;
use tempfile::TempDir;

use super::{
    DEFAULT_GRAPH_VALKEY_KEY_PREFIX, graph_redis_client, normalize_graph_key_prefix,
    resolve_graph_key_prefix_with_settings_and_lookup,
    resolve_graph_valkey_url_with_settings_and_lookup,
};

fn has_valkey() -> bool {
    [
        "XIUXIAN_WENDAO_GRAPH_VALKEY_URL",
        "VALKEY_URL",
        "XIUXIAN_WENDAO_KNOWLEDGE_VALKEY_URL",
    ]
    .into_iter()
    .any(|name| {
        std::env::var(name)
            .ok()
            .is_some_and(|value| !value.trim().is_empty())
    })
}

fn settings_from_yaml(yaml: &str) -> Value {
    serde_yaml::from_str(yaml).unwrap_or_else(|error| panic!("settings yaml: {error}"))
}

#[tokio::test]
async fn test_save_and_load_valkey_roundtrip() {
    if !has_valkey() {
        return;
    }
    let temp_dir = TempDir::new().unwrap();
    let scope_key = temp_dir
        .path()
        .join("knowledge")
        .to_string_lossy()
        .into_owned();

    // Build graph
    let graph = KnowledgeGraph::new();

    let mut entity1 = Entity::new(
        "tool:python".to_string(),
        "Python".to_string(),
        EntityType::Skill,
        "Programming language".to_string(),
    );
    entity1.aliases = vec!["py".to_string(), "python3".to_string()];
    entity1.confidence = 0.95;

    let mut entity2 = Entity::new(
        "tool:claude-code".to_string(),
        "Claude Code".to_string(),
        EntityType::Tool,
        "AI coding assistant".to_string(),
    );
    entity2.vector = Some(vec![0.1; 128]);

    graph.add_entity(entity1).unwrap();
    graph.add_entity(entity2).unwrap();

    let relation = Relation::new(
        "Claude Code".to_string(),
        "Python".to_string(),
        RelationType::Uses,
        "Claude Code uses Python".to_string(),
    )
    .with_confidence(0.8);
    graph.add_relation(relation).unwrap();

    graph.save_to_valkey(&scope_key, 128).unwrap();

    // Load into new graph
    let mut graph2 = KnowledgeGraph::new();
    graph2.load_from_valkey(&scope_key).unwrap();

    // Verify entity counts
    let stats = graph2.get_stats();
    assert_eq!(stats.total_entities, 2, "Should have 2 entities");
    assert_eq!(stats.total_relations, 1, "Should have 1 relation");

    // Verify entity data
    let python = graph2.get_entity_by_name("Python").unwrap();
    assert_eq!(python.aliases.len(), 2);
    assert!(python.aliases.contains(&"py".to_string()));
    assert_eq!(python.confidence, 0.95);
    assert!(
        python.vector.is_none(),
        "Python entity should have no vector"
    );

    let claude = graph2.get_entity_by_name("Claude Code").unwrap();
    assert!(
        claude.vector.is_some(),
        "Claude entity should have a vector"
    );
    assert_eq!(claude.vector.as_ref().unwrap().len(), 128);

    // Verify relation data
    let rels = graph2.get_relations(None, None);
    assert_eq!(rels.len(), 1);
    assert_eq!(rels[0].source, "Claude Code");
    assert_eq!(rels[0].target, "Python");
    assert_eq!(rels[0].confidence, 0.8);
}

#[tokio::test]
async fn test_valkey_persistence_with_skill_registration() {
    if !has_valkey() {
        return;
    }
    let temp_dir = TempDir::new().unwrap();
    let scope_key = temp_dir
        .path()
        .join("knowledge")
        .to_string_lossy()
        .into_owned();

    let graph = KnowledgeGraph::new();

    let docs = vec![
        SkillDoc {
            id: "git".to_string(),
            doc_type: "skill".to_string(),
            skill_name: "git".to_string(),
            tool_name: String::new(),
            content: "Git operations".to_string(),
            routing_keywords: vec![],
        },
        SkillDoc {
            id: "git.commit".to_string(),
            doc_type: "command".to_string(),
            skill_name: "git".to_string(),
            tool_name: "git.commit".to_string(),
            content: "Create a commit".to_string(),
            routing_keywords: vec!["commit".to_string(), "git".to_string()],
        },
    ];
    graph.register_skill_entities(&docs).unwrap();

    let stats_before = graph.get_stats();

    graph.save_to_valkey(&scope_key, 1024).unwrap();

    let mut graph2 = KnowledgeGraph::new();
    graph2.load_from_valkey(&scope_key).unwrap();

    let stats_after = graph2.get_stats();
    assert_eq!(stats_before.total_entities, stats_after.total_entities);
    assert_eq!(stats_before.total_relations, stats_after.total_relations);

    // Verify search still works after roundtrip
    let results = graph2.search_entities("git", 10);
    assert!(
        !results.is_empty(),
        "Search should find git entities after Valkey roundtrip"
    );
}

#[test]
fn normalize_graph_key_prefix_falls_back_for_blank_input() {
    assert_eq!(
        normalize_graph_key_prefix("   "),
        DEFAULT_GRAPH_VALKEY_KEY_PREFIX.to_string()
    );
}

#[test]
fn normalize_graph_key_prefix_trims_non_blank_input() {
    assert_eq!(
        normalize_graph_key_prefix("  xiuxian:graph:test  "),
        "xiuxian:graph:test".to_string()
    );
}

#[test]
fn graph_redis_client_opens_trimmed_valid_url() {
    let client = graph_redis_client(" redis://127.0.0.1/ ");
    assert!(client.is_ok());
}

#[test]
fn graph_redis_client_preserves_graph_error_identity() {
    let Err(error) = graph_redis_client("  ") else {
        panic!("blank URL should fail");
    };
    assert!(matches!(
        error,
        GraphError::InvalidRelation(ref field, _) if field == "graph_valkey_client"
    ));
}

#[test]
fn graph_valkey_resolution_prefers_toml_values() {
    let settings = settings_from_yaml(
        r#"
graph:
  persistence:
    valkey_url: "redis://127.0.0.1:6380/0"
    key_prefix: "xiuxian:test:graph"
"#,
    );

    let url = resolve_graph_valkey_url_with_settings_and_lookup(&settings, &|_| {
        Some("redis://127.0.0.1:6379/0".to_string())
    })
    .unwrap_or_else(|error| panic!("graph valkey url should resolve: {error:?}"));
    let key_prefix = resolve_graph_key_prefix_with_settings_and_lookup(&settings, &|_| None);

    assert_eq!(url, "redis://127.0.0.1:6380/0");
    assert_eq!(key_prefix, "xiuxian:test:graph");
}

#[test]
fn graph_valkey_resolution_falls_back_to_env_when_toml_is_missing() {
    let url = resolve_graph_valkey_url_with_settings_and_lookup(&Value::Null, &|name| match name {
        "XIUXIAN_WENDAO_GRAPH_VALKEY_URL" => Some("redis://127.0.0.1:6379/5".to_string()),
        _ => None,
    })
    .unwrap_or_else(|error| panic!("graph env fallback should resolve: {error:?}"));

    assert_eq!(url, "redis://127.0.0.1:6379/5");
}

#[test]
fn graph_valkey_resolution_keeps_invalid_toml_authoritative() {
    let settings = settings_from_yaml(
        r#"
graph:
  persistence:
    valkey_url: " definitely-not-a-redis-url "
"#,
    );

    let url = resolve_graph_valkey_url_with_settings_and_lookup(&settings, &|name| match name {
        "XIUXIAN_WENDAO_GRAPH_VALKEY_URL" => Some("redis://127.0.0.1:6379/5".to_string()),
        _ => None,
    })
    .unwrap_or_else(|error| panic!("graph valkey url should still resolve raw TOML: {error:?}"));

    let error = graph_redis_client(url.as_str())
        .err()
        .unwrap_or_else(|| panic!("invalid TOML url should stay authoritative"));

    assert!(matches!(
        error,
        GraphError::InvalidRelation(ref field, _) if field == "graph_valkey_client"
    ));
}
