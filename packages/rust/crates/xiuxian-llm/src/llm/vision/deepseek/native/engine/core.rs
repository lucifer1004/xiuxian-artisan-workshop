use std::io::{self, Write as _};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use deepseek_ocr_core::{DecodeParameters, OcrEngine, VisionSettings, render_prompt};
use image::DynamicImage;
use tokenizers::Tokenizer;

use crate::llm::error::LlmResult;
use crate::llm::vision::PreparedVisionImage;
use crate::llm::vision::deepseek::native::env::ocr_prompt;
use crate::llm::vision::deepseek::util::{internal_error, sanitize_error_string};

use super::cache_io::non_empty_markdown;
use super::image_decode::decode_engine_input_image;
use super::retry::{safe_vision_settings, should_retry_with_safe_vision};
use super::telemetry::{InferenceTelemetry, log_inference_completed};

pub(super) struct DeepseekEngine {
    pub(super) model: Mutex<Box<dyn OcrEngine>>,
    pub(super) tokenizer: Tokenizer,
    pub(super) vision: VisionSettings,
    pub(super) max_tiles: usize,
    pub(super) decode: DecodeParameters,
}

struct DecodedMarkdown {
    markdown: String,
    prompt_tokens: u64,
    response_tokens: u64,
    generated_tokens: u64,
    model_decode_ms: u128,
}

impl DeepseekEngine {
    pub(super) fn warmup_once(&self, prepared: &PreparedVisionImage) -> LlmResult<()> {
        let prompt = render_prompt("plain", "", "<image>\n<|grounding|>Warmup.")
            .map_err(|error| internal_error(format!("deepseek warmup prompt failed: {error}")))?;
        let image = decode_engine_input_image(prepared);
        let images = [image];
        let decode = warmup_decode_parameters(&self.decode);
        let vision = warmup_vision_settings(self.vision);
        tracing::info!(
            event = "llm.vision.deepseek.engine.prewarm_decode",
            max_new_tokens = decode.max_new_tokens,
            use_cache = decode.use_cache,
            base_size = vision.base_size,
            image_size = vision.image_size,
            crop_mode = vision.crop_mode,
            "DeepSeek OCR prewarm decode parameters resolved"
        );
        let model = self
            .model
            .lock()
            .map_err(|_| internal_error("deepseek model mutex poisoned during warmup"))?;
        let _ = model
            .decode(
                &self.tokenizer,
                prompt.as_str(),
                &images,
                vision,
                &decode,
                None,
            )
            .map_err(|error| internal_error(format!("deepseek warmup decode failed: {error}")))?;
        Ok(())
    }

    pub(super) fn infer_markdown(
        &self,
        prepared: &PreparedVisionImage,
        stop_signal: Option<Arc<std::sync::atomic::AtomicBool>>,
    ) -> LlmResult<Option<String>> {
        let total_started = Instant::now();
        let prompt_text = resolve_ocr_prompt_text();
        let (effective_vision, estimated_tiles) =
            resolve_effective_vision_settings(self.vision, prepared, self.max_tiles);
        tracing::debug!(
            event = "llm.vision.deepseek.infer.direct_path",
            width = prepared.width,
            height = prepared.height,
            estimated_tiles,
            input_mode = prepared.mode.as_str(),
            "DeepSeek OCR inference is using the direct single-request path"
        );
        self.infer_direct_markdown(
            prepared,
            prompt_text.as_str(),
            total_started,
            effective_vision,
            estimated_tiles,
            stop_signal,
        )
    }

    fn infer_direct_markdown(
        &self,
        prepared: &PreparedVisionImage,
        prompt_text: &str,
        total_started: Instant,
        effective_vision: VisionSettings,
        estimated_tiles: usize,
        stop_signal: Option<Arc<std::sync::atomic::AtomicBool>>,
    ) -> LlmResult<Option<String>> {
        self.log_tile_cap_override_if_applied(prepared, effective_vision, estimated_tiles);
        self.log_inference_start(prepared, effective_vision, estimated_tiles);

        let image_decode_started = Instant::now();
        let image = decode_engine_input_image(prepared);
        let image_decode_ms = image_decode_started.elapsed().as_millis();
        let images = [image];

        let prompt = render_prompt("plain", "", prompt_text)
            .map_err(|error| internal_error(format!("deepseek prompt render failed: {error}")))?;
        let decode = self.decode.clone();
        self.log_decode_budget(prepared, estimated_tiles, &decode);
        emit_stage_trace(
            "xiuxian.decode.started",
            &[
                ("use_cache", decode.use_cache.to_string()),
                ("max_new_tokens", decode.max_new_tokens.to_string()),
                ("base_size", effective_vision.base_size.to_string()),
                ("image_size", effective_vision.image_size.to_string()),
                ("crop_mode", effective_vision.crop_mode.to_string()),
                ("input_mode", prepared.mode.as_str().to_string()),
                (
                    "engine_input_bytes",
                    prepared.engine_input.len().to_string(),
                ),
            ],
        );

        let model = self
            .model
            .lock()
            .map_err(|_| internal_error("deepseek model mutex poisoned during OCR inference"))?;
        let decoded = self.decode_markdown_with_retry(
            model.as_ref(),
            prompt.as_str(),
            &images,
            effective_vision,
            &decode,
            stop_signal,
        )?;

        let telemetry = InferenceTelemetry {
            total_started,
            image_decode_ms,
            model_decode_ms: decoded.model_decode_ms,
            prompt_tokens: decoded.prompt_tokens,
            response_tokens: decoded.response_tokens,
            generated_tokens: decoded.generated_tokens,
        };
        log_inference_completed(
            prepared,
            decoded.markdown.as_str(),
            &telemetry,
            estimated_tiles,
        );

        Ok(non_empty_markdown(decoded.markdown))
    }

