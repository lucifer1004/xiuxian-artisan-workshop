use crate::llm::error::LlmResult;

use super::runtime::DeepseekRuntime;
use crate::llm::vision::PreparedVisionImage;

mod runtime_lane;

use self::runtime_lane::{infer_enabled_runtime, prewarm_enabled_runtime};

/// Infer OCR markdown truth using `DeepSeek` runtime.
///
/// # Errors
///
/// Returns runtime/model loading/inference errors when `DeepSeek` OCR is enabled.
pub async fn infer_deepseek_ocr_truth(
    runtime: &DeepseekRuntime,
    prepared: &PreparedVisionImage,
    stop_signal: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
) -> LlmResult<Option<String>> {
    match runtime {
        DeepseekRuntime::Disabled { .. } => Ok(None),
        DeepseekRuntime::Configured { .. } => infer_enabled_runtime(runtime, prepared, stop_signal),
        DeepseekRuntime::RemoteHttp { base_url } => {
            super::remote_http::infer_remote_deepseek_ocr_from_bytes(
                base_url.as_ref(),
                prepared.original.as_ref(),
                "image/png",
            )
            .await
        }
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
        DeepseekRuntime::RemoteHttp { base_url } => {
            super::remote_http::prewarm_remote_deepseek_ocr(base_url.as_ref())
        }
    }
}
