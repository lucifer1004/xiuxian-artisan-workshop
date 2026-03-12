use crate::llm::error::LlmResult;

use super::super::PreparedVisionImage;
use super::runtime::DeepseekRuntime;

mod runtime_lane;

use self::runtime_lane::{infer_enabled_runtime, prewarm_enabled_runtime};

/// Infer OCR markdown truth using `DeepSeek` runtime.
///
/// # Errors
///
/// Returns runtime/model loading/inference errors when `DeepSeek` OCR is enabled.
pub fn infer_deepseek_ocr_truth(
    runtime: &DeepseekRuntime,
    prepared: &PreparedVisionImage,
    stop_signal: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
) -> LlmResult<Option<String>> {
    match runtime {
        DeepseekRuntime::Disabled { .. } => Ok(None),
        DeepseekRuntime::Configured { .. } => infer_enabled_runtime(runtime, prepared, stop_signal),
    }
}

/// Preload `DeepSeek` OCR runtime assets (model/tokenizer/device state) once.
///
/// # Errors
///
/// Returns runtime/model loading errors when `DeepSeek` OCR is enabled.
pub fn prewarm_deepseek_ocr(runtime: &DeepseekRuntime) -> LlmResult<()> {
    match runtime {
        DeepseekRuntime::Disabled { .. } => Ok(()),
        DeepseekRuntime::Configured { .. } => prewarm_enabled_runtime(runtime),
    }
}
