use std::path::{Path, PathBuf};

use super::super::config;
use super::super::util::non_empty_env;
use xiuxian_config_core::{resolve_cache_home, resolve_data_home, resolve_project_root_or_cwd};

pub(super) fn resolve_model_root() -> Option<String> {
    let project_root = project_root();
    resolve_model_root_with(
        non_empty_env("XIUXIAN_VISION_MODEL_PATH")
            .map(|value| normalize_model_root(value.as_str(), project_root.as_path())),
        config::model_root()
            .map(|value| normalize_model_root(value.as_str(), project_root.as_path())),
        default_model_root(project_root.as_path()),
    )
}

pub(crate) fn resolve_model_root_with(
    env_model_root: Option<String>,
    config_model_root: Option<String>,
    default_model_root: Option<String>,
) -> Option<String> {
    env_model_root.or(config_model_root).or(default_model_root)
}

fn default_model_root(project_root: &Path) -> Option<String> {
    let cache_home =
        resolve_cache_home(Some(project_root)).unwrap_or_else(|| project_root.join(".cache"));
    let data_home =
        resolve_data_home(Some(project_root)).unwrap_or_else(|| project_root.join(".data"));
    let candidates = [
        cache_home.join("models/deepseek-ocr-2"),
        cache_home.join("models/DeepSeek-OCR-2"),
        cache_home.join("MODELS/deepseek-ocr-2"),
        cache_home.join("MODELS/DeepSeek-OCR-2"),
        data_home.join("models/deepseek-ocr-2"),
        data_home.join("models/DeepSeek-OCR-2"),
    ];
    candidates.into_iter().find_map(|candidate| {
        if candidate.exists() {
            Some(candidate.to_string_lossy().to_string())
        } else {
            None
        }
    })
}

fn project_root() -> PathBuf {
    resolve_project_root_or_cwd()
}

pub(crate) fn normalize_model_root(raw: &str, project_root: &Path) -> String {
    let value = PathBuf::from(raw.trim());
    if value.is_absolute() {
        value.to_string_lossy().to_string()
    } else {
        project_root.join(value).to_string_lossy().to_string()
    }
}
