use crate::llm::error::LlmResult;

use super::super::super::PreparedVisionImage;
use super::super::runtime::DeepseekRuntime;

#[cfg(feature = "vision-dots")]
pub(super) fn infer_enabled_runtime(
    runtime: &DeepseekRuntime,
    prepared: &PreparedVisionImage,
    stop_signal: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
) -> LlmResult<Option<String>> {
    super::super::native::infer(runtime, prepared, stop_signal)
}

#[cfg(feature = "vision-dots")]
pub(super) fn prewarm_enabled_runtime(runtime: &DeepseekRuntime) -> LlmResult<()> {
    super::super::native::prewarm(runtime)
}

#[cfg(not(feature = "vision-dots"))]
pub(super) fn infer_enabled_runtime(
    runtime: &DeepseekRuntime,
    prepared: &PreparedVisionImage,
    _stop_signal: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
) -> LlmResult<Option<String>> {
    let _ = runtime;
    let _ = prepared;
    Ok(None)
}

#[cfg(not(feature = "vision-dots"))]
pub(super) fn prewarm_enabled_runtime(runtime: &DeepseekRuntime) -> LlmResult<()> {
    let _ = runtime;
    Ok(())
}
