use crate::entity::RelationType;

pub(crate) const PROPERTY_RELATION_KEYS: &[&str] =
    &["RELATED", "DEPENDS_ON", "EXTENDS", "SEE_ALSO"];

pub(crate) fn map_property_relation_type(key: &str) -> Option<RelationType> {
    match key {
        "RELATED" => Some(RelationType::RelatedTo),
        "DEPENDS_ON" => Some(RelationType::DependsOn),
        "EXTENDS" => Some(RelationType::Extends),
        "SEE_ALSO" => Some(RelationType::References),
        _ => None,
    }
}
