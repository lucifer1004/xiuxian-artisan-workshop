//! `DeepSeek` OCR prepared-image cache regression tests.

use std::io::Cursor;
use std::sync::Arc;

use anyhow::Result;
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgb};
use xiuxian_llm::llm::vision::preprocess_image_for_ocr;

#[test]
fn preprocess_cache_reuses_decoded_image_for_same_bytes_and_dimension() -> Result<()> {
    let image = Arc::<[u8]>::from(build_test_png(512, 256, 31)?);

    let first = preprocess_image_for_ocr(Arc::clone(&image), 256)?;
    let second = preprocess_image_for_ocr(image, 256)?;

    assert!(Arc::ptr_eq(&first.decoded, &second.decoded));
    assert_eq!(first.width, second.width);
    assert_eq!(first.height, second.height);
    Ok(())
}

#[test]
fn preprocess_cache_separates_entries_by_requested_dimension() -> Result<()> {
    let image = Arc::<[u8]>::from(build_test_png(1024, 512, 79)?);

    let full = preprocess_image_for_ocr(Arc::clone(&image), 1024)?;
    let downscaled = preprocess_image_for_ocr(image, 256)?;

    assert!(!Arc::ptr_eq(&full.decoded, &downscaled.decoded));
    assert_ne!(
        (full.width, full.height),
        (downscaled.width, downscaled.height)
    );
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
