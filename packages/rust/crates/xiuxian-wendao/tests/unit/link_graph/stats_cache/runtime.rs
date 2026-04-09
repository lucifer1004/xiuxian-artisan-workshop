use crate::link_graph::runtime_config::DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX;

use super::normalize_stats_cache_key_prefix;

#[test]
fn normalize_stats_cache_key_prefix_falls_back_for_blank_input() {
    assert_eq!(
        normalize_stats_cache_key_prefix("   "),
        DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX.to_string()
    );
}

#[test]
fn normalize_stats_cache_key_prefix_trims_non_blank_input() {
    assert_eq!(
        normalize_stats_cache_key_prefix("  xiuxian:stats  "),
        "xiuxian:stats".to_string()
    );
}
