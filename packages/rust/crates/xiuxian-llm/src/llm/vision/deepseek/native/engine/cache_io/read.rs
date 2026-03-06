use std::time::Instant;

use super::super::super::super::super::preprocess::PreparedVisionImage;
use super::super::super::cache::{
    local_get_shared, local_set, normalize_owned_non_empty, trim_non_empty, valkey_get,
};
use super::layer::CacheLayer;

pub(in crate::llm::vision::deepseek::native::engine) fn read_cache_entry(
    cache_layer: CacheLayer,
    cache_key: &str,
    prepared: &PreparedVisionImage,
    total_started: Instant,
) -> Option<String> {
    match cache_layer {
        CacheLayer::Local => {
            let hit = local_get_shared(cache_key)?;
            let text = trim_non_empty(hit.as_ref()).map(ToString::to_string);
            complete_cache_read(cache_layer, text, None, prepared, total_started)
        }
        CacheLayer::Valkey => {
            let hit = valkey_get(cache_key)?;
            let text = normalize_owned_non_empty(hit);
            complete_cache_read(cache_layer, text, Some(cache_key), prepared, total_started)
        }
    }
}

fn complete_cache_read(
    cache_layer: CacheLayer,
    text: Option<String>,
    local_backfill_key: Option<&str>,
    prepared: &PreparedVisionImage,
    total_started: Instant,
) -> Option<String> {
    let Some(text) = text else {
        tracing::debug!(
            event = "llm.vision.deepseek.cache.empty_ignored",
            cache_layer = cache_layer.as_str(),
            elapsed_ms = total_started.elapsed().as_millis(),
            width = prepared.width,
            height = prepared.height,
            "DeepSeek OCR cache entry is empty; bypassing cache"
        );
        return None;
    };

    if let Some(cache_key) = local_backfill_key {
        local_set(cache_key, text.as_str());
    }

    tracing::debug!(
        event = "llm.vision.deepseek.cache.hit",
        cache_layer = cache_layer.as_str(),
        chars = text.chars().count(),
        elapsed_ms = total_started.elapsed().as_millis(),
        width = prepared.width,
        height = prepared.height,
        "DeepSeek OCR served from cache"
    );

    Some(text)
}
