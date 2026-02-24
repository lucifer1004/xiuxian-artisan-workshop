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
fn test_entity_from_dict() {
    let data = serde_json::json!({
        "name": "Claude Code",
        "entity_type": "TOOL",
        "description": "AI coding assistant",
        "source": "docs/tools.md",
        "aliases": ["claude", "claude-dev"],
        "confidence": 0.95
    });

    let entity = entity_from_dict(&data).unwrap();
    assert_eq!(entity.name, "Claude Code");
    assert!(matches!(entity.entity_type, EntityType::Tool));
    assert_eq!(entity.aliases.len(), 2);
}

#[test]
fn test_save_and_load_graph() {
    let temp_dir = TempDir::new().unwrap();
    let graph_path = temp_dir.path().join("test_graph.json");

    {
        let graph = KnowledgeGraph::new();

        let entity1 = Entity::new(
            "tool:python".to_string(),
            "Python".to_string(),
            EntityType::Skill,
            "Programming language".to_string(),
        );
        let entity2 = Entity::new(
            "tool:claude-code".to_string(),
            "Claude Code".to_string(),
            EntityType::Tool,
            "AI coding assistant".to_string(),
        );

        graph.add_entity(entity1).unwrap();
        graph.add_entity(entity2).unwrap();

        let relation = Relation::new(
            "Claude Code".to_string(),
            "Python".to_string(),
            RelationType::Uses,
            "Claude Code uses Python".to_string(),
        );
        graph.add_relation(relation).unwrap();
        graph.save_to_file(graph_path.to_str().unwrap()).unwrap();
    }

    {
        let mut graph = KnowledgeGraph::new();
        graph.load_from_file(graph_path.to_str().unwrap()).unwrap();

        let stats = graph.get_stats();
        assert_eq!(stats.total_entities, 2);
        assert_eq!(stats.total_relations, 1);

        let python = graph.get_entity_by_name("Python");
        assert!(python.is_some());
        assert_eq!(python.unwrap().entity_type, EntityType::Skill);

        let relations = graph.get_relations(None, None);
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].source, "Claude Code");
    }
}

#[test]
fn test_export_as_json() {
    let graph = KnowledgeGraph::new();

    let entity = Entity::new(
        "project:omni".to_string(),
        "Omni Dev Fusion".to_string(),
        EntityType::Project,
        "Development environment".to_string(),
    );

    graph.add_entity(entity).unwrap();

    let json = graph.export_as_json().unwrap();
    assert!(json.contains("Omni Dev Fusion"));
    assert!(json.contains("entities"));
    assert!(json.contains("relations"));
}

#[test]
fn test_export_import_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let graph_path = temp_dir.path().join("roundtrip.json");

    let graph1 = KnowledgeGraph::new();

    let entities = vec![
        ("Python", EntityType::Skill),
        ("Rust", EntityType::Skill),
        ("Claude Code", EntityType::Tool),
        ("Omni Dev Fusion", EntityType::Project),
    ];

    for (name, etype) in &entities {
        let entity = Entity::new(
            format!(
                "{}:{}",
                etype.to_string().to_lowercase(),
                name.to_lowercase().replace(' ', "_")
            ),
            name.to_string(),
            etype.clone(),
            format!("Description of {}", name),
        );
        graph1.add_entity(entity).unwrap();
    }

    let relations = vec![
        ("Claude Code", "Python", RelationType::Uses),
        ("Claude Code", "Rust", RelationType::Uses),
        ("Omni Dev Fusion", "Claude Code", RelationType::CreatedBy),
    ];

    for (source, target, rtype) in &relations {
        let relation = Relation::new(
            source.to_string(),
            target.to_string(),
            rtype.clone(),
            format!("{} -> {}", source, target),
        );
        graph1.add_relation(relation).unwrap();
    }

    graph1.save_to_file(graph_path.to_str().unwrap()).unwrap();

    let mut graph2 = KnowledgeGraph::new();
    graph2.load_from_file(graph_path.to_str().unwrap()).unwrap();

    let stats1 = graph1.get_stats();
    let stats2 = graph2.get_stats();
    assert_eq!(stats1.total_entities, stats2.total_entities);
    assert_eq!(stats1.total_relations, stats2.total_relations);
}
