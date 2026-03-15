//! SkillDefinition metadata and routing keyword tests.

use serde_json::json;
use xiuxian_types::SkillDefinition;
#[test]
fn skill_definition_defaults_metadata() {
    let value = json!({
        "name": "git",
        "description": "desc"
    });
    let def: SkillDefinition = serde_json::from_value(value).expect("deserialize skill definition");
    assert!(def.metadata.is_object());
    assert!(
        def.metadata
            .as_object()
            .expect("metadata object")
            .is_empty()
    );
    assert!(def.routing_keywords.is_empty());
}

#[test]
fn require_refs_variants_normalize() {
    let metadata = json!({
        "requireRefs": [" core ", "", "core", "extra"]
    });
    let def = SkillDefinition::new("git".to_string(), "desc".to_string(), metadata);
    assert_eq!(def.get_require_refs(), vec!["core", "extra"]);
}

#[test]
fn routing_keywords_merge_metadata_and_explicit() {
    let value = json!({
        "name": "git",
        "description": "desc",
        "metadata": {
            "routing_keywords": ["alpha"],
            "routingKeywords": ["beta"]
        },
        "routing_keywords": ["beta", "gamma", " "]
    });
    let def: SkillDefinition = serde_json::from_value(value).expect("deserialize skill definition");
    assert_eq!(
        def.routing_keywords,
        vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()]
    );
}

#[test]
fn serialization_injects_routing_keywords_into_metadata() {
    let def = SkillDefinition {
        name: "git".to_string(),
        description: "desc".to_string(),
        metadata: json!({}),
        routing_keywords: vec!["alpha".to_string()],
    };
    let value = serde_json::to_value(&def).expect("serialize skill definition");
    assert!(value.get("routing_keywords").is_none());
    let metadata = value
        .get("metadata")
        .and_then(|meta| meta.as_object())
        .expect("metadata object");
    let routing = metadata
        .get("routing_keywords")
        .and_then(|value| value.as_array())
        .expect("routing keywords array");
    let keywords: Vec<&str> = routing.iter().filter_map(|value| value.as_str()).collect();
    assert_eq!(keywords, vec!["alpha"]);
}

#[test]
fn serialization_merges_routing_keywords_variants() {
    let def = SkillDefinition {
        name: "git".to_string(),
        description: "desc".to_string(),
        metadata: json!({
            "routing_keywords": ["alpha"],
            "routingKeywords": ["beta"]
        }),
        routing_keywords: vec!["gamma".to_string()],
    };
    let value = serde_json::to_value(&def).expect("serialize skill definition");
    let metadata = value
        .get("metadata")
        .and_then(|meta| meta.as_object())
        .expect("metadata object");
    let routing = metadata
        .get("routing_keywords")
        .and_then(|value| value.as_array())
        .expect("routing keywords array");
    let keywords: Vec<&str> = routing.iter().filter_map(|value| value.as_str()).collect();
    assert_eq!(keywords, vec!["alpha", "beta", "gamma"]);
    assert!(metadata.get("routingKeywords").is_none());
}
