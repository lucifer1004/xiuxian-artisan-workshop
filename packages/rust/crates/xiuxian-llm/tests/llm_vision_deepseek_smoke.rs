//! Real-model OCR smoke test.

#[cfg(any(feature = "vision-dots-metal", feature = "vision-dots-cuda"))]
use std::io::Cursor;
#[cfg(any(feature = "vision-dots-metal", feature = "vision-dots-cuda"))]
use std::sync::Arc;
#[cfg(any(feature = "vision-dots-metal", feature = "vision-dots-cuda"))]
use std::time::Instant;
#[cfg(any(feature = "vision-dots-metal", feature = "vision-dots-cuda"))]
use tokio;

#[cfg(any(feature = "vision-dots-metal", feature = "vision-dots-cuda"))]
use anyhow::{Result, bail};
#[cfg(any(feature = "vision-dots-metal", feature = "vision-dots-cuda"))]
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgb, RgbImage};
#[cfg(any(feature = "vision-dots-metal", feature = "vision-dots-cuda"))]
use xiuxian_llm::llm::vision::{VisualRefiner, get_deepseek_runtime};

#[cfg(not(any(feature = "vision-dots-metal", feature = "vision-dots-cuda")))]
#[test]
fn deepseek_smoke_requires_acceleration_feature() {
    eprintln!(
        "Skipping real OCR smoke: enable `vision-dots-metal` or `vision-dots-cuda` features."
    );
}

#[cfg(any(feature = "vision-dots-metal", feature = "vision-dots-cuda"))]
#[tokio::test]
async fn deepseek_smoke_runs_real_inference_from_local_model_cache() -> Result<()> {
    let runtime = get_deepseek_runtime();
    if !runtime.is_enabled() {
        bail!("DeepSeek runtime is disabled: {runtime:?}");
    }

    let source_png = build_high_contrast_probe_png()?;
    let started = Instant::now();
    let refinement = VisualRefiner::default()
        .refine(Arc::from(source_png))
        .await?;
    let elapsed = started.elapsed();

    println!("deepseek_runtime={runtime:?}");
    println!(
        "ocr_elapsed_ms={} ocr_truth_len={}",
        elapsed.as_millis(),
        refinement
            .ocr_truth_markdown
            .as_ref()
            .map_or(0, String::len)
    );
    if let Some(markdown) = refinement.ocr_truth_markdown.as_ref() {
        println!(
            "ocr_truth_preview={}",
            markdown.lines().next().unwrap_or_default()
        );
    }

    assert!(refinement.prepared.width > 0);
    assert!(refinement.prepared.height > 0);
    Ok(())
}

#[cfg(any(feature = "vision-dots-metal", feature = "vision-dots-cuda"))]
fn build_high_contrast_probe_png() -> Result<Vec<u8>> {
    let width = 1_024;
    let height = 640;
    let mut image: RgbImage = ImageBuffer::from_pixel(width, height, Rgb([255, 255, 255]));

    // Draw strong black bands to produce OCR-friendly structure-like regions.
    for y in 120..160 {
        for x in 120..900 {
            image.put_pixel(x, y, Rgb([0, 0, 0]));
        }
    }
    for y in 260..300 {
        for x in 120..760 {
            image.put_pixel(x, y, Rgb([0, 0, 0]));
        }
    }
    for y in 400..520 {
        for x in 120..980 {
            image.put_pixel(x, y, Rgb([0, 0, 0]));
        }
    }

    let mut cursor = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(image).write_to(&mut cursor, ImageFormat::Png)?;
    Ok(cursor.into_inner())
}
