use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};

use sha2::{Digest, Sha256};

use super::super::super::super::preprocess::PreparedVisionImage;
use super::super::super::config;

const DEFAULT_FINGERPRINT_CACHE_MAX_ENTRIES: usize = 128;

static FINGERPRINT_CACHE: OnceLock<Mutex<FingerprintCacheStore>> = OnceLock::new();

#[derive(Default)]
struct FingerprintCacheStore {
    entries: HashMap<ResizedPngKey, Arc<str>>,
    order: VecDeque<ResizedPngKey>,
}

#[derive(Clone)]
struct ResizedPngKey(Arc<[u8]>);

impl PartialEq for ResizedPngKey {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for ResizedPngKey {}

impl Hash for ResizedPngKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state);
    }
}

pub(in crate::llm::vision::deepseek) fn prepared_pixels_fingerprint(
    prepared: &PreparedVisionImage,
) -> Arc<str> {
    let max_entries = fingerprint_cache_max_entries();
    if max_entries == 0 {
        return Arc::<str>::from(build_fingerprint(prepared));
    }

    let key = ResizedPngKey(Arc::clone(&prepared.resized_png));
    if let Some(fingerprint) = read_cached_fingerprint(&key) {
        return fingerprint;
    }

    let fingerprint = Arc::<str>::from(build_fingerprint(prepared));
    store_cached_fingerprint(key, Arc::clone(&fingerprint), max_entries);
    fingerprint
}

pub(in crate::llm::vision::deepseek) fn fingerprint_cache_clear_for_tests() {
    let cache = FINGERPRINT_CACHE.get_or_init(|| Mutex::new(FingerprintCacheStore::default()));
    let mut guard = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.entries.clear();
    guard.order.clear();
}

pub(in crate::llm::vision::deepseek) fn fingerprint_cache_len_for_tests() -> usize {
    let cache = FINGERPRINT_CACHE.get_or_init(|| Mutex::new(FingerprintCacheStore::default()));
    let guard = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.entries.len()
}

fn build_fingerprint(prepared: &PreparedVisionImage) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prepared.resized_png.as_ref());
    hex::encode(hasher.finalize())
}

fn read_cached_fingerprint(key: &ResizedPngKey) -> Option<Arc<str>> {
    let cache = FINGERPRINT_CACHE.get_or_init(|| Mutex::new(FingerprintCacheStore::default()));
    let guard = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.entries.get(key).cloned()
}

fn store_cached_fingerprint(key: ResizedPngKey, fingerprint: Arc<str>, max_entries: usize) {
    let cache = FINGERPRINT_CACHE.get_or_init(|| Mutex::new(FingerprintCacheStore::default()));
    let mut guard = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    if guard.entries.contains_key(&key) {
        guard.order.retain(|existing| existing != &key);
    }

    guard.order.push_back(key.clone());
    guard.entries.insert(key, fingerprint);

    while guard.entries.len() > max_entries {
        let Some(oldest) = guard.order.pop_front() else {
            break;
        };
        guard.entries.remove(&oldest);
    }
}

fn fingerprint_cache_max_entries() -> usize {
    std::env::var("XIUXIAN_VISION_OCR_PREPROCESS_CACHE_LOCAL_MAX_ENTRIES")
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .or_else(config::cache_local_max_entries)
        .unwrap_or(DEFAULT_FINGERPRINT_CACHE_MAX_ENTRIES)
}
