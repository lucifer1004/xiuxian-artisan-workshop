use crate::link_graph::runtime_config::{
    DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX, resolve_link_graph_cache_runtime,
};
use crate::valkey_common::normalize_key_prefix;
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) fn now_unix_f64() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0.0, |d| d.as_secs_f64())
}

pub(super) fn resolve_stats_cache_runtime() -> Result<(String, String), String> {
    let runtime = resolve_link_graph_cache_runtime()?;
    let key_prefix = normalize_stats_cache_key_prefix(runtime.key_prefix.as_str());
    Ok((runtime.valkey_url, key_prefix))
}

fn normalize_stats_cache_key_prefix(candidate: &str) -> String {
    normalize_key_prefix(candidate, DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX)
}

#[cfg(test)]
#[path = "../../../tests/unit/link_graph/stats_cache/runtime.rs"]
mod tests;
