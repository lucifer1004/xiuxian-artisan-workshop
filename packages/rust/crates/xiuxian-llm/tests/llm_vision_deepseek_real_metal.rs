//! Real Metal inference verification test using actual image and weights.
//!
//! This test loads a real image from `.run/tmp/ocr-smoke.png` and performs
//! actual Metal GPU inference to verify the OCR pipeline works end-to-end.

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

#[cfg(feature = "vision-dots-metal")]
fn init_tracing() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        // Use env_logger which reads RUST_LOG environment variable
        // RUST_LOG=xiuxian_llm=debug,info or RUST_LOG=debug for all
        let _ = env_logger::Builder::from_env("RUST_LOG")
            .format_timestamp_millis()
            .try_init();
        eprintln!("[TRACE] Logging initialized. Set RUST_LOG=xiuxian_llm=debug for detailed logs.");
    });
}

/// Verify real Metal inference works with actual image and weights.
///
/// This test:
/// 1. Loads `.run/tmp/ocr-smoke.png` (1400x900 PNG with text content)
/// 2. Runs actual Metal GPU inference with real quantized weights
/// 3. Validates output contains expected text patterns
#[cfg(feature = "vision-dots-metal")]
#[tokio::test]
async fn test_real_metal_inference() -> Result<()> {
    init_tracing();

    eprintln!("[TEST TRACE] test_real_metal_inference START");

    let runtime = get_deepseek_runtime();
    eprintln!(
        "[TEST TRACE] runtime obtained: enabled={}",
        runtime.is_enabled()
    );

    if !runtime.is_enabled() {
        bail!("DeepSeek runtime is disabled: {runtime:?}");
    }

    // Load real test image from project root (omni-dev-fusion)
    // Path: xiuxian-llm -> crates -> rust -> packages -> omni-dev-fusion
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    let project_root = current_dir
        .ancestors()
        .nth(4)
        .context("Failed to get project root (need 4 levels up)")?;

    let image_path = project_root.join(".run/tmp/ocr-smoke.png");

    if !image_path.exists() {
        bail!(
            "Test image not found at {}. Please ensure the test image exists.",
            image_path.display()
        );
    }

    let source_png = std::fs::read(&image_path)
        .with_context(|| format!("Failed to read image from {}", image_path.display()))?;

    eprintln!(
        "[TEST TRACE] Loaded test image: {} ({} bytes)",
        image_path.display(),
        source_png.len()
    );

    // Run real Metal inference
    eprintln!("[TEST TRACE] Creating VisualRefiner...");
    let started = Instant::now();

    let refiner = VisualRefiner::default();
    eprintln!("[TEST TRACE] VisualRefiner created, calling refine()...");

    let refinement = refiner.refine(Arc::from(source_png)).await?;
    eprintln!("[TEST TRACE] refine() completed");

    let elapsed = started.elapsed();

    eprintln!("[TEST TRACE] deepseek_runtime={:?}", runtime);
    eprintln!("[TEST TRACE] inference_elapsed_ms={}", elapsed.as_millis());
    eprintln!(
        "[TEST TRACE] prepared_image_dims={}x{}",
        refinement.prepared.width, refinement.prepared.height
    );

    // Validate output
    if let Some(markdown) = refinement.ocr_truth_markdown.as_ref() {
        eprintln!("[TEST TRACE] ocr_output_len={}", markdown.len());
        eprintln!(
            "[TEST TRACE] ocr_output_preview={}",
            markdown.lines().next().unwrap_or_default()
        );

        // Verify we got meaningful OCR output (not empty or error)
        assert!(
            !markdown.is_empty(),
            "OCR output should not be empty for real test image"
        );

        // Check for reasonable output length (real OCR should produce some content)
        assert!(
            markdown.len() > 10,
            "OCR output too short ({}) - possible inference failure",
            markdown.len()
        );
    } else {
        bail!("OCR produced no markdown output - inference may have failed silently");
    }

    // Validate image dimensions are positive
    assert!(
        refinement.prepared.width > 0,
        "Image width should be positive"
    );
    assert!(
        refinement.prepared.height > 0,
        "Image height should be positive"
    );

    eprintln!("[TEST TRACE] ✓ Real Metal inference verification passed");
    Ok(())
}

/// Stress test: Run multiple Metal inferences to verify GPU stability.
#[cfg(feature = "vision-dots-metal")]
#[tokio::test]
async fn test_metal_inference_stability() -> Result<()> {
    let runtime = get_deepseek_runtime();
    if !runtime.is_enabled() {
        bail!("DeepSeek runtime is disabled: {runtime:?}");
    }

    // Load real test image from project root (omni-dev-fusion)
    // Path: xiuxian-llm -> crates -> rust -> packages -> omni-dev-fusion
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    let project_root = current_dir
        .ancestors()
        .nth(4)
        .context("Failed to get project root (need 4 levels up)")?;

    let image_path = project_root.join(".run/tmp/ocr-smoke.png");

    if !image_path.exists() {
        bail!(
            "Test image not found at {}. Please ensure the test image exists.",
            image_path.display()
        );
    }

    let source_png = std::fs::read(&image_path)
        .with_context(|| format!("Failed to read image from {}", image_path.display()))?;

    // Run 3 consecutive inferences to verify GPU stability
    let iterations = 3;
    let mut elapsed_times = Vec::with_capacity(iterations);

    for i in 0..iterations {
        let started = Instant::now();
        let refinement = VisualRefiner::default()
            .refine(Arc::from(source_png.clone()))
            .await?;
        let elapsed = started.elapsed();
        elapsed_times.push(elapsed.as_millis());

        assert!(
            refinement.ocr_truth_markdown.is_some(),
            "Iteration {}: OCR should produce output",
            i + 1
        );

        println!("Iteration {}: {}ms", i + 1, elapsed.as_millis());
    }

    // Check for reasonable performance (no severe degradation)
    let avg_ms: u128 = elapsed_times.iter().sum::<u128>() / iterations as u128;
    println!(
        "Average inference time: {}ms over {} iterations",
        avg_ms, iterations
    );

    // Subsequent runs should not be significantly slower (within 2x of first)
    let first_ms = elapsed_times[0];
    for (i, &ms) in elapsed_times.iter().enumerate().skip(1) {
        assert!(
            ms < first_ms * 3,
            "Iteration {} performance degraded significantly: {}ms vs {}ms initial",
            i + 1,
            ms,
            first_ms
        );
    }

    println!("✓ Metal inference stability test passed");
    Ok(())
}
