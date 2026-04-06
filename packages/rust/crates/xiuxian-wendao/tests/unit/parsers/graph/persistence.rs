use crate::parsers::graph::persistence::{entity_from_dict, relation_from_dict};
use crate::{EntityType, RelationType};

#[test]
fn entity_from_dict_parses_entity_metadata() {
    let data = serde_json::json!({
        "name": "Claude Code",
        "entity_type": "TOOL",
        "description": "AI coding assistant",
        "source": "docs/tools.md",
        "aliases": ["claude", "claude-dev"],
        "confidence": 0.95
    });

    let Some(entity) = entity_from_dict(&data) else {
        panic!("entity_from_dict should return an entity");
    };

    assert_eq!(entity.name, "Claude Code");
    assert!(matches!(entity.entity_type, EntityType::Tool));
    assert_eq!(entity.aliases.len(), 2);
    assert_eq!(entity.source.as_deref(), Some("docs/tools.md"));
}

#[test]
fn relation_from_dict_parses_persisted_relation_token() {
    let data = serde_json::json!({
        "source": "Skill",
        "target": "logo.png",
        "relation_type": "ATTACHED_TO",
        "description": "Skill attaches logo.png",
        "source_doc": "skills/demo/SKILL.md",
        "confidence": 0.8
    });

    let Some(relation) = relation_from_dict(&data) else {
        panic!("relation_from_dict should return a relation");
    };

    assert_eq!(relation.source, "Skill");
    assert_eq!(relation.target, "logo.png");
    assert!(matches!(relation.relation_type, RelationType::AttachedTo));
    assert_eq!(relation.source_doc.as_deref(), Some("skills/demo/SKILL.md"));
}

#[test]
fn relation_from_dict_preserves_note_wikilink_label_as_other() {
    let data = serde_json::json!({
        "source": "Claude Code",
        "target": "design",
        "relation_type": "[[notes/design]]",
        "description": "Claude Code links to design"
    });

    let Some(relation) = relation_from_dict(&data) else {
        panic!("relation_from_dict should return a relation");
    };

    assert!(matches!(
        relation.relation_type,
        RelationType::Other(ref value) if value == "[[notes/design]]"
    ));
}

#[test]
fn relation_from_dict_preserves_attachment_wikilink_label_as_other() {
    let data = serde_json::json!({
        "source": "Skill",
        "target": "logo.png",
        "relation_type": "[[assets/logo.png]]",
        "description": "Skill links to logo.png"
    });

    let Some(relation) = relation_from_dict(&data) else {
        panic!("relation_from_dict should return a relation");
    };

    assert!(matches!(
        relation.relation_type,
        RelationType::Other(ref value) if value == "[[assets/logo.png]]"
    ));
}

#[test]
fn relation_from_dict_preserves_arbitrary_wikilink_label() {
    let data = serde_json::json!({
        "source": "Skill",
        "target": "Custom",
        "relation_type": "[[projects/custom-edge]]",
        "description": "Custom graph edge"
    });

    let Some(relation) = relation_from_dict(&data) else {
        panic!("relation_from_dict should return a relation");
    };

    assert!(matches!(
        relation.relation_type,
        RelationType::Other(ref value) if value == "[[projects/custom-edge]]"
    ));
}

#[test]
fn entity_from_dict_falls_back_to_other_type_for_unknown_values() {
    let data = serde_json::json!({
        "name": "Mystery",
        "entity_type": "CUSTOM_ENTITY",
        "description": "Unknown domain type"
    });

    let Some(entity) = entity_from_dict(&data) else {
        panic!("entity_from_dict should return an entity");
    };

    assert!(matches!(
        entity.entity_type,
        EntityType::Other(ref value) if value == "CUSTOM_ENTITY"
    ));
}
