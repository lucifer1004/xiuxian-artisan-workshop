//! `SkillDefinition` metadata and routing keyword tests.

use std::error::Error;

use serde_json::json;
use xiuxian_types::SkillDefinition;

fn metadata_object(
    value: &serde_json::Value,
) -> Result<&serde_json::Map<String, serde_json::Value>, std::io::Error> {
    value
        .as_object()
        .ok_or_else(|| std::io::Error::other("metadata should be an object"))
}

fn routing_keywords_array(
    metadata: &serde_json::Map<String, serde_json::Value>,
) -> Result<&Vec<serde_json::Value>, std::io::Error> {
    metadata
        .get("routing_keywords")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| std::io::Error::other("routing_keywords should be an array"))
}

#[test]
fn skill_definition_defaults_metadata() -> Result<(), Box<dyn Error>> {
    let value = json!({
        "name": "git",
        "description": "desc"
    });
    let def: SkillDefinition = serde_json::from_value(value)?;
    assert!(def.metadata.is_object());
    assert!(metadata_object(&def.metadata)?.is_empty());
    assert!(def.routing_keywords.is_empty());
    Ok(())
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
fn routing_keywords_merge_metadata_and_explicit() -> Result<(), Box<dyn Error>> {
    let value = json!({
        "name": "git",
        "description": "desc",
        "metadata": {
            "routing_keywords": ["alpha"],
            "routingKeywords": ["beta"]
        },
        "routing_keywords": ["beta", "gamma", " "]
    });
    let def: SkillDefinition = serde_json::from_value(value)?;
    assert_eq!(
        def.routing_keywords,
        vec!["alpha".to_string(), "beta".to_string(), "gamma".to_string()]
    );
    Ok(())
}

#[test]
fn serialization_injects_routing_keywords_into_metadata() -> Result<(), Box<dyn Error>> {
    let def = SkillDefinition {
        name: "git".to_string(),
        description: "desc".to_string(),
        metadata: json!({}),
        routing_keywords: vec!["alpha".to_string()],
    };
    let value = serde_json::to_value(&def)?;
    assert!(value.get("routing_keywords").is_none());
    let metadata = metadata_object(
        value
            .get("metadata")
            .ok_or_else(|| std::io::Error::other("serialized metadata field missing"))?,
    )?;
    let routing = routing_keywords_array(metadata)?;
    let keywords: Vec<&str> = routing.iter().filter_map(|value| value.as_str()).collect();
    assert_eq!(keywords, vec!["alpha"]);
    Ok(())
}

#[test]
fn serialization_merges_routing_keywords_variants() -> Result<(), Box<dyn Error>> {
    let def = SkillDefinition {
        name: "git".to_string(),
        description: "desc".to_string(),
        metadata: json!({
            "routing_keywords": ["alpha"],
            "routingKeywords": ["beta"]
        }),
        routing_keywords: vec!["gamma".to_string()],
    };
    let value = serde_json::to_value(&def)?;
    let metadata = metadata_object(
        value
            .get("metadata")
            .ok_or_else(|| std::io::Error::other("serialized metadata field missing"))?,
    )?;
    let routing = routing_keywords_array(metadata)?;
    let keywords: Vec<&str> = routing.iter().filter_map(|value| value.as_str()).collect();
    assert_eq!(keywords, vec!["alpha", "beta", "gamma"]);
    assert!(metadata.get("routingKeywords").is_none());
    Ok(())
}
