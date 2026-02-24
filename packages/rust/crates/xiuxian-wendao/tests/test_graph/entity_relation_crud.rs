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
fn test_add_entity() {
    let graph = KnowledgeGraph::new();

    let entity = Entity::new(
        "person:john_doe".to_string(),
        "John Doe".to_string(),
        EntityType::Person,
        "A developer".to_string(),
    );

    assert!(graph.add_entity(entity).is_ok());
    assert_eq!(graph.get_stats().total_entities, 1);
}

#[test]
fn test_add_relation() {
    let graph = KnowledgeGraph::new();

    let entity1 = Entity::new(
        "person:john_doe".to_string(),
        "John Doe".to_string(),
        EntityType::Person,
        "A developer".to_string(),
    );
    let entity2 = Entity::new(
        "organization:acme".to_string(),
        "Acme Corp".to_string(),
        EntityType::Organization,
        "A company".to_string(),
    );

    graph.add_entity(entity1).unwrap();
    graph.add_entity(entity2).unwrap();

    let relation = Relation::new(
        "John Doe".to_string(),
        "Acme Corp".to_string(),
        RelationType::WorksFor,
        "Works at the company".to_string(),
    );

    assert!(graph.add_relation(relation).is_ok());
    assert_eq!(graph.get_stats().total_relations, 1);
}
