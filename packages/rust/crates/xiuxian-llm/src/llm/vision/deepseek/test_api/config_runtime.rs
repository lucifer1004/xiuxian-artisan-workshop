use std::path::Path;

use crate::llm::vision::deepseek::config::{self, DeepseekConfigSnapshot};
use crate::llm::vision::deepseek::runtime::{
    normalize_model_root,
    resolve_default_model_root_with_for_tests as runtime_resolve_default_model_root_with_for_tests,
    resolve_model_root_with,
};

/// Load `DeepSeek` config with explicit paths for test assertions.
///
/// Note: Returns crate-internal type for test support module consumption.
pub fn load_config_with_paths_for_tests(
    project_root: Option<&Path>,
    config_home: Option<&Path>,
) -> DeepseekConfigSnapshot {
    config::load_config_with_paths_for_tests(project_root, config_home)
}

/// Resolve model root with fallback chain for test assertions.
pub fn resolve_model_root_with_for_tests(
    env_model_root: Option<&str>,
    config_model_root: Option<&str>,
    default_model_root: Option<&str>,
) -> Option<String> {
    resolve_model_root_with(
        env_model_root.map(ToString::to_string),
        config_model_root.map(ToString::to_string),
        default_model_root.map(ToString::to_string),
    )
}

/// Resolve the default `DeepSeek` model root search path for test assertions.
#[must_use]
pub fn resolve_default_model_root_with_for_tests(
    cache_home: &Path,
    data_home: &Path,
) -> Option<String> {
    runtime_resolve_default_model_root_with_for_tests(cache_home, data_home)
}

/// Normalize model root path for test assertions.
pub fn normalize_model_root_for_tests(raw: &str, project_root: &Path) -> String {
    normalize_model_root(raw, project_root)
}
