use std::collections::HashMap;
use std::sync::{Arc, OnceLock, RwLock};

type LocalCache = HashMap<String, Arc<str>>;

static OCR_MARKDOWN_CACHE: OnceLock<RwLock<LocalCache>> = OnceLock::new();

pub(super) fn get(key: &str) -> Option<String> {
    let guard = local_cache().read().ok()?;
    guard.get(key).map(|value| value.as_ref().to_string())
}

pub(super) fn len() -> usize {
    local_cache().read().map_or(0, |guard| guard.len())
}

pub(super) fn clear() {
    if let Ok(mut guard) = local_cache().write() {
        guard.clear();
    }
}

pub(super) fn insert(key: &str, markdown: &str) {
    if let Ok(mut guard) = local_cache().write() {
        guard.insert(key.to_string(), Arc::<str>::from(markdown));
    }
}

fn local_cache() -> &'static RwLock<LocalCache> {
    OCR_MARKDOWN_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}
