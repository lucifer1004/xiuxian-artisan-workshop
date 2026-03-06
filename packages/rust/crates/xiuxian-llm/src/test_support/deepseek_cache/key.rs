use std::sync::Arc;

use crate::llm::vision::PreparedVisionImage;

/// Input payload for building a `DeepSeek` OCR cache key in tests.
#[derive(Debug, Clone)]
pub struct DeepseekCacheKeyInput<'a> {
    /// Model root path segment.
    pub model_root: &'a str,
    /// OCR prompt text.
    pub prompt: &'a str,
    /// Vision base size.
    pub base_size: u32,
    /// Vision image size.
    pub image_size: u32,
    /// Crop mode switch.
    pub crop_mode: bool,
    /// Decode budget.
    pub max_new_tokens: usize,
    /// Original bytes.
    pub original: &'a [u8],
}

pub(super) fn build_cache_key(input: &DeepseekCacheKeyInput<'_>) -> String {
    let decoded = build_cache_key_image_for_tests(input.original);
    let prepared = PreparedVisionImage {
        original: Arc::from(input.original.to_vec().into_boxed_slice()),
        width: decoded.width(),
        height: decoded.height(),
        scale: 1.0,
        decoded: Arc::new(decoded),
    };
    build_cache_key_from_prepared(
        input.model_root,
        &prepared,
        input.prompt,
        input.base_size,
        input.image_size,
        input.crop_mode,
        input.max_new_tokens,
    )
}

pub(super) fn build_cache_key_from_prepared(
    model_root: &str,
    prepared: &PreparedVisionImage,
    prompt: &str,
    base_size: u32,
    image_size: u32,
    crop_mode: bool,
    max_new_tokens: usize,
) -> String {
    #[cfg(feature = "vision-dots")]
    {
        crate::llm::vision::deepseek::build_cache_key_with_for_tests(
            model_root,
            prepared,
            prompt,
            base_size,
            image_size,
            crop_mode,
            max_new_tokens,
        )
    }
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = prepared;
        format!(
            "{model_root}:{prompt}:{base_size}:{image_size}:{crop_mode}:{max_new_tokens}:{}",
            0,
            model_root = model_root,
            prompt = prompt,
            base_size = base_size,
            image_size = image_size,
            crop_mode = crop_mode,
            max_new_tokens = max_new_tokens
        )
    }
}

fn build_cache_key_image_for_tests(original: &[u8]) -> image::DynamicImage {
    if original.is_empty() {
        return image::DynamicImage::new_luma8(1, 1);
    }
    let width = u32::try_from(original.len()).unwrap_or(u32::MAX).max(1);
    if let Some(image) = image::GrayImage::from_raw(width, 1, original.to_vec()) {
        image::DynamicImage::ImageLuma8(image)
    } else {
        image::DynamicImage::new_luma8(1, 1)
    }
}
