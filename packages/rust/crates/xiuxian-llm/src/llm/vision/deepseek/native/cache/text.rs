/// Text normalization utilities for cache keys.

pub fn trim_non_empty(text: &str) -> Option<&str> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

pub fn normalize_owned_non_empty(text: String) -> Option<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Normalizes cache text from a view (for test API compatibility).
pub fn normalize_cache_text_view_for_tests(text: &str) -> Option<String> {
    trim_non_empty(text).map(std::string::ToString::to_string)
}

/// Normalizes cache text from owned string (for test API compatibility).
pub fn normalize_cache_text_owned_for_tests(text: String) -> Option<String> {
    normalize_owned_non_empty(text)
}
