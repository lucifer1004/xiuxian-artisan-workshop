use std::time::Instant;

use crate::llm::vision::PreparedVisionImage;

pub(super) struct InferenceTelemetry {
    pub(super) total_started: Instant,
    pub(super) image_decode_ms: u128,
    pub(super) model_decode_ms: u128,
    pub(super) prompt_tokens: u64,
    pub(super) response_tokens: u64,
    pub(super) generated_tokens: u64,
}

pub(super) fn log_inference_completed(
    prepared: &PreparedVisionImage,
    markdown: &str,
    telemetry: &InferenceTelemetry,
    estimated_tiles: usize,
) {
    tracing::info!(
        event = "llm.vision.deepseek.infer.completed",
        width = prepared.width,
        height = prepared.height,
        estimated_tiles,
        image_decode_ms = telemetry.image_decode_ms,
        model_decode_ms = telemetry.model_decode_ms,
        total_ms = telemetry.total_started.elapsed().as_millis(),
        chars = markdown.chars().count(),
        prompt_tokens = telemetry.prompt_tokens,
        response_tokens = telemetry.response_tokens,
        generated_tokens = telemetry.generated_tokens,
        "DeepSeek OCR inference completed"
    );
}
