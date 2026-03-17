use std::path::{Path, PathBuf};

use crate::llm::vision::deepseek::config;
use crate::llm::vision::deepseek::model_kind::VisionModelKind;
use crate::llm::vision::deepseek::util::non_empty_env;

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

pub(crate) fn resolve_model_root_for_kind(kind: VisionModelKind) -> Option<String> {
    let project_root = project_root();
    match kind {
        VisionModelKind::DotsOcr => non_empty_env("XIUXIAN_VISION_DOTS_MODEL_PATH")
            .map(|value: String| normalize_model_root(value.as_str(), project_root.as_path()))
            .or_else(|| {
                config::dots_model_root().map(|value: String| {
                    normalize_model_root(value.as_str(), project_root.as_path())
                })
            })
            .or_else(|| default_dots_model_root(project_root.as_path())),
        VisionModelKind::Deepseek | VisionModelKind::PaddleOcrVl => resolve_model_root(),
    }
}

fn default_dots_model_root(project_root: &Path) -> Option<String> {
    let cache_home = resolve_cache_home(project_root);
    let data_home = resolve_data_home(project_root);
    let candidates = [
        cache_home.join("models/dots-ocr"),
        data_home.join("models/dots-ocr"),
    ];
    find_existing_model_root(candidates)
}

pub(crate) fn resolve_model_root_with(
    env_model_root: Option<String>,
    config_model_root: Option<String>,
    default_model_root: Option<String>,
) -> Option<String> {
    env_model_root.or(config_model_root).or(default_model_root)
}

fn default_model_root(project_root: &Path) -> Option<String> {
    let cache_home = resolve_cache_home(project_root);
    let data_home = resolve_data_home(project_root);
    default_model_root_with(cache_home.as_path(), data_home.as_path())
}

fn default_model_root_with(cache_home: &Path, data_home: &Path) -> Option<String> {
    let candidates = [
        cache_home.join("models/deepseek-ocr"),
        cache_home.join("models/DeepSeek-OCR"),
        cache_home.join("MODELS/deepseek-ocr"),
        cache_home.join("MODELS/DeepSeek-OCR"),
        cache_home.join("models/deepseek-ocr-2"),
        cache_home.join("models/DeepSeek-OCR-2"),
        cache_home.join("MODELS/deepseek-ocr-2"),
        cache_home.join("MODELS/DeepSeek-OCR-2"),
        data_home.join("models/deepseek-ocr"),
        data_home.join("models/DeepSeek-OCR"),
        data_home.join("models/deepseek-ocr-2"),
        data_home.join("models/DeepSeek-OCR-2"),
    ];
    find_existing_model_root(candidates)
}

fn find_existing_model_root<const N: usize>(candidates: [PathBuf; N]) -> Option<String> {
    candidates.into_iter().find_map(|candidate| {
        if candidate.exists() {
            Some(candidate.to_string_lossy().to_string())
        } else {
            None
        }
    })
}

fn project_root() -> PathBuf {
    resolve_project_root()
}

fn resolve_project_root() -> PathBuf {
    std::env::var("PRJ_ROOT")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map_or_else(
            || std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            PathBuf::from,
        )
}

fn resolve_cache_home(project_root: &Path) -> PathBuf {
    std::env::var("PRJ_CACHE_HOME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map_or_else(
            || project_root.join(".cache"),
            |value| {
                let path = PathBuf::from(value);
                if path.is_absolute() {
                    path
                } else {
                    project_root.join(path)
                }
            },
        )
}

fn resolve_data_home(project_root: &Path) -> PathBuf {
    std::env::var("PRJ_DATA_HOME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map_or_else(
            || project_root.join(".data"),
            |value| {
                let path = PathBuf::from(value);
                if path.is_absolute() {
                    path
                } else {
                    project_root.join(path)
                }
            },
        )
}

pub(crate) fn normalize_model_root(raw: &str, project_root: &Path) -> String {
    let value = PathBuf::from(raw.trim());
    if value.is_absolute() {
        value.to_string_lossy().to_string()
    } else {
        project_root.join(value).to_string_lossy().to_string()
    }
}

pub(crate) fn resolve_default_model_root_with_for_tests(
    cache_home: &Path,
    data_home: &Path,
) -> Option<String> {
    default_model_root_with(cache_home, data_home)
}
