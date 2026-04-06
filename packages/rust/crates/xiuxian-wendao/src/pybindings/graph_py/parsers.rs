use crate::entity::{EntityType, RelationType};
use std::str::FromStr;

/// Parse entity type from string.
pub(crate) fn parse_entity_type(s: &str) -> EntityType {
    match s.to_uppercase().as_str() {
        "PERSON" => EntityType::Person,
        "ORGANIZATION" => EntityType::Organization,
        "CONCEPT" => EntityType::Concept,
        "PROJECT" => EntityType::Project,
        "TOOL" => EntityType::Tool,
        "SKILL" => EntityType::Skill,
        "LOCATION" => EntityType::Location,
        "EVENT" => EntityType::Event,
        "DOCUMENT" => EntityType::Document,
        "CODE" => EntityType::Code,
        "API" => EntityType::Api,
        "ERROR" => EntityType::Error,
        "PATTERN" => EntityType::Pattern,
        _ => EntityType::Other(s.to_string()),
    }
}

/// Parse a persisted relation token from string.
pub(crate) fn parse_persisted_relation_type(s: &str) -> RelationType {
    match RelationType::from_str(s.trim()) {
        Ok(RelationType::Other(_)) => RelationType::Other(s.trim().to_string()),
        Ok(relation_type) => relation_type,
        Err(never) => match never {},
    }
}
