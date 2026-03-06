use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use deepseek_ocr_core::{DecodeParameters, OcrEngine, VisionSettings, render_prompt};
use image::DynamicImage;
use tokenizers::Tokenizer;

use super::super::super::super::preprocess::PreparedVisionImage;
use super::super::super::util::{internal_error, sanitize_error_string};
use super::super::cache::build_cache_key;
use super::super::env::{ocr_prompt, parse_env_u64};
use crate::llm::error::LlmResult;

use super::cache_io::{CacheLayer, non_empty_markdown, read_cache_entry, store_markdown_in_cache};
use super::coalescer::{CoalesceAcquire, SharedCoalescedResult, acquire as acquire_coalesced};
use super::image_decode::decode_engine_input_image;
use super::retry::{safe_vision_settings, should_retry_with_safe_vision};
use super::telemetry::{InferenceTelemetry, log_inference_completed};

pub(super) struct DeepseekEngine {
    pub(super) model: Mutex<Box<dyn OcrEngine>>,
    pub(super) tokenizer: Tokenizer,
    pub(super) vision: VisionSettings,
    pub(super) max_tiles: usize,
    pub(super) decode: DecodeParameters,
    pub(super) model_root: Arc<str>,
}

struct DecodedMarkdown {
    markdown: String,
    prompt_tokens: u64,
    response_tokens: u64,
    generated_tokens: u64,
    model_decode_ms: u128,
}

const DEFAULT_BATCH_WINDOW_MS: u64 = 50;
const DEFAULT_INFLIGHT_WAIT_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_INFLIGHT_STALE_MS: u64 = 120_000;

impl DeepseekEngine {
    pub(super) fn warmup_once(&self, prepared: &PreparedVisionImage) -> LlmResult<()> {
        let prompt = render_prompt("plain", "", "<image>\n<|grounding|>Warmup.")
            .map_err(|error| internal_error(format!("deepseek warmup prompt failed: {error}")))?;
        let image = decode_engine_input_image(prepared);
        let images = [image];
        let mut decode = self.decode.clone();
        decode.max_new_tokens = 1;
        let model = self
            .model
            .lock()
            .map_err(|_| internal_error("deepseek model mutex poisoned during warmup"))?;
        let _ = model
            .decode(
                &self.tokenizer,
                prompt.as_str(),
                &images,
                self.vision,
                &decode,
                None,
            )
            .map_err(|error| internal_error(format!("deepseek warmup decode failed: {error}")))?;
        Ok(())
    }

    pub(super) fn infer_markdown(
        &self,
        prepared: &PreparedVisionImage,
    ) -> LlmResult<Option<String>> {
        let total_started = Instant::now();
        let prompt_text = resolve_ocr_prompt_text();
        let (effective_vision, estimated_tiles) =
            resolve_effective_vision_settings(self.vision, prepared, self.max_tiles);
        let cache_key = build_cache_key(
            self.model_root.as_ref(),
            prepared,
            prompt_text.as_str(),
            effective_vision.base_size,
            effective_vision.image_size,
            effective_vision.crop_mode,
            self.decode.max_new_tokens,
        );

        if let Some(markdown) =
            Self::try_read_cached_markdown(cache_key.as_str(), prepared, total_started)
        {
            return Ok(Some(markdown));
        }

        let batch_window = ocr_batch_window();
        let inflight_wait_timeout = ocr_inflight_wait_timeout();
        let inflight_stale_timeout = ocr_inflight_stale_timeout(inflight_wait_timeout);
        let mut leader = None;
        match acquire_coalesced(cache_key.as_str(), inflight_stale_timeout) {
            CoalesceAcquire::Follower(follower) => {
                if let Some(shared) = follower.wait(inflight_wait_timeout) {
                    tracing::debug!(
                        event = "llm.vision.deepseek.infer.coalesce_hit",
                        key_len = cache_key.len(),
                        wait_timeout_ms = inflight_wait_timeout.as_millis(),
                        "DeepSeek OCR coalescer follower reused in-flight inference result"
                    );
                    return llm_result_from_shared(shared);
                }
                tracing::warn!(
                    event = "llm.vision.deepseek.infer.coalesce_timeout",
                    key_len = cache_key.len(),
                    wait_timeout_ms = inflight_wait_timeout.as_millis(),
                    "DeepSeek OCR coalescer follower wait timed out; falling back to direct inference"
                );
            }
            CoalesceAcquire::Leader(permit) => {
                leader = Some(permit);
            }
        }

        if let Some(permit) = leader {
            let followers = permit.follower_count();
            if followers > 0 && !batch_window.is_zero() {
                tracing::debug!(
                    event = "llm.vision.deepseek.infer.coalesce_window",
                    followers,
                    batch_window_ms = batch_window.as_millis(),
                    "DeepSeek OCR coalescer leader waiting micro-batch window"
                );
                std::thread::sleep(batch_window);
            }

            let result = self.infer_uncached_markdown(
                cache_key.as_str(),
                prepared,
                prompt_text.as_str(),
                total_started,
                effective_vision,
                estimated_tiles,
            );
            permit.complete(shared_from_llm_result(&result));
            return result;
        }

        self.infer_uncached_markdown(
            cache_key.as_str(),
            prepared,
            prompt_text.as_str(),
            total_started,
            effective_vision,
            estimated_tiles,
        )
    }

