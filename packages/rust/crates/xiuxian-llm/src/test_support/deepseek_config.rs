use std::path::Path;

/// Test-facing `DeepSeek` cache configuration snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepseekCacheConfigSnapshot {
    /// Maximum local cache entries.
    pub local_max_entries: Option<usize>,
    /// Optional `valkey` endpoint URL.
    pub valkey_url: Option<String>,
    /// Optional key prefix for shared caches.
    pub key_prefix: Option<String>,
    /// Optional cache TTL in seconds.
    pub ttl_seconds: Option<u64>,
    /// Optional cache operation timeout in milliseconds.
    pub timeout_ms: Option<u64>,
}

/// Test-facing `DeepSeek` runtime configuration snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeepseekConfigSnapshot {
    /// Model root directory.
    pub model_root: Option<String>,
    /// Weights path override.
    pub weights_path: Option<String>,
    /// Snapshot path override.
    pub snapshot_path: Option<String>,
    /// OCR prompt template override.
    pub ocr_prompt: Option<String>,
    /// Device hint.
    pub device: Option<String>,
    /// Base size used by preprocessing.
    pub base_size: Option<u32>,
    /// Image size used by preprocessing.
    pub image_size: Option<u32>,
    /// Crop-mode switch.
    pub crop_mode: Option<bool>,
    /// Decoder max new tokens.
    pub max_new_tokens: Option<usize>,
    /// Cache subsection snapshot.
    pub cache: DeepseekCacheConfigSnapshot,
}

/// Load `DeepSeek` config with explicit path roots for integration tests.
#[must_use]
pub fn load_deepseek_config_with_paths(
    project_root: Option<&Path>,
    config_home: Option<&Path>,
) -> DeepseekConfigSnapshot {
    let snapshot =
        crate::llm::vision::deepseek::load_config_with_paths_for_tests(project_root, config_home);
    DeepseekConfigSnapshot {
        model_root: snapshot.model_root,
        weights_path: snapshot.weights_path,
        snapshot_path: snapshot.snapshot_path,
        ocr_prompt: snapshot.ocr_prompt,
        device: snapshot.device,
        base_size: snapshot.base_size,
        image_size: snapshot.image_size,
        crop_mode: snapshot.crop_mode,
        max_new_tokens: snapshot.max_new_tokens,
        cache: DeepseekCacheConfigSnapshot {
            local_max_entries: snapshot.cache.local_max_entries,
            valkey_url: snapshot.cache.valkey_url,
            key_prefix: snapshot.cache.key_prefix,
            ttl_seconds: snapshot.cache.ttl_seconds,
            timeout_ms: snapshot.cache.timeout_ms,
        },
    }
}

/// Resolve the `DeepSeek` model root using env/config/default precedence.
#[must_use]
pub fn resolve_deepseek_model_root_with(
    env_model_root: Option<&str>,
    config_model_root: Option<&str>,
    default_model_root: Option<&str>,
) -> Option<String> {
    crate::llm::vision::deepseek::resolve_model_root_with_for_tests(
        env_model_root,
        config_model_root,
        default_model_root,
    )
}

/// Normalize `DeepSeek` model root relative to `project_root` when needed.
#[must_use]
pub fn normalize_deepseek_model_root(raw: &str, project_root: &Path) -> String {
    crate::llm::vision::deepseek::normalize_model_root_for_tests(raw, project_root)
}
