//! Unified and modular Xiuxian configuration loader.
//!
//! Supports unified `xiuxian.toml` plus optional modular `wendao.toml` fallback.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use xiuxian_config_core::{
    load_toml_value_with_imports, resolve_config_home as resolve_config_home_path,
    resolve_project_root_or_cwd,
};

use super::settings::RuntimeSettings;

const DEFAULT_SYSTEM_CONFIG_BASE_RELATIVE_PATH: &str =
    "packages/rust/crates/xiuxian-daochang/resources/config";
const DEFAULT_CONFIG_HOME_RELATIVE_PATH: &str = ".config";
const DEFAULT_USER_CONFIG_NAMESPACE: &str = "xiuxian-artisan-workshop";

/// The root configuration structure.
#[xiuxian_macros::xiuxian_config(
    namespace = "",
    internal_path = "resources/config/xiuxian.toml",
    orphan_file = ""
)]
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct XiuxianConfig {
    /// Consolidated runtime settings preserved for compatibility with unified `xiuxian.toml`.
    #[serde(flatten)]
    _runtime_settings: RuntimeSettings,

    /// LLM-specific provider configuration.
    #[serde(default)]
    pub llm: LlmConfig,

    /// Wendao (Knowledge Management) configuration.
    #[serde(default)]
    pub wendao: WendaoConfig,

    /// Qianhuan (orchestration/persona) configuration.
    #[serde(default)]
    pub qianhuan: QianhuanConfig,

    /// Zhenfa (HTTP matrix gateway) tool bridge settings.
    #[serde(default)]
    pub zhenfa: ZhenfaConfig,
}

/// LLM routing defaults and provider map for runtime model selection.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct LlmConfig {
    /// Provider key selected when no request-specific provider is supplied.
    pub default_provider: Option<String>,
    /// Optional model alias selected when no request-specific model is supplied.
    pub default_model: Option<String>,
    /// Wire protocol default for OpenAI-compatible transports (`chat_completions` or `responses`).
    pub wire_api: Option<String>,
    /// Named provider configurations keyed by provider id.
    #[serde(default)]
    pub providers: HashMap<String, LlmProviderConfig>,
}

/// Connection and model alias configuration for one LLM provider.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct LlmProviderConfig {
    /// Provider API base URL.
    pub base_url: Option<String>,
    /// Provider API key (literal token or env-key reference string).
    pub api_key: Option<String>,
    /// Provider-scoped default model for this provider profile.
    pub model: Option<String>,
    /// Provider-specific wire protocol (`chat_completions` or `responses`).
    pub wire_api: Option<String>,
    /// Logical model alias to concrete provider model mapping.
    #[serde(default)]
    pub model_aliases: HashMap<String, String>,
}

/// Wendao specific settings.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct WendaoConfig {
    /// Action-oriented configurations (Notebook path, timer settings, etc.)
    #[serde(default)]
    pub zhixing: ZhixingConfig,
    /// Link graph indexing settings.
    #[serde(default)]
    pub link_graph: LinkGraphConfig,
}

/// Zhixing runtime settings.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct ZhixingConfig {
    /// Root directory for the notebook.
    pub notebook_path: Option<String>,
    /// Time zone for scheduled tasks and reminders (e.g., "Asia/Shanghai").
    pub time_zone: Option<String>,
    /// Active persona id used for Zhixing workflow rendering.
    pub persona_id: Option<String>,
    /// Default notification recipient for timer reminders when task metadata has no recipient.
    pub notification_recipient: Option<String>,
    /// Template directories for Qianhuan manifestations.
    pub template_paths: Option<Vec<String>>,
    /// Optional Valkey-backed reminder queue settings.
    #[serde(default)]
    pub reminder_queue: ZhixingReminderQueueConfig,
}

/// Reminder queue settings.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct ZhixingReminderQueueConfig {
    /// Override Valkey URL for reminder queue.
    pub valkey_url: Option<String>,
    /// Valkey key prefix namespace.
    pub key_prefix: Option<String>,
    /// Queue poll interval in seconds.
    pub poll_interval_seconds: Option<u64>,
    /// Maximum consumed reminders per poll.
    pub poll_batch_size: Option<usize>,
}

/// Qianhuan specific settings.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct QianhuanConfig {
    /// Persona profile resolution settings.
    #[serde(default)]
    pub persona: QianhuanPersonaConfig,
    /// Template resolution settings.
    #[serde(default)]
    pub template: QianhuanTemplateConfig,
}

