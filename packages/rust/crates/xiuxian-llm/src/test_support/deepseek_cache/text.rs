#[must_use]
pub(super) fn normalize_view(text: &str) -> Option<String> {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::normalize_cache_text_view_for_tests(text)
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }
}

#[must_use]
pub(super) fn normalize_owned(text: String) -> Option<String> {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::normalize_cache_text_owned_for_tests(text)
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else if trimmed.len() == text.len() {
            Some(text)
        } else {
            Some(trimmed.to_string())
        }
    }
}
