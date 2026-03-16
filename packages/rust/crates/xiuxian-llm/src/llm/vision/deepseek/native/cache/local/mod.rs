use std::sync::Arc;

mod policy;
mod storage;

pub struct DeepseekLocalCache;

impl DeepseekLocalCache {
    pub fn get(key: &str) -> Option<String> {
        storage::get(key)
    }

    pub fn get_shared(key: &str) -> Option<Arc<str>> {
        storage::get_shared(key)
    }

    pub fn set(key: &str, markdown: &str) {
        Self::set_with_max_entries(key, markdown, policy::resolve_max_entries());
    }

    pub fn set_with_max_entries(key: &str, markdown: &str, max_entries: usize) {
        let max_entries = policy::normalize_max_entries(max_entries);
        if policy::should_clear_before_insert(storage::len(), max_entries) {
            storage::clear();
        }
        storage::insert(key, markdown);
    }

    pub fn clear() {
        storage::clear();
    }
}

pub fn local_get(key: &str) -> Option<String> {
    DeepseekLocalCache::get(key)
}

pub fn local_get_shared(key: &str) -> Option<Arc<str>> {
    DeepseekLocalCache::get_shared(key)
}

pub fn local_set(key: &str, markdown: &str) {
    DeepseekLocalCache::set(key, markdown);
}

pub fn local_clear_for_tests() {
    DeepseekLocalCache::clear();
}