/// Persona location settings.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct QianhuanPersonaConfig {
    /// Optional single override directory for persona profiles.
    pub persona_dir: Option<String>,
    /// Optional ordered list of persona profile directories.
    pub persona_dirs: Option<Vec<String>>,
}

/// Template location settings.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct QianhuanTemplateConfig {
    /// Optional single override directory for Qianhuan templates.
    pub template_dir: Option<String>,
    /// Optional ordered list of template directories.
    pub template_dirs: Option<Vec<String>>,
}

/// Zhenfa tool bridge settings.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct ZhenfaConfig {
    /// Base URL for zhenfa gateway, for example `http://127.0.0.1:18093`.
    pub base_url: Option<String>,
    /// Explicit enabled RPC tools exposed to LLM (for example `wendao.search`).
    pub enabled_tools: Option<Vec<String>>,
    /// Optional Valkey runtime hooks for zhenfa native orchestrator.
    #[serde(default)]
    pub valkey: ZhenfaValkeyConfig,
}

/// Valkey hook settings for zhenfa native orchestrator.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct ZhenfaValkeyConfig {
    /// Valkey URL (for example `redis://127.0.0.1:6379/0`).
    pub url: Option<String>,
    /// Key prefix namespace for zhenfa cache/lock/stream entries.
    pub key_prefix: Option<String>,
    /// TTL for deterministic tool result cache entries.
    pub cache_ttl_seconds: Option<u64>,
    /// TTL for mutation lock lease entries.
    pub lock_ttl_seconds: Option<u64>,
    /// Audit stream suffix name used for `XADD` events.
    pub audit_stream: Option<String>,
}

/// Link graph indexing and watch settings for knowledge traversal.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct LinkGraphConfig {
    /// Link graph backend identifier.
    pub backend: Option<String>,
    /// Explicit include directories for indexing.
    pub include_dirs: Option<Vec<String>>,
    /// Enable automatic include directory discovery.
    pub include_dirs_auto: Option<bool>,
    /// Watch roots for incremental updates.
    pub watch_dirs: Option<Vec<String>>,
    /// Glob patterns to include during watch/index operations.
    pub watch_patterns: Option<Vec<String>>,
    /// File extensions allowed for watch/index operations.
    pub watch_extensions: Option<Vec<String>>,
    /// Directories excluded from indexing/watching.
    pub exclude_dirs: Option<Vec<String>>,
    /// Cache backend settings for link graph artifacts.
    #[serde(default)]
    pub cache: LinkGraphCacheConfig,
}

/// Cache configuration for link graph persistence and acceleration.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct LinkGraphCacheConfig {
    /// Valkey URL used for cache reads/writes.
    pub valkey_url: Option<String>,
    /// Key prefix namespace for link graph cache entries.
    pub key_prefix: Option<String>,
    /// Cache entry TTL in seconds.
    pub ttl_seconds: Option<u64>,
}

/// Resolve system and user config paths with the same base logic as runtime settings.
fn resolve_config_paths(filename: &str) -> (PathBuf, PathBuf) {
    let project_root = resolve_project_root_or_cwd();
    let (default_system_base, default_user_base) = default_config_bases(project_root.as_path());
    let (system_settings_path, user_settings_path) = super::settings::runtime_settings_paths();
    let system_base = system_settings_path
        .parent()
        .map_or_else(|| default_system_base.clone(), Path::to_path_buf);
    let user_base = user_settings_path
        .parent()
        .map_or_else(|| default_user_base.clone(), Path::to_path_buf);
    (system_base.join(filename), user_base.join(filename))
}

fn resolve_config_home(project_root: &Path) -> PathBuf {
    resolve_config_home_path(Some(project_root))
        .unwrap_or_else(|| project_root.join(DEFAULT_CONFIG_HOME_RELATIVE_PATH))
}

fn default_config_bases(project_root: &Path) -> (PathBuf, PathBuf) {
    (
        project_root.join(DEFAULT_SYSTEM_CONFIG_BASE_RELATIVE_PATH),
        resolve_config_home(project_root).join(DEFAULT_USER_CONFIG_NAMESPACE),
    )
}

fn read_toml_value(path: &Path) -> Option<toml::Value> {
    if !path.is_file() {
        return None;
    }
    match load_toml_value_with_imports(path) {
        Ok(value) => Some(value),
        Err(error) => {
            tracing::warn!(
                path = %path.display(),
                error = %error,
                "failed to load xiuxian config; ignoring"
            );
            None
        }
    }
}

