//! Real Metal inference verification test using actual image and weights.
//!
//! This test is `#[ignore]` by default to prevent accidental memory exhaustion.
//! Use the capfox script to run safely with memory limits:
//!   just test-real-metal

#[cfg(feature = "vision-dots-metal")]
use std::sync::Arc;
#[cfg(feature = "vision-dots-metal")]
use std::time::Instant;

#[cfg(feature = "vision-dots-metal")]
use anyhow::{Context, Result, bail};
#[cfg(feature = "vision-dots-metal")]
use xiuxian_llm::llm::vision::{VisualRefiner, get_deepseek_runtime};

#[cfg(not(feature = "vision-dots-metal"))]
#[test]
fn real_metal_inference_requires_metal_feature() {
    eprintln!("Skipping real Metal inference test: enable `vision-dots-metal` feature.");
}

/// Real Metal inference test - requires explicit --ignored flag.
/// Use `just test-real-metal` for safe execution with memory guard.
#[cfg(feature = "vision-dots-metal")]
#[ignore]
#[tokio::test]
async fn test_real_metal_inference() -> Result<()> {
    let runtime = get_deepseek_runtime();
    eprintln!("[TEST] runtime: enabled={}", runtime.is_enabled());

    if !runtime.is_enabled() {
        bail!("DeepSeek runtime is disabled: {:?}", runtime);
    }

    // Load test image
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    let project_root = current_dir
        .ancestors()
        .nth(4)
        .context("Failed to get project root")?;
    let image_path = resolve_test_image_path(project_root)?;

    let source_png = std::fs::read(&image_path)
        .with_context(|| format!("Failed to read image from {}", image_path.display()))?;

    eprintln!("[TEST] Loaded test image: {} bytes", source_png.len());

    // Run inference
    let started = Instant::now();
    let refiner = VisualRefiner::default();
    let refinement = refiner.refine(Arc::from(source_png)).await?;
    let elapsed = started.elapsed();

    eprintln!("[TEST] Inference completed in {:?}", elapsed);

    if let Some(markdown) = refinement.ocr_truth_markdown.as_ref() {
        eprintln!("[TEST] OCR output length: {}", markdown.len());
        assert!(!markdown.is_empty(), "OCR output should not be empty");
        assert!(markdown.len() > 10, "OCR output too short");
    } else {
        bail!("OCR produced no output");
    }

    eprintln!("[TEST] ✓ Real Metal inference passed");
    Ok(())
}

#[cfg(feature = "vision-dots-metal")]
fn resolve_test_image_path(project_root: &std::path::Path) -> Result<std::path::PathBuf> {
    if let Ok(path) = std::env::var("XIUXIAN_VISION_TEST_IMAGE") {
        let path = std::path::PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
        bail!(
            "Test image configured via XIUXIAN_VISION_TEST_IMAGE does not exist: {}",
            path.display()
        );
    }

    let fallback = project_root.join(".run/tmp/ocr-smoke.png");
    if fallback.exists() {
        return Ok(fallback);
    }

    bail!("Test image not found at {}", fallback.display())
}
