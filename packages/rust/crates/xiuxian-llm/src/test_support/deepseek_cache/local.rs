#[must_use]
pub(super) fn get(key: &str) -> Option<String> {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::local_cache_get_for_tests(key)
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = key;
        None
    }
}

pub(super) fn set_with_max_entries(key: &str, markdown: &str, max_entries: usize) {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::local_cache_set_with_max_entries_for_tests(
            key,
            markdown,
            max_entries,
        );
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = key;
        let _ = markdown;
        let _ = max_entries;
    }
}

pub(super) fn clear() {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::local_cache_clear_for_tests();
    }
}
