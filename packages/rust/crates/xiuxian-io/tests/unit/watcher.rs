//! Integration tests for `xiuxian-io` watcher configuration.

#![cfg(feature = "notify")]

use crate::WatcherConfig;

#[tokio::test]
async fn test_watcher_config() {
    let config = WatcherConfig::default();
    assert!(config.patterns.contains(&"**/*".to_string()));
    assert!(config.exclude.iter().any(|e| e.contains("*.pyc")));
}