fn merge_toml_values(dst: &mut toml::Value, src: toml::Value) {
    match (dst, src) {
        (toml::Value::Table(dst_table), toml::Value::Table(src_table)) => {
            for (key, src_value) in src_table {
                if let Some(dst_value) = dst_table.get_mut(&key) {
                    merge_toml_values(dst_value, src_value);
                } else {
                    dst_table.insert(key, src_value);
                }
            }
        }
        (dst_value, src_value) => {
            *dst_value = src_value;
        }
    }
}

fn parse_merged_config(merged: toml::Value, context: &str) -> XiuxianConfig {
    match merged.try_into::<XiuxianConfig>() {
        Ok(config) => config,
        Err(error) => {
            tracing::warn!(error = %error, %context, "failed to decode xiuxian config; using defaults");
            XiuxianConfig::default()
        }
    }
}

fn apply_modular_wendao_fallback(config: &mut XiuxianConfig, system_base: &Path, user_base: &Path) {
    if config.wendao.zhixing.notebook_path.is_some() {
        return;
    }

    let system_wendao_path = system_base.join("wendao.toml");
    let user_wendao_path = user_base.join("wendao.toml");
    let wendao_path = if user_wendao_path.exists() {
        user_wendao_path
    } else {
        system_wendao_path
    };

    if let Some(value) = read_toml_value(&wendao_path)
        && let Ok(wendao_only) = value.try_into::<WendaoConfig>()
    {
        config.wendao = wendao_only;
        tracing::info!(
            path = %wendao_path.display(),
            "Merged modular Wendao configuration."
        );
    }
}

fn config_home_from_user_xiuxian_path(path: &Path) -> Option<PathBuf> {
    // user path = <config_home>/xiuxian-artisan-workshop/xiuxian.toml
    path.parent().and_then(Path::parent).map(Path::to_path_buf)
}

/// Load and merge Xiuxian configuration from explicit system/user file paths.
///
/// This compatibility helper is primarily used in tests and explicit-path callers.
/// It performs a generic TOML deep merge (user overrides system).
#[must_use]
pub fn load_xiuxian_config_from_paths(system_path: &Path, user_path: &Path) -> XiuxianConfig {
    let mut merged =
        read_toml_value(system_path).unwrap_or_else(|| toml::Value::Table(toml::map::Map::new()));
    if let Some(overlay) = read_toml_value(user_path) {
        merge_toml_values(&mut merged, overlay);
    }
    parse_merged_config(merged, "from_paths")
}

/// Load and merge Xiuxian configuration using explicit system/user base directories.
///
/// This resolves `xiuxian.toml` under each base path and applies user-over-system overlay.
/// If notebook path is still absent after merge, it falls back to modular `wendao.toml`.
#[must_use]
pub fn load_xiuxian_config_from_bases(system_base: &Path, user_base: &Path) -> XiuxianConfig {
    let system_xiuxian_path = system_base.join("xiuxian.toml");
    let user_xiuxian_path = user_base.join("xiuxian.toml");
    let mut config = load_xiuxian_config_from_paths(&system_xiuxian_path, &user_xiuxian_path);
    apply_modular_wendao_fallback(&mut config, system_base, user_base);
    config
}

/// Primary loader that merges embedded system defaults and user overrides.
#[must_use]
pub fn load_xiuxian_config() -> XiuxianConfig {
    let (system_xiuxian_path, user_xiuxian_path) = resolve_config_paths("xiuxian.toml");

    let project_root = resolve_project_root_or_cwd();
    let config_home = config_home_from_user_xiuxian_path(&user_xiuxian_path);
    let (default_system_base, default_user_base) = default_config_bases(project_root.as_path());

    let mut config = XiuxianConfig::load_with_paths(
        Some(project_root.as_path()),
        config_home.as_deref(),
    )
    .unwrap_or_else(|error| {
        tracing::warn!(
            error = %error,
            "failed to load cascade xiuxian config via config-core; falling back to explicit paths"
        );
        load_xiuxian_config_from_paths(&system_xiuxian_path, &user_xiuxian_path)
    });

    let system_base = system_xiuxian_path
        .parent()
        .map_or_else(|| default_system_base, Path::to_path_buf);
    let user_base = user_xiuxian_path
        .parent()
        .map_or_else(|| default_user_base, Path::to_path_buf);
    apply_modular_wendao_fallback(&mut config, &system_base, &user_base);

    if system_xiuxian_path.is_file() {
        tracing::debug!(
            path = %system_xiuxian_path.display(),
            "Loaded system xiuxian configuration."
        );
    }
    if user_xiuxian_path.is_file() {
        tracing::debug!(
            path = %user_xiuxian_path.display(),
            "Loaded user xiuxian overlay configuration."
        );
    }

    config
}
