use crate::link_graph::addressing::Address;

use super::types::ExplicitRelationTarget;

pub(crate) fn parse_relation_targets(value: &str) -> Vec<ExplicitRelationTarget> {
    tokenize_relation_value(value)
        .into_iter()
        .filter_map(|token| parse_relation_target_token(&token))
        .collect()
}

fn tokenize_relation_value(value: &str) -> Vec<String> {
    let bytes = value.as_bytes();
    let mut tokens = Vec::new();
    let mut index = 0usize;

    while index < bytes.len() {
        while index < bytes.len()
            && matches!(bytes[index], b' ' | b'\t' | b'\r' | b'\n' | b',' | b';')
        {
            index += 1;
        }

        if index >= bytes.len() {
            break;
        }

        if bytes[index..].starts_with(b"[[") {
            let start = index;
            index += 2;
            while index + 1 < bytes.len() && !bytes[index..].starts_with(b"]]") {
                index += 1;
            }
            if index + 1 < bytes.len() {
                index += 2;
            } else {
                index = bytes.len();
            }
            tokens.push(value[start..index].trim().to_string());
            continue;
        }

        let start = index;
        while index < bytes.len() && !matches!(bytes[index], b',' | b';' | b'\r' | b'\n') {
            index += 1;
        }
        let token = value[start..index].trim();
        if !token.is_empty() {
            tokens.push(token.to_string());
        }
    }

    tokens
}

fn parse_relation_target_token(token: &str) -> Option<ExplicitRelationTarget> {
    let trimmed = token.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(inner) = trimmed
        .strip_prefix("[[")
        .and_then(|value| value.strip_suffix("]]"))
    {
        return parse_wiki_target(inner.trim(), trimmed.to_string());
    }

    parse_address_only_target(trimmed, trimmed.to_string())
}

fn parse_wiki_target(inner: &str, original: String) -> Option<ExplicitRelationTarget> {
    if inner.starts_with(['#', '@', '/']) {
        let address = Address::parse(inner)?;
        return Some(ExplicitRelationTarget {
            note_target: None,
            address: Some(address),
            original,
        });
    }

    if let Some((note_target, address)) = split_note_and_scope(inner) {
        return Some(ExplicitRelationTarget {
            note_target: Some(note_target),
            address,
            original,
        });
    }

    let note_target = inner.trim();
    if note_target.is_empty() {
        return None;
    }

    Some(ExplicitRelationTarget {
        note_target: Some(note_target.to_string()),
        address: None,
        original,
    })
}

fn parse_address_only_target(raw: &str, original: String) -> Option<ExplicitRelationTarget> {
    Address::parse(raw).map(|address| ExplicitRelationTarget {
        note_target: None,
        address: Some(address),
        original,
    })
}

fn split_note_and_scope(inner: &str) -> Option<(String, Option<Address>)> {
    if let Some((note_target, scope)) = inner.split_once('#') {
        let note_target = note_target.trim();
        let scope = scope.trim();
        if note_target.is_empty() {
            return None;
        }

        if scope.is_empty() {
            return Some((note_target.to_string(), None));
        }

        let address = if scope.starts_with('/') {
            Address::parse(scope)
        } else {
            Address::parse(&format!("#{scope}"))
        };

        return Some((note_target.to_string(), address));
    }

    if let Some((note_target, scope)) = inner.split_once('@') {
        let note_target = note_target.trim();
        let scope = scope.trim();
        if note_target.is_empty() || scope.is_empty() {
            return None;
        }

        return Some((
            note_target.to_string(),
            Address::parse(&format!("@{scope}")),
        ));
    }

    None
}
