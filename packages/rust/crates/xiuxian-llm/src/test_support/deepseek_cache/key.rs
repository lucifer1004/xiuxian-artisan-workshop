#[cfg(feature = "vision-dots")]
use crate::llm::vision::deepseek::build_cache_key_with_for_tests as build_cache_key_from_prepared;
#[cfg(feature = "vision-dots")]
use crate::llm::vision::{PreparedVisionImage, PreparedVisionImageMode, encode_png};
#[cfg(not(feature = "vision-dots"))]
use sha2::{Digest, Sha256};
#[cfg(feature = "vision-dots")]
use std::sync::Arc;

pub struct DeepseekCacheKeyInput<'a> {
    pub model_root: &'a str,
    pub prompt: &'a str,
    pub base_size: u32,
    pub image_size: u32,
    pub crop_mode: bool,
    pub max_new_tokens: u32,
    /// Original bytes.
    pub original: &'a [u8],
}

pub(super) fn build_cache_key(input: &DeepseekCacheKeyInput<'_>) -> String {
    #[cfg(feature = "vision-dots")]
    {
        let decoded = build_cache_key_image_for_tests(input.original);
        let original_arc = Arc::from(input.original.to_vec().into_boxed_slice());
        let png = encode_png(&decoded).unwrap_or_else(|_| Arc::from(Vec::new().into_boxed_slice()));

        let prepared = PreparedVisionImage {
            mode: PreparedVisionImageMode::Preprocessed,
            original: original_arc,
            engine_input: png.clone(),
            width: decoded.width(),
            height: decoded.height(),
            scale: 1.0,
            resized_png: png.clone(),
            grayscale_png: png,
        };
        build_cache_key_from_prepared(
            input.model_root,
            &prepared,
            input.prompt,
            input.base_size,
            input.image_size,
            input.crop_mode,
            usize::try_from(input.max_new_tokens).unwrap_or(usize::MAX),
        )
    }

    #[cfg(not(feature = "vision-dots"))]
    {
        let mut hasher = Sha256::new();
        hasher.update(input.model_root.as_bytes());
        hasher.update([0]);
        hasher.update(input.prompt.as_bytes());
        hasher.update([0]);
        hasher.update(input.base_size.to_le_bytes());
        hasher.update(input.image_size.to_le_bytes());
        hasher.update([u8::from(input.crop_mode)]);
        hasher.update(input.max_new_tokens.to_le_bytes());
        hasher.update(input.original);
        format!("{:x}", hasher.finalize())
    }
}

#[cfg(feature = "vision-dots")]
fn build_cache_key_image_for_tests(original: &[u8]) -> image::DynamicImage {
    image::load_from_memory(original)
        .unwrap_or_else(|_| image::DynamicImage::ImageRgb8(image::ImageBuffer::new(1, 1)))
}
