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
pub async fn infer_deepseek_ocr_truth(
    runtime: &DeepseekRuntime,
    prepared: &PreparedVisionImage,
    stop_signal: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
) -> LlmResult<Option<String>> {
    eprintln!("[OCR TRACE] infer_deepseek_ocr_truth() called");
    match runtime {
        DeepseekRuntime::Disabled { .. } => {
            eprintln!("[OCR TRACE] runtime is DISABLED");
            Ok(None)
        }
        DeepseekRuntime::Configured { .. } => {
            eprintln!("[OCR TRACE] runtime is CONFIGURED, calling native::infer");
            let result = infer_enabled_runtime(runtime, prepared, stop_signal);
            eprintln!(
                "[OCR TRACE] native::infer returned, result={:?}",
                result.as_ref().map(|o| o.as_ref().map(|s| s.len()))
            );
            result
        }
        DeepseekRuntime::RemoteHttp { base_url } => {
            eprintln!("[OCR TRACE] runtime is REMOTE_HTTP, calling remote");
            super::remote_http::infer_remote_deepseek_ocr_from_bytes(
                base_url.as_ref(),
                prepared.original.as_ref(),
                "image/png", // Assuming PNG from preprocessing
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
