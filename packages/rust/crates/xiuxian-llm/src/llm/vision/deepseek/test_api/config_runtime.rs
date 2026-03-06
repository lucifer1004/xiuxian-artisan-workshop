use std::path::Path;

use super::super::config::DeepseekConfigSnapshot;

pub(crate) fn load_config_with_paths_for_tests(
    project_root: Option<&Path>,
    config_home: Option<&Path>,
) -> DeepseekConfigSnapshot {
    super::super::config::load_config_with_paths_for_tests(project_root, config_home)
}

pub(crate) fn resolve_model_root_with_for_tests(
    env_model_root: Option<&str>,
    config_model_root: Option<&str>,
    default_model_root: Option<&str>,
) -> Option<String> {
    super::super::runtime::resolve_model_root_with(
        env_model_root.map(ToString::to_string),
        config_model_root.map(ToString::to_string),
        default_model_root.map(ToString::to_string),
    )
}

pub(crate) fn normalize_model_root_for_tests(raw: &str, project_root: &Path) -> String {
    super::super::runtime::normalize_model_root(raw, project_root)
}
