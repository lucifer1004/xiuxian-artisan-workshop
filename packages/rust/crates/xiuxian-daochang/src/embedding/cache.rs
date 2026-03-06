use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::sync::RwLock;

#[derive(Clone)]
struct EmbeddingCacheEntry {
    vector: Vec<f32>,
    cached_at: Instant,
}

pub(crate) struct EmbeddingCache {
    inner: RwLock<HashMap<String, EmbeddingCacheEntry>>,
    ttl: Duration,
    max_entries: usize,
}

impl EmbeddingCache {
    pub(crate) fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
            ttl,
            max_entries,
        }
    }

    pub(crate) async fn get_batch(
        &self,
        texts: &[String],
        model: Option<&str>,
    ) -> Option<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Some(vec![]);
        }
        let now = Instant::now();
        let mut expired_keys = Vec::new();
        let mut vectors = Vec::with_capacity(texts.len());
        let mut all_hit = true;
        {
            let cache = self.inner.read().await;
            for text in texts {
                let key = cache_key(text, model);
                match cache.get(&key) {
                    Some(entry) if now.duration_since(entry.cached_at) <= self.ttl => {
                        vectors.push(entry.vector.clone());
                    }
                    Some(_) => {
                        all_hit = false;
                        expired_keys.push(key);
                    }
                    None => {
                        all_hit = false;
                    }
                }
            }
        }

        if !expired_keys.is_empty() {
            let mut cache = self.inner.write().await;
            for key in expired_keys {
                cache.remove(&key);
            }
        }

        if all_hit { Some(vectors) } else { None }
    }

    pub(crate) async fn put_batch(
        &self,
        texts: &[String],
        vectors: &[Vec<f32>],
        model: Option<&str>,
    ) {
        if texts.is_empty() || vectors.is_empty() || texts.len() != vectors.len() {
            return;
        }
        let mut cache = self.inner.write().await;
        for (text, vector) in texts.iter().zip(vectors.iter()) {
            let key = cache_key(text, model);
            cache.insert(
                key,
                EmbeddingCacheEntry {
                    vector: vector.clone(),
                    cached_at: Instant::now(),
                },
            );
        }

        while cache.len() > self.max_entries {
            let Some(oldest_key) = cache
                .iter()
                .min_by_key(|(_, entry)| entry.cached_at)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            cache.remove(&oldest_key);
        }
    }
}

fn cache_key(text: &str, model: Option<&str>) -> String {
    match model.map(str::trim).filter(|value| !value.is_empty()) {
        Some(model) => format!("{model}\n{text}"),
        None => text.to_string(),
    }
}
