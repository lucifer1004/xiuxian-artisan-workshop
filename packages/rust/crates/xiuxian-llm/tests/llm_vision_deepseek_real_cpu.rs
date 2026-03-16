//! Real CPU inference verification test using actual image and weights.
//!
//! This test is `#[ignore]` by default because it loads the full OCR model.

use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use xiuxian_llm::llm::vision::{VisualRefiner, get_deepseek_runtime};

#[ignore]
#[tokio::test]
async fn test_real_cpu_inference() -> Result<()> {
    let runtime = get_deepseek_runtime();
    eprintln!("[TEST] runtime: enabled={}", runtime.is_enabled());

    if !runtime.is_enabled() {
        bail!("DeepSeek runtime is disabled: {:?}", runtime);
    }

    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    let project_root = current_dir
        .ancestors()
        .nth(4)
        .context("Failed to get project root")?;
    let image_path = resolve_test_image_path(project_root)?;

    let source_png = std::fs::read(&image_path)
        .with_context(|| format!("Failed to read image from {}", image_path.display()))?;

    eprintln!("[TEST] Loaded test image: {} bytes", source_png.len());

    let started = Instant::now();
    let refinement = VisualRefiner::default()
        .refine(Arc::from(source_png))
        .await?;
    let elapsed = started.elapsed();

    eprintln!("[TEST] Inference completed in {:?}", elapsed);

    if let Some(markdown) = refinement.ocr_truth_markdown.as_ref() {
        eprintln!("[TEST] OCR output length: {}", markdown.len());
        assert!(!markdown.is_empty(), "OCR output should not be empty");
        assert!(markdown.len() > 10, "OCR output too short");
    } else {
        bail!("OCR produced no output");
    }

    eprintln!("[TEST] ✓ Real CPU inference passed");
    Ok(())
}

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
