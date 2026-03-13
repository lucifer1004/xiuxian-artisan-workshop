use crate::llm::vision::deepseek::build_cache_key_with_for_tests as build_cache_key_from_prepared;
use crate::llm::vision::{PreparedVisionImage, encode_png};
use image::GenericImageView;
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
    let decoded = build_cache_key_image_for_tests(input.original);
    let original_arc = Arc::from(input.original.to_vec().into_boxed_slice());
    let png = encode_png(&decoded).unwrap();

    let prepared = PreparedVisionImage {
        original: original_arc,
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
        usize::try_from(input.max_new_tokens).unwrap(),
    )
}

fn build_cache_key_image_for_tests(original: &[u8]) -> image::DynamicImage {
    image::load_from_memory(original)
        .unwrap_or_else(|_| image::DynamicImage::ImageRgb8(image::ImageBuffer::new(1, 1)))
}
