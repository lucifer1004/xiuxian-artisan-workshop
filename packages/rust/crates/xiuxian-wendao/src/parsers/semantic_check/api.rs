//! Parser-owned semantic-check grammar helpers.

use super::types::HashReference;

/// Extract ID references from text content.
///
/// Looks for wiki-style links like `[[#id]]` or `[[id]]`.
#[allow(clippy::expect_used)]
#[must_use]
pub(crate) fn extract_id_references(text: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '[' && chars.peek() == Some(&'[') {
            chars.next();
            let mut link_content = String::new();
            while let Some(&next) = chars.peek() {
                if next == ']' {
                    chars.next();
                    if chars.peek() == Some(&']') {
                        chars.next();
                        break;
                    }
                    link_content.push(']');
                } else {
                    link_content.push(chars.next().expect("char exists after peek"));
                }
            }
            let link = link_content.trim();
            if link.starts_with('#') {
                refs.push(link.to_string());
            }
        }
    }
    refs
}

/// Extract hash-annotated references from text content.
///
/// Format: `[[#id@hash]]` where `@hash` is the expected content hash.
#[allow(clippy::expect_used)]
#[must_use]
pub(crate) fn extract_hash_references(text: &str) -> Vec<HashReference> {
    let mut refs = Vec::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '[' && chars.peek() == Some(&'[') {
            chars.next();
            let mut link_content = String::new();
            while let Some(&next) = chars.peek() {
                if next == ']' {
                    chars.next();
                    if chars.peek() == Some(&']') {
                        chars.next();
                        break;
                    }
                    link_content.push(']');
                } else {
                    link_content.push(chars.next().expect("char exists after peek"));
                }
            }
            let link = link_content.trim();
            if let Some(id_part) = link.strip_prefix('#') {
                if let Some(at_pos) = id_part.find('@') {
                    let target_id = id_part[..at_pos].to_string();
                    let expect_hash = id_part[at_pos + 1..].to_string();
                    refs.push(HashReference {
                        target_id,
                        expect_hash: Some(expect_hash),
                    });
                } else {
                    refs.push(HashReference {
                        target_id: id_part.to_string(),
                        expect_hash: None,
                    });
                }
            }
        }
    }
    refs
}

/// Validate a contract expression against content.
///
/// Supported contract formats:
/// - `must_contain("term1", "term2", ...)`
/// - `must_not_contain("term")`
/// - `min_length(N)`
#[must_use]
pub(crate) fn validate_contract(contract: &str, content: &str) -> Option<String> {
    let contract = contract.trim();

    if let Some(args) = extract_function_args(contract, "must_contain") {
        let terms: Vec<&str> = args
            .split(',')
            .map(|s| s.trim().trim_matches('"').trim())
            .filter(|s| !s.is_empty())
            .collect();

        for term in terms {
            if !content.contains(term) {
                return Some(format!("missing required term '{term}'"));
            }
        }
        return None;
    }

    if let Some(args) = extract_function_args(contract, "must_not_contain") {
        let term = args.trim().trim_matches('"').trim();
        if content.contains(term) {
            return Some(format!("contains forbidden term '{term}'"));
        }
        return None;
    }

    if let Some(args) = extract_function_args(contract, "min_length") {
        if let Ok(min_len) = args.trim().parse::<usize>()
            && content.len() < min_len
        {
            return Some(format!(
                "content length {} is less than required {}",
                content.len(),
                min_len
            ));
        }
        return None;
    }

    None
}

/// Extract arguments from a function-like contract expression.
#[must_use]
pub(crate) fn extract_function_args<'a>(contract: &'a str, function_name: &str) -> Option<&'a str> {
    let prefix = format!("{function_name}(");
    if contract.starts_with(&prefix) && contract.ends_with(')') {
        Some(&contract[prefix.len()..contract.len() - 1])
    } else {
        None
    }
}

/// Generate a suggested ID from a title.
#[must_use]
pub(crate) fn generate_suggested_id(title: &str) -> String {
    title
        .to_lowercase()
        .replace(' ', "-")
        .replace(|c: char| !c.is_alphanumeric() && c != '-', "")
        .trim_matches('-')
        .to_string()
}
