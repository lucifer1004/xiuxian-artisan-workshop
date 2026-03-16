use std::path::Path;

use crate::llm::vision::deepseek::config::{self, DeepseekConfigSnapshot};
use crate::llm::vision::deepseek::runtime;

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
    runtime::resolve_model_root_with(
        env_model_root.map(ToString::to_string),
        config_model_root.map(ToString::to_string),
        default_model_root.map(ToString::to_string),
    )
}

/// Normalize model root path for test assertions.
pub fn normalize_model_root_for_tests(raw: &str, project_root: &Path) -> String {
    runtime::normalize_model_root(raw, project_root)
}
