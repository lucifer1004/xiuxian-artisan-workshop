use super::raw::split_frontmatter;
use super::types::NoteFrontmatter;
use serde_yaml::{Mapping, Value};

fn mapping_value<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(Value::String(key.to_string()))
}

fn mapping_string(mapping: &Mapping, key: &str) -> Option<String> {
    mapping_value(mapping, key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn mapping_string_vec(mapping: &Mapping, key: &str) -> Vec<String> {
    mapping_value(mapping, key)
        .and_then(Value::as_sequence)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Parse semantic note frontmatter used by shared document consumers.
#[must_use]
pub fn parse_frontmatter(content: &str) -> NoteFrontmatter {
    let (frontmatter, _body) = split_frontmatter(content);
    let Some(mapping) = frontmatter.as_ref().and_then(Value::as_mapping) else {
        return NoteFrontmatter::default();
    };

    let metadata = mapping_value(mapping, "metadata").and_then(Value::as_mapping);
    let mut tags = mapping_string_vec(mapping, "tags");
    if tags.is_empty() {
        tags = metadata.map_or_else(Vec::new, |value| mapping_string_vec(value, "tags"));
    }

    NoteFrontmatter {
        title: mapping_string(mapping, "title"),
        description: mapping_string(mapping, "description"),
        name: mapping_string(mapping, "name"),
        category: mapping_string(mapping, "category"),
        tags,
        routing_keywords: metadata.map_or_else(Vec::new, |value| {
            mapping_string_vec(value, "routing_keywords")
        }),
        intents: metadata.map_or_else(Vec::new, |value| mapping_string_vec(value, "intents")),
    }
}
