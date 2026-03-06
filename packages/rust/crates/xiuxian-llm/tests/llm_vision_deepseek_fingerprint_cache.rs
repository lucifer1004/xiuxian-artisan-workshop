//! `DeepSeek` prepared-image fingerprint cache regression tests.

use std::io::Cursor;
use std::sync::Arc;

use anyhow::Result;
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgb};
use xiuxian_llm::llm::vision::preprocess_image_for_ocr;
use xiuxian_llm::test_support::{
    build_deepseek_cache_key_from_prepared_for_tests, deepseek_fingerprint_cache_clear_for_tests,
    deepseek_fingerprint_cache_len_for_tests,
};

const MODEL_ROOT: &str = "model-root";
const PROMPT: &str = "prompt-a";

#[test]
fn deepseek_cache_key_reuses_prepared_fingerprint_for_cloned_prepared_images() -> Result<()> {
    deepseek_fingerprint_cache_clear_for_tests();
    let image = Arc::<[u8]>::from(build_test_png(768, 384, 17)?);
    let prepared = preprocess_image_for_ocr(image, 384)?;

    let first_key = build_deepseek_cache_key_from_prepared_for_tests(
        MODEL_ROOT, &prepared, PROMPT, 448, 448, true, 512,
    );
    assert_eq!(deepseek_fingerprint_cache_len_for_tests(), 1);

    let cloned = prepared.clone();
    let second_key = build_deepseek_cache_key_from_prepared_for_tests(
        MODEL_ROOT, &cloned, PROMPT, 448, 448, true, 512,
    );
    assert_eq!(deepseek_fingerprint_cache_len_for_tests(), 1);
    assert_eq!(first_key, second_key);
    Ok(())
}

fn build_test_png(width: u32, height: u32, seed: u8) -> Result<Vec<u8>> {
    let image = ImageBuffer::from_fn(width, height, |x, y| {
        let red = seed.wrapping_add((x % 251) as u8);
        let green = seed.wrapping_add((y % 241) as u8);
        let blue = seed.wrapping_add(((x + y) % 239) as u8);
        Rgb([red, green, blue])
    });
    let mut cursor = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(image).write_to(&mut cursor, ImageFormat::Png)?;
    Ok(cursor.into_inner())
}
