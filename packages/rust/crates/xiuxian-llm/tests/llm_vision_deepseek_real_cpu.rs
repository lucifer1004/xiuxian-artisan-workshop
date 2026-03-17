//! Real CPU inference verification test using actual image and weights.
//!
//! This test is `#[ignore]` by default because it loads the full OCR model.

use std::sync::Arc;
use std::sync::Once;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use tracing_subscriber::EnvFilter;
use xiuxian_llm::llm::vision::deepseek::{
    load_deepseek_ocr_for_tests, prewarm_deepseek_ocr, reset_deepseek_engine_state_for_tests,
};
use xiuxian_llm::llm::vision::{VisualRefiner, get_deepseek_runtime};

static LOGGING_INIT: Once = Once::new();

#[ignore]
#[tokio::test]
async fn test_real_cpu_inference() -> Result<()> {
    init_test_logging();
    let runtime = get_deepseek_runtime();
    eprintln!("[TEST] runtime: enabled={}", runtime.is_enabled());

    if !runtime.is_enabled() {
        bail!("DeepSeek runtime is disabled: {:?}", runtime);
    }

    reset_deepseek_engine_state_for_tests();
    let phase = resolve_real_run_phase()?;
    eprintln!("[TEST] phase={}", phase.as_str());
    let started = Instant::now();
    match phase {
        RealRunPhase::Load => {
            load_deepseek_ocr_for_tests(runtime.as_ref())?;
            eprintln!("[TEST] Load-only completed in {:?}", started.elapsed());
        }
        RealRunPhase::Prewarm => {
            prewarm_deepseek_ocr(runtime.as_ref())?;
            eprintln!("[TEST] Prewarm completed in {:?}", started.elapsed());
        }
        RealRunPhase::Infer => {
            let current_dir = std::env::current_dir().context("Failed to get current directory")?;
            let project_root = current_dir
                .ancestors()
                .nth(4)
                .context("Failed to get project root")?;
            let image_path = resolve_test_image_path(project_root)?;

            let source_png = std::fs::read(&image_path)
                .with_context(|| format!("Failed to read image from {}", image_path.display()))?;

            eprintln!("[TEST] Loaded test image: {} bytes", source_png.len());

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
        }
    }

    eprintln!("[TEST] ✓ Real CPU phase passed");
    Ok(())
}

fn init_test_logging() {
    LOGGING_INIT.call_once(|| {
        let filter = std::env::var("RUST_LOG")
            .unwrap_or_else(|_| "xiuxian_llm::llm::vision::deepseek=info".to_string());
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::new(filter))
            .with_test_writer()
            .try_init();
    });
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RealRunPhase {
    Load,
    Prewarm,
    Infer,
}

impl RealRunPhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Load => "load",
            Self::Prewarm => "prewarm",
            Self::Infer => "infer",
        }
    }
}

fn resolve_real_run_phase() -> Result<RealRunPhase> {
    let Some(raw) = std::env::var_os("XIUXIAN_VISION_REAL_PHASE") else {
        return Ok(RealRunPhase::Infer);
    };
    match raw.to_string_lossy().trim().to_ascii_lowercase().as_str() {
        "" | "infer" => Ok(RealRunPhase::Infer),
        "load" => Ok(RealRunPhase::Load),
        "prewarm" => Ok(RealRunPhase::Prewarm),
        other => bail!("Unsupported XIUXIAN_VISION_REAL_PHASE: {other}"),
    }
}
