use std::path::{Path, PathBuf};

use super::super::config;
use super::super::model_kind::VisionModelKind;
use super::super::util::non_empty_env;
use xiuxian_config_core::{resolve_cache_home, resolve_data_home, resolve_project_root_or_cwd};

pub(super) fn resolve_model_root() -> Option<String> {
    let model_kind = resolve_model_kind();
    if let Some(model_root) = resolve_model_root_for_kind(model_kind) {
        return Some(model_root);
    }
    if model_kind == VisionModelKind::Deepseek {
        let fallback = resolve_model_root_for_kind(VisionModelKind::DotsOcr);
        if fallback.is_some() {
            tracing::info!(
                event = "llm.vision.deepseek.runtime.model_kind_fallback",
                requested = VisionModelKind::Deepseek.as_str(),
                fallback = VisionModelKind::DotsOcr.as_str(),
                "Configured DeepSeek model root was not found; falling back to Dots OCR model root"
            );
        }
        return fallback;
    }
    None
}

pub(crate) fn resolve_model_root_for_kind(model_kind: VisionModelKind) -> Option<String> {
    let project_root = project_root();
    resolve_model_root_with(
        env_model_root_for_kind(model_kind, project_root.as_path()),
        config_model_root_for_kind(model_kind, project_root.as_path()),
        default_model_root(project_root.as_path(), model_kind),
    )
}

pub(crate) fn resolve_model_root_with(
    env_model_root: Option<String>,
    config_model_root: Option<String>,
    default_model_root: Option<String>,
) -> Option<String> {
    env_model_root.or(config_model_root).or(default_model_root)
}

fn default_model_root(project_root: &Path, model_kind: VisionModelKind) -> Option<String> {
    let cache_home =
        resolve_cache_home(Some(project_root)).unwrap_or_else(|| project_root.join(".cache"));
    let data_home =
        resolve_data_home(Some(project_root)).unwrap_or_else(|| project_root.join(".data"));
    let root_dirs = [
        cache_home.join("models"),
        cache_home.join("MODELS"),
        data_home.join("models"),
        data_home.join("MODELS"),
    ];

    root_dirs.into_iter().find_map(|root| {
        model_dir_candidates(model_kind)
            .iter()
            .find_map(|relative| {
                let candidate = root.join(relative);
                if candidate.exists() {
                    Some(candidate.to_string_lossy().to_string())
                } else {
                    None
                }
            })
    })
}

fn resolve_model_kind() -> VisionModelKind {
    let configured = non_empty_env("XIUXIAN_VISION_MODEL_KIND").or_else(config::model_kind);
    if configured
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("auto"))
    {
        return VisionModelKind::DEFAULT;
    }
    if let Some(kind) = configured.as_deref().and_then(VisionModelKind::parse) {
        kind
    } else {
        if let Some(raw) = configured {
            tracing::warn!(
                event = "llm.vision.deepseek.runtime.invalid_model_kind",
                model_kind = %raw,
                fallback = VisionModelKind::DEFAULT.as_str(),
                "Unknown XIUXIAN_VISION_MODEL_KIND or llm.vision.deepseek.model_kind; falling back to default model kind"
            );
        }
        VisionModelKind::DEFAULT
    }
}

fn env_model_root_for_kind(model_kind: VisionModelKind, project_root: &Path) -> Option<String> {
    let value = match model_kind {
        VisionModelKind::Deepseek => non_empty_env("XIUXIAN_VISION_MODEL_PATH"),
        VisionModelKind::DotsOcr => non_empty_env("XIUXIAN_VISION_DOTS_MODEL_PATH")
            .or_else(|| non_empty_env("XIUXIAN_VISION_MODEL_PATH")),
        VisionModelKind::PaddleOcrVl => non_empty_env("XIUXIAN_VISION_PADDLE_MODEL_PATH")
            .or_else(|| non_empty_env("XIUXIAN_VISION_MODEL_PATH")),
    };
    value.map(|path| normalize_model_root(path.as_str(), project_root))
}

fn config_model_root_for_kind(model_kind: VisionModelKind, project_root: &Path) -> Option<String> {
    let value = match model_kind {
        VisionModelKind::DotsOcr => config::dots_model_root().or_else(config::model_root),
        VisionModelKind::Deepseek | VisionModelKind::PaddleOcrVl => config::model_root(),
    };
    value.map(|path| normalize_model_root(path.as_str(), project_root))
}

fn model_dir_candidates(model_kind: VisionModelKind) -> &'static [&'static str] {
    match model_kind {
        VisionModelKind::Deepseek => &["deepseek-ocr-2", "DeepSeek-OCR-2"],
        VisionModelKind::PaddleOcrVl => &["paddleocr-vl", "PaddleOCR-VL", "paddle-ocr-vl"],
        VisionModelKind::DotsOcr => &["dots-ocr", "DotsOCR", "dotsocr"],
    }
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
