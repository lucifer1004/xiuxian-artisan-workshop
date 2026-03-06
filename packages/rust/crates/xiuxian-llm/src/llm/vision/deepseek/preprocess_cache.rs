use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

use sha2::{Digest, Sha256};

use super::config;
use crate::llm::error::LlmResult;
use crate::llm::vision::{PreparedVisionImage, preprocess_image_with_max_dimension};

const DEFAULT_PREPROCESS_LOCAL_MAX_ENTRIES: usize = 128;

static PREPROCESS_CACHE: OnceLock<Mutex<PreprocessCacheStore>> = OnceLock::new();

#[derive(Default)]
struct PreprocessCacheStore {
    entries: HashMap<String, PreparedVisionImage>,
    order: VecDeque<String>,
}

/// Preprocess an image for `DeepSeek` OCR with local prepared-image caching.
///
/// The cache is keyed by original bytes plus the requested max dimension, so
/// repeated OCR turns over the same image can skip decode and resize work.
///
/// # Errors
///
/// Returns an error when the underlying image decode or resize pipeline fails.
pub fn preprocess_image_for_ocr(
    image_bytes: Arc<[u8]>,
    max_dimension: u32,
) -> LlmResult<PreparedVisionImage> {
    let max_entries = preprocess_local_max_entries();
    if max_entries == 0 {
        return preprocess_image_with_max_dimension(image_bytes, max_dimension);
    }

    let cache_key = build_preprocess_cache_key(image_bytes.as_ref(), max_dimension);
    if let Some(prepared) = read_preprocessed_image(cache_key.as_str()) {
        tracing::debug!(
            event = "llm.vision.deepseek.preprocess_cache.hit",
            max_dimension,
            width = prepared.width,
            height = prepared.height,
            entries = cache_len(),
            "DeepSeek OCR served prepared image from local preprocess cache"
        );
        return Ok(prepared);
    }

    let prepared = preprocess_image_with_max_dimension(image_bytes, max_dimension)?;
    tracing::debug!(
        event = "llm.vision.deepseek.preprocess_cache.miss",
        max_dimension,
        width = prepared.width,
        height = prepared.height,
        entries = cache_len(),
        "DeepSeek OCR prepared image computed and stored in local preprocess cache"
    );
    store_preprocessed_image(cache_key.as_str(), prepared.clone(), max_entries);
    Ok(prepared)
}

fn build_preprocess_cache_key(image_bytes: &[u8], max_dimension: u32) -> String {
    let mut hasher = Sha256::new();
    hasher.update(max_dimension.to_le_bytes());
    hasher.update((image_bytes.len() as u64).to_le_bytes());
    hasher.update(image_bytes);
    format!(
        "vision:deepseek:preprocess:{}",
        hex::encode(hasher.finalize())
    )
}

fn read_preprocessed_image(cache_key: &str) -> Option<PreparedVisionImage> {
    let cache = PREPROCESS_CACHE.get_or_init(|| Mutex::new(PreprocessCacheStore::default()));
    let guard = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.entries.get(cache_key).cloned()
}

fn store_preprocessed_image(cache_key: &str, prepared: PreparedVisionImage, max_entries: usize) {
    let cache = PREPROCESS_CACHE.get_or_init(|| Mutex::new(PreprocessCacheStore::default()));
    let mut guard = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    if guard.entries.contains_key(cache_key) {
        guard.order.retain(|existing| existing != cache_key);
    }

    guard.order.push_back(cache_key.to_string());
    guard.entries.insert(cache_key.to_string(), prepared);

    while guard.entries.len() > max_entries {
        let Some(oldest) = guard.order.pop_front() else {
            break;
        };
        guard.entries.remove(oldest.as_str());
    }
}

fn preprocess_local_max_entries() -> usize {
    std::env::var("XIUXIAN_VISION_OCR_PREPROCESS_CACHE_LOCAL_MAX_ENTRIES")
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .or_else(config::preprocess_local_max_entries)
        .unwrap_or(DEFAULT_PREPROCESS_LOCAL_MAX_ENTRIES)
}

fn cache_len() -> usize {
    let cache = PREPROCESS_CACHE.get_or_init(|| Mutex::new(PreprocessCacheStore::default()));
    let guard = cache
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.entries.len()
}
