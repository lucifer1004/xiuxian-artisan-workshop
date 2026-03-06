use std::sync::Arc;

mod policy;
mod storage;

pub(in crate::llm::vision::deepseek::native) fn local_get(key: &str) -> Option<String> {
    storage::get(key)
}

pub(in crate::llm::vision::deepseek::native) fn local_get_shared(key: &str) -> Option<Arc<str>> {
    storage::get_shared(key)
}

pub(in crate::llm::vision::deepseek::native) fn local_set(key: &str, markdown: &str) {
    local_set_with_max_entries_for_tests(key, markdown, policy::resolve_max_entries());
}

pub(in crate::llm::vision::deepseek::native) fn local_set_with_max_entries_for_tests(
    key: &str,
    markdown: &str,
    max_entries: usize,
) {
    let max_entries = policy::normalize_max_entries(max_entries);
    if policy::should_clear_before_insert(storage::len(), max_entries) {
        storage::clear();
    }
    storage::insert(key, markdown);
}

pub(in crate::llm::vision::deepseek::native) fn local_clear_for_tests() {
    storage::clear();
}