    fn try_read_cached_markdown(
        cache_key: &str,
        prepared: &PreparedVisionImage,
        total_started: Instant,
    ) -> Option<String> {
        if let Some(markdown) =
            read_cache_entry(CacheLayer::Local, cache_key, prepared, total_started)
        {
            return Some(markdown);
        }

        if let Some(markdown) =
            read_cache_entry(CacheLayer::Valkey, cache_key, prepared, total_started)
        {
            return Some(markdown);
        }

        None
    }

    fn infer_uncached_markdown(
        &self,
        cache_key: &str,
        prepared: &PreparedVisionImage,
        prompt_text: &str,
        total_started: Instant,
        effective_vision: VisionSettings,
        estimated_tiles: usize,
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
        )?;

        store_markdown_in_cache(cache_key, decoded.markdown.as_str());
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
            decoded_pixels_bytes = prepared.decoded.as_bytes().len(),
            scale = prepared.scale,
            estimated_tiles,
            max_tiles = self.max_tiles,
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
    ) -> LlmResult<DecodedMarkdown> {
        let decode_once =
            |vision| model.decode(&self.tokenizer, prompt, images, vision, decode, None);
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

fn ocr_batch_window() -> Duration {
    Duration::from_millis(
        parse_env_u64("XIUXIAN_VISION_OCR_BATCH_WINDOW_MS").unwrap_or(DEFAULT_BATCH_WINDOW_MS),
    )
}

fn ocr_inflight_wait_timeout() -> Duration {
    Duration::from_millis(
        parse_env_u64("XIUXIAN_VISION_OCR_INFLIGHT_WAIT_TIMEOUT_MS")
            .unwrap_or(DEFAULT_INFLIGHT_WAIT_TIMEOUT_MS)
            .max(1),
    )
}

fn ocr_inflight_stale_timeout(wait_timeout: Duration) -> Duration {
    let stale_ms = parse_env_u64("XIUXIAN_VISION_OCR_INFLIGHT_STALE_MS")
        .unwrap_or(DEFAULT_INFLIGHT_STALE_MS)
        .max(u64::try_from(wait_timeout.as_millis()).unwrap_or(u64::MAX));
    Duration::from_millis(stale_ms)
}

fn shared_from_llm_result(result: &LlmResult<Option<String>>) -> SharedCoalescedResult {
    match result {
        Ok(Some(value)) => Ok(Some(Arc::from(value.clone()))),
        Ok(None) => Ok(None),
        Err(error) => Err(Arc::from(error.to_string())),
    }
}

fn llm_result_from_shared(shared: SharedCoalescedResult) -> LlmResult<Option<String>> {
    match shared {
        Ok(Some(value)) => Ok(Some(value.as_ref().to_string())),
        Ok(None) => Ok(None),
        Err(error) => Err(internal_error(format!(
            "deepseek OCR coalesced inference failed: {}",
            error.as_ref()
        ))),
    }
}
