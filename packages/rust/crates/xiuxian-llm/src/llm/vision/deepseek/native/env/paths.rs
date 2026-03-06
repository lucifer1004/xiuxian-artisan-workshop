use std::path::{Path, PathBuf};

use super::super::super::config;
use super::super::super::model_kind::VisionModelKind;
use super::super::super::util::{non_empty_env, sanitize_error_string};

pub(in crate::llm::vision::deepseek::native) fn resolve_weights_path(
    model_root: &Path,
    model_kind: VisionModelKind,
) -> Result<PathBuf, String> {
    let override_path = non_empty_env("XIUXIAN_VISION_WEIGHTS_PATH").or_else(config::weights_path);
    resolve_weights_path_with(model_root, model_kind, override_path.as_deref())
}

pub(crate) fn resolve_weights_path_with_for_tests(
    model_root: &Path,
    model_kind: VisionModelKind,
    override_path: Option<&str>,
) -> Result<PathBuf, String> {
    resolve_weights_path_with(model_root, model_kind, override_path)
}

fn resolve_weights_path_with(
    model_root: &Path,
    model_kind: VisionModelKind,
    override_path: Option<&str>,
) -> Result<PathBuf, String> {
    if let Some(raw_override) = override_path {
        let candidate = PathBuf::from(raw_override);
        if candidate.is_file() {
            return Ok(candidate);
        }
        if candidate.is_dir() {
            return candidate_weight_paths(candidate.as_path(), model_kind)
                .into_iter()
                .find(|path| path.is_file())
                .ok_or_else(|| {
                    sanitize_error_string(format!(
                        "XIUXIAN_VISION_WEIGHTS_PATH directory {} did not contain OCR weights for model_kind={}",
                        candidate.display(),
                        model_kind.as_str(),
                    ))
                });
        }
        if candidate.exists() {
            return Ok(candidate);
        }
        return Err(sanitize_error_string(format!(
            "XIUXIAN_VISION_WEIGHTS_PATH does not exist: {}",
            candidate.display()
        )));
    }

    candidate_weight_paths(model_root, model_kind)
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            sanitize_error_string(format!(
                "no OCR weights found under {} for model_kind={} (set XIUXIAN_VISION_WEIGHTS_PATH or llm.vision.deepseek.weights_path explicitly)",
                model_root.display(),
                model_kind.as_str(),
            ))
        })
}

pub(in crate::llm::vision::deepseek::native) fn resolve_snapshot_path(
    model_root: &Path,
) -> Option<PathBuf> {
    if let Some(explicit) = non_empty_env("XIUXIAN_VISION_SNAPSHOT_PATH")
        .or_else(config::snapshot_path)
        .map(PathBuf::from)
    {
        return Some(explicit);
    }

    auto_detect_snapshot_path(model_root)
}

pub(in crate::llm::vision::deepseek::native) fn ocr_prompt() -> Option<String> {
    non_empty_env("XIUXIAN_VISION_OCR_PROMPT").or_else(config::ocr_prompt)
}

pub(in crate::llm::vision::deepseek::native) fn cache_valkey_url() -> Option<String> {
    non_empty_env("XIUXIAN_VISION_OCR_CACHE_VALKEY_URL").or_else(config::cache_valkey_url)
}

pub(in crate::llm::vision::deepseek::native) fn cache_key_prefix() -> Option<String> {
    non_empty_env("XIUXIAN_VISION_OCR_CACHE_PREFIX").or_else(config::cache_key_prefix)
}

fn auto_detect_snapshot_path(model_root: &Path) -> Option<PathBuf> {
    let mut candidates = std::fs::read_dir(model_root)
        .ok()?
        .filter_map(|entry| entry.ok().map(|value| value.path()))
        .filter(|path| {
            path.extension()
                .and_then(|value| value.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("dsq"))
        })
        .collect::<Vec<_>>();
    candidates.sort();

    match candidates.as_slice() {
        [] => None,
        [single] => Some(single.clone()),
        _ => {
            tracing::warn!(
                event = "llm.vision.deepseek.snapshot.auto_detect_ambiguous",
                model_root = %model_root.display(),
                snapshots = ?candidates.iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
                "Multiple DSQ snapshots detected; set XIUXIAN_VISION_SNAPSHOT_PATH or llm.vision.deepseek.snapshot_path explicitly"
            );
            None
        }
    }
}

fn candidate_weight_paths(model_root: &Path, model_kind: VisionModelKind) -> Vec<PathBuf> {
    let mut candidates = match model_kind {
        VisionModelKind::Deepseek => vec![
            model_root.join("model-00001-of-000001.safetensors"),
            model_root.join("model.safetensors"),
        ],
        VisionModelKind::PaddleOcrVl => vec![
            model_root.join("PaddleOCR-VL/model.safetensors"),
            model_root.join("model-00001-of-000001.safetensors"),
            model_root.join("model.safetensors"),
        ],
        VisionModelKind::DotsOcr => vec![
            model_root.join("model.safetensors.index.json"),
            model_root.join("dots.ocr/model.safetensors.index.json"),
            model_root.join("dots-ocr.safetensors"),
            model_root.join("model-00001-of-000001.safetensors"),
            model_root.join("model.safetensors"),
        ],
    };

    if let Ok(entries) = std::fs::read_dir(model_root) {
        for path in entries.filter_map(|entry| entry.ok().map(|value| value.path())) {
            let is_safetensors = path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("safetensors"));
            let is_safetensors_index = path
                .file_name()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.ends_with(".safetensors.index.json"));
            if (is_safetensors || is_safetensors_index)
                && !candidates.iter().any(|candidate| candidate == &path)
            {
                candidates.push(path);
            }
        }
    }

    candidates
}
