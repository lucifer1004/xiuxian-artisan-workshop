/// Normalize one relative directory entry for config-driven path lists.
#[must_use]
pub fn normalize_relative_dir(value: &str) -> Option<String> {
    let normalized = value
        .trim()
        .replace('\\', "/")
        .trim_matches('/')
        .to_string();
    if normalized.is_empty() || normalized == "." {
        None
    } else {
        Some(normalized)
    }
}

/// Deduplicate directory entries while preserving original order.
#[must_use]
pub fn dedup_dirs(entries: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for entry in entries {
        let lowered = entry.to_lowercase();
        if seen.insert(lowered) {
            out.push(entry);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{dedup_dirs, normalize_relative_dir};

    #[test]
    fn dir_helpers_normalize_and_deduplicate() {
        assert_eq!(normalize_relative_dir(" /src/ "), Some("src".to_string()));
        assert_eq!(normalize_relative_dir("."), None);
        assert_eq!(
            dedup_dirs(vec![
                "src".to_string(),
                "SRC".to_string(),
                "tests".to_string(),
            ]),
            vec!["src".to_string(), "tests".to_string()]
        );
    }
}