    fn log_tile_cap_override_if_applied(
        &self,
        prepared: &PreparedVisionImage,
        effective_vision: VisionSettings,
        estimated_tiles: usize,
    ) {
        if self.vision.crop_mode && !effective_vision.crop_mode {
            tracing::warn!(
                event = "llm.vision.deepseek.infer.tile_cap_applied",
                width = prepared.width,
                height = prepared.height,
                estimated_tiles,
                max_tiles = self.max_tiles,
                configured_image_size = self.vision.image_size,
                "DeepSeek OCR estimated tile budget exceeded max_tiles; forcing crop_mode=false for this request"
            );
        }
    }

    fn log_inference_start(
        &self,
        prepared: &PreparedVisionImage,
        effective_vision: VisionSettings,
        estimated_tiles: usize,
    ) {
        tracing::debug!(
            event = "llm.vision.deepseek.infer.start",
            width = prepared.width,
            height = prepared.height,
            input_mode = prepared.mode.as_str(),
            original_bytes = prepared.original.len(),
            engine_input_bytes = prepared.engine_input.len(),
            scale = prepared.scale,
            estimated_tiles,
            max_tiles = self.max_tiles,
            pipeline = "direct",
            configured_base_size = self.vision.base_size,
            configured_image_size = self.vision.image_size,
            configured_crop_mode = self.vision.crop_mode,
            effective_base_size = effective_vision.base_size,
            effective_image_size = effective_vision.image_size,
            effective_crop_mode = effective_vision.crop_mode,
            configured_max_new_tokens = self.decode.max_new_tokens,
            "DeepSeek OCR inference started"
        );
    }

    fn log_decode_budget(
        &self,
        prepared: &PreparedVisionImage,
        estimated_tiles: usize,
        decode: &DecodeParameters,
    ) {
        tracing::debug!(
            event = "llm.vision.deepseek.infer.decode_budget",
            width = prepared.width,
            height = prepared.height,
            estimated_tiles,
            max_tiles = self.max_tiles,
            configured_max_new_tokens = self.decode.max_new_tokens,
            effective_max_new_tokens = decode.max_new_tokens,
            "DeepSeek OCR effective decode budget resolved"
        );
    }

    fn decode_markdown_with_retry(
        &self,
        model: &dyn OcrEngine,
        prompt: &str,
        images: &[DynamicImage],
        effective_vision: VisionSettings,
        decode: &DecodeParameters,
        stop_signal: Option<Arc<std::sync::atomic::AtomicBool>>,
    ) -> LlmResult<DecodedMarkdown> {
        let stream = stop_signal.map(|signal| {
            let callback = move |_step: usize, _tokens: &[i64]| {
                assert!(
                    !signal.load(std::sync::atomic::Ordering::Acquire),
                    "deepseek_ocr_interrupted"
                );
            };
            Box::new(callback) as Box<dyn Fn(usize, &[i64])>
        });

        let decode_once = |vision| {
            model.decode(
                &self.tokenizer,
                prompt,
                images,
                vision,
                decode,
                stream.as_ref().map(|b| b.as_ref()),
            )
        };
        let model_decode_started = Instant::now();
        let outcome = match decode_once(effective_vision) {
            Ok(outcome) => outcome,
            Err(error) => {
                let error_text = error.to_string();
                if should_retry_with_safe_vision(error_text.as_str(), effective_vision) {
                    let fallback_vision = safe_vision_settings();
                    tracing::warn!(
                        event = "llm.vision.deepseek.infer.retry_safe_vision",
                        base_size = fallback_vision.base_size,
                        image_size = fallback_vision.image_size,
                        crop_mode = fallback_vision.crop_mode,
                        error = %sanitize_error_string(error_text),
                        "DeepSeek OCR decode failed on current vision settings; retrying once with safe OCR2 dimensions"
                    );
                    decode_once(fallback_vision).map_err(|retry_error| {
                        internal_error(format!(
                            "deepseek OCR decode failed after safe-vision retry: {retry_error}"
                        ))
                    })?
                } else {
                    return Err(internal_error(format!(
                        "deepseek OCR decode failed: {error}"
                    )));
                }
            }
        };
        let model_decode_ms = model_decode_started.elapsed().as_millis();
        Ok(DecodedMarkdown {
            markdown: outcome.text.trim().to_string(),
            prompt_tokens: u64::try_from(outcome.prompt_tokens).unwrap_or(u64::MAX),
            response_tokens: u64::try_from(outcome.response_tokens).unwrap_or(u64::MAX),
            generated_tokens: u64::try_from(outcome.generated_tokens.len()).unwrap_or(u64::MAX),
            model_decode_ms,
        })
    }
}

