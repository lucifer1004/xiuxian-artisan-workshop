use super::keys::{PROPERTY_RELATION_KEYS, map_property_relation_type};
use super::targets::parse_relation_targets;
use super::types::{ExplicitRelationSource, ExplicitSectionRelation};
use crate::parsers::markdown::ParsedSection;
use crate::parsers::markdown::content::parse_frontmatter as parse_raw_frontmatter;
use crate::parsers::markdown::sections::extract_sections_without_source_context;

#[must_use]
pub fn parse_property_relations(content: &str) -> Vec<ExplicitSectionRelation> {
    let (_frontmatter, body) = parse_raw_frontmatter(content);
    let sections = extract_sections_without_source_context(body);
    extract_property_relations(&sections)
}

#[must_use]
pub fn extract_property_relations(sections: &[ParsedSection]) -> Vec<ExplicitSectionRelation> {
    let mut relations = Vec::new();

    for section in sections {
        let source = ExplicitRelationSource {
            heading_path: section.heading_path.clone(),
            explicit_id: section.attributes.get("ID").cloned(),
        };

        for property_key in PROPERTY_RELATION_KEYS {
            let Some(value) = section.attributes.get(*property_key) else {
                continue;
            };
            let Some(relation_type) = map_property_relation_type(property_key) else {
                continue;
            };

            for target in parse_relation_targets(value) {
                relations.push(ExplicitSectionRelation {
                    property_key: (*property_key).to_string(),
                    relation_type: relation_type.clone(),
                    source: source.clone(),
                    target,
                });
            }
        }
    }

    relations
}
