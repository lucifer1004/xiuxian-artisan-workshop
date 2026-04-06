use crate::entity::EntityType;

pub(super) fn parse_entity_type_str(raw: &str) -> EntityType {
    match raw.to_uppercase().as_str() {
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
        _ => EntityType::Other(raw.to_string()),
    }
}