fn stage_trace_enabled() -> bool {
    std::env::var("XIUXIAN_VISION_STAGE_TRACE_STDERR")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
}

fn emit_stage_trace(stage: &str, fields: &[(&str, String)]) {
    if !stage_trace_enabled() {
        return;
    }
    let mut line = format!("[XIUXIAN STAGE] {stage}");
    for (key, value) in fields {
        line.push(' ');
        line.push_str(key);
        line.push('=');
        line.push_str(value);
    }
    eprintln!("{line}");
    let _ = io::stderr().flush();
}

fn warmup_decode_parameters(base: &DecodeParameters) -> DecodeParameters {
    let mut decode = base.clone();
    decode.max_new_tokens = 1;
    decode.use_cache = false;
    decode
}

fn warmup_vision_settings(configured: VisionSettings) -> VisionSettings {
    let safe = safe_vision_settings();
    VisionSettings {
        base_size: configured.base_size.min(safe.base_size),
        image_size: configured.image_size.min(safe.image_size),
        crop_mode: configured.crop_mode && safe.crop_mode,
    }
}

fn resolve_effective_vision_settings(
    configured: VisionSettings,
    prepared: &PreparedVisionImage,
    max_tiles: usize,
) -> (VisionSettings, usize) {
    let estimated_tiles = estimate_tile_count(
        prepared.width,
        prepared.height,
        configured.image_size,
        configured.crop_mode,
    );
    if configured.crop_mode && estimated_tiles > max_tiles {
        let mut effective = configured;
        effective.crop_mode = false;
        return (effective, estimated_tiles);
    }
    (configured, estimated_tiles)
}

#[cfg(test)]
mod tests {
    use deepseek_ocr_core::{DecodeParameters, VisionSettings};

    use super::warmup_decode_parameters;

    #[test]
    fn warmup_decode_disables_cache_and_limits_generation() {
        let base = DecodeParameters {
            max_new_tokens: 512,
            do_sample: false,
            temperature: 0.0,
            top_p: Some(0.95),
            top_k: Some(40),
            repetition_penalty: 1.2,
            no_repeat_ngram_size: Some(16),
            seed: Some(7),
            use_cache: true,
        };

        let decode = warmup_decode_parameters(&base);

        assert_eq!(decode.max_new_tokens, 1);
        assert!(!decode.use_cache);
        assert_eq!(decode.do_sample, base.do_sample);
        assert_eq!(decode.temperature, base.temperature);
        assert_eq!(decode.top_p, base.top_p);
        assert_eq!(decode.top_k, base.top_k);
        assert_eq!(decode.repetition_penalty, base.repetition_penalty);
        assert_eq!(decode.no_repeat_ngram_size, base.no_repeat_ngram_size);
        assert_eq!(decode.seed, base.seed);
    }

    #[test]
    fn warmup_vision_uses_safe_ocr_dimensions_without_upscaling_smaller_configs() {
        let configured = VisionSettings {
            base_size: 1024,
            image_size: 640,
            crop_mode: true,
        };

        let warmup = super::warmup_vision_settings(configured);

        assert_eq!(warmup.base_size, 448);
        assert_eq!(warmup.image_size, 448);
        assert!(warmup.crop_mode);

        let smaller = VisionSettings {
            base_size: 320,
            image_size: 256,
            crop_mode: false,
        };
        let warmup_smaller = super::warmup_vision_settings(smaller);
        assert_eq!(warmup_smaller.base_size, 320);
        assert_eq!(warmup_smaller.image_size, 256);
        assert!(!warmup_smaller.crop_mode);
    }
}

fn estimate_tile_count(width: u32, height: u32, image_size: u32, crop_mode: bool) -> usize {
    if !crop_mode || image_size == 0 {
        return 1;
    }
    let tiles_w = width.saturating_add(image_size.saturating_sub(1)) / image_size;
    let tiles_h = height.saturating_add(image_size.saturating_sub(1)) / image_size;
    let local_tiles_u32 = tiles_w.saturating_mul(tiles_h).max(1);
    let local_tiles = usize::try_from(local_tiles_u32).unwrap_or(usize::MAX);
    if local_tiles > 1 {
        local_tiles.saturating_add(1)
    } else {
        local_tiles
    }
}

fn resolve_ocr_prompt_text() -> String {
    ocr_prompt()
        .unwrap_or_else(|| "<image>\n<|grounding|>Convert this image to markdown.".to_string())
}
