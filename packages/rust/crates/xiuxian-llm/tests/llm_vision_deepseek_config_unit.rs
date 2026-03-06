//! `DeepSeek` config namespace overlay tests.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use xiuxian_llm::test_support::load_deepseek_config_with_paths;

struct TempTree {
    root: PathBuf,
    config_home: PathBuf,
}

impl TempTree {
    fn new(label: &str) -> Self {
        let mut root = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_nanos());
        root.push(format!(
            "xiuxian-llm-deepseek-config-{label}-{}-{nanos}",
            std::process::id()
        ));
        if let Err(error) = fs::create_dir_all(&root) {
            panic!(
                "failed to create test temp root {}: {error}",
                root.display()
            );
        }
        let config_home = root.join("config-home");
        if let Err(error) = fs::create_dir_all(&config_home) {
            panic!(
                "failed to create config home {}: {error}",
                config_home.display()
            );
        }
        Self { root, config_home }
    }

    fn xiuxian_toml_path(&self) -> PathBuf {
        self.config_home
            .join("xiuxian-artisan-workshop")
            .join("xiuxian.toml")
    }
}

impl Drop for TempTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_toml(path: &Path, content: &str) {
    if let Some(parent) = path.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        panic!(
            "failed to create config parent directory {}: {error}",
            parent.display()
        );
    }
    if let Err(error) = fs::write(path, content) {
        panic!("failed to write config file {}: {error}", path.display());
    }
}

#[test]
fn load_config_uses_embedded_defaults_without_user_overlay() {
    let layout = TempTree::new("embedded-defaults");
    let config = load_deepseek_config_with_paths(
        Some(layout.root.as_path()),
        Some(layout.config_home.as_path()),
    );

    assert_eq!(config.model_root, None);
    assert_eq!(config.model_kind.as_deref(), Some("dots"));
    assert_eq!(config.dots_model_root, None);
    assert_eq!(config.base_size, Some(448));
    assert_eq!(config.image_size, Some(448));
    assert_eq!(config.max_tiles, Some(12));
    assert_eq!(config.max_new_tokens, Some(1_024));
    assert_eq!(config.decode_temperature, Some(0.0));
    assert_eq!(config.decode_top_p, Some(1.0));
    assert_eq!(config.decode_repetition_penalty, Some(1.0));
    assert_eq!(config.decode_use_cache, Some(true));
    assert_eq!(config.ocr_batch_window_ms, Some(50));
    assert_eq!(config.ocr_batch_max_size, Some(8));
    assert_eq!(config.auto_route_complex_min_tiles, Some(8));
    assert_eq!(config.auto_route_complex_min_pixels, Some(2_500_000));
    assert_eq!(config.ocr_inflight_wait_timeout_ms, Some(30_000));
    assert_eq!(config.ocr_inflight_stale_ms, Some(120_000));
    assert_eq!(config.cache.local_max_entries, Some(1_024));
    assert_eq!(config.cache.preprocess_local_max_entries, Some(128));
    assert_eq!(config.cache.ttl_seconds, Some(3_600));
}

#[test]
fn load_config_merges_user_namespace_over_embedded_defaults() {
    let layout = TempTree::new("user-overlay");
    write_toml(
        layout.xiuxian_toml_path().as_path(),
        r#"
[llm.vision.deepseek]
model_root = "/models/user"
model_kind = "dots"
dots_model_root = "/models/dots"
device = "metal"
decode_top_p = 0.92
decode_repetition_penalty = 1.08
ocr_batch_window_ms = 10
ocr_batch_max_size = 7
auto_route_complex_min_tiles = 5
auto_route_complex_min_pixels = 123456
ocr_inflight_wait_timeout_ms = 99
ocr_inflight_stale_ms = 111

[llm.vision.deepseek.cache]
preprocess_local_max_entries = 12
ttl_seconds = 42
timeout_ms = 11
"#,
    );

    let config = load_deepseek_config_with_paths(
        Some(layout.root.as_path()),
        Some(layout.config_home.as_path()),
    );

    assert_eq!(config.model_root.as_deref(), Some("/models/user"));
    assert_eq!(config.model_kind.as_deref(), Some("dots"));
    assert_eq!(config.dots_model_root.as_deref(), Some("/models/dots"));
    assert_eq!(config.device.as_deref(), Some("metal"));
    assert_eq!(config.base_size, Some(448));
    assert_eq!(config.image_size, Some(448));
    assert_eq!(config.max_tiles, Some(12));
    assert_eq!(config.max_new_tokens, Some(1_024));
    assert_eq!(config.decode_top_p, Some(0.92));
    assert_eq!(config.decode_repetition_penalty, Some(1.08));
    assert_eq!(config.ocr_batch_window_ms, Some(10));
    assert_eq!(config.ocr_batch_max_size, Some(7));
    assert_eq!(config.auto_route_complex_min_tiles, Some(5));
    assert_eq!(config.auto_route_complex_min_pixels, Some(123_456));
    assert_eq!(config.ocr_inflight_wait_timeout_ms, Some(99));
    assert_eq!(config.ocr_inflight_stale_ms, Some(111));
    assert_eq!(config.cache.preprocess_local_max_entries, Some(12));
    assert_eq!(config.cache.ttl_seconds, Some(42));
    assert_eq!(config.cache.timeout_ms, Some(11));
    assert_eq!(
        config.cache.key_prefix.as_deref(),
        Some("xiuxian:vision:ocr:v1")
    );
}

#[test]
fn load_config_ignores_legacy_namespace() {
    let layout = TempTree::new("legacy-ignored");
    write_toml(
        layout.xiuxian_toml_path().as_path(),
        r#"
[vision.deepseek]
model_root = "/models/legacy"

[vision.deepseek.cache]
ttl_seconds = 1
"#,
    );

    let config = load_deepseek_config_with_paths(
        Some(layout.root.as_path()),
        Some(layout.config_home.as_path()),
    );

    assert_eq!(config.model_root, None);
    assert_eq!(config.model_kind.as_deref(), Some("dots"));
    assert_eq!(config.dots_model_root, None);
    assert_eq!(config.cache.ttl_seconds, Some(3_600));
}
