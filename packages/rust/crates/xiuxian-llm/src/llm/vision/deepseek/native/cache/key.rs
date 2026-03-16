use sha2::{Digest, Sha256};

use crate::llm::vision::PreparedVisionImage;

pub(in crate::llm::vision::deepseek) fn build_cache_key(
    model_root: &str,
    prepared: &PreparedVisionImage,
    prompt: &str,
    base_size: u32,
    image_size: u32,
    crop_mode: bool,
    max_new_tokens: usize,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(model_root.as_bytes());
    hasher.update(prompt.as_bytes());
    hasher.update(base_size.to_le_bytes());
    hasher.update(image_size.to_le_bytes());
    hasher.update([u8::from(crop_mode)]);
    hasher.update(max_new_tokens.to_le_bytes());
    hasher.update(prepared.resized_png.as_ref());
    hasher.update(prepared.grayscale_png.as_ref());
    format!("vision:deepseek:ocr:{}", hex::encode(hasher.finalize()))
}
