//! Real Metal inference verification test using actual image and weights.
//!
//! This test is `#[ignore]` by default to prevent accidental memory exhaustion.
//! Use the capfox script to run safely with memory limits:
//!   just test-real-metal

#[cfg(feature = "vision-dots-metal")]
use std::sync::Arc;
#[cfg(feature = "vision-dots-metal")]
use std::sync::Once;
#[cfg(feature = "vision-dots-metal")]
use std::time::Instant;

#[cfg(feature = "vision-dots-metal")]
use anyhow::{Context, Result, bail};
#[cfg(feature = "vision-dots-metal")]
use tracing_subscriber::EnvFilter;
#[cfg(feature = "vision-dots-metal")]
use xiuxian_llm::llm::vision::deepseek::{
    load_deepseek_ocr_for_tests, prewarm_deepseek_ocr, reset_deepseek_engine_state_for_tests,
};
#[cfg(feature = "vision-dots-metal")]
use xiuxian_llm::llm::vision::{VisualRefiner, get_deepseek_runtime};

#[cfg(feature = "vision-dots-metal")]
static LOGGING_INIT: Once = Once::new();

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
            let allow_empty_output = resolve_allow_empty_output();
            let min_output_chars = resolve_min_output_chars();

            let source_png = std::fs::read(&image_path)
                .with_context(|| format!("Failed to read image from {}", image_path.display()))?;

            eprintln!("[TEST] Loaded test image: {} bytes", source_png.len());

            let refiner = VisualRefiner::default();
            let refinement = refiner.refine(Arc::from(source_png)).await?;
            let elapsed = started.elapsed();

            eprintln!("[TEST] Inference completed in {:?}", elapsed);

            if let Some(markdown) = refinement.ocr_truth_markdown.as_ref() {
                eprintln!("[TEST] OCR output length: {}", markdown.len());
                eprintln!("[TEST] OCR preview: {}", preview_text(markdown, 120));
                if allow_empty_output {
                    eprintln!("[TEST] Empty OCR output is allowed for this smoke profile");
                } else {
                    assert!(!markdown.is_empty(), "OCR output should not be empty");
                    assert!(
                        markdown.chars().count() >= min_output_chars,
                        "OCR output too short: expected at least {min_output_chars} chars, got {}",
                        markdown.chars().count()
                    );
                }
            } else {
                if allow_empty_output {
                    eprintln!("[TEST] OCR produced no output, but empty output is allowed");
                } else {
                    bail!("OCR produced no output");
                }
            }
        }
    }

    eprintln!("[TEST] ✓ Real Metal phase passed");
    Ok(())
}

#[cfg(feature = "vision-dots-metal")]
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

#[cfg(feature = "vision-dots-metal")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RealRunPhase {
    Load,
    Prewarm,
    Infer,
}

#[cfg(feature = "vision-dots-metal")]
impl RealRunPhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Load => "load",
            Self::Prewarm => "prewarm",
            Self::Infer => "infer",
        }
    }
}

#[cfg(feature = "vision-dots-metal")]
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

#[cfg(feature = "vision-dots-metal")]
fn resolve_allow_empty_output() -> bool {
    std::env::var("XIUXIAN_VISION_ALLOW_EMPTY_OUTPUT")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "yes" | "on"))
}

#[cfg(feature = "vision-dots-metal")]
fn resolve_min_output_chars() -> usize {
    parse_min_output_chars(
        std::env::var("XIUXIAN_VISION_MIN_OUTPUT_CHARS")
            .ok()
            .as_deref(),
    )
}

#[cfg(feature = "vision-dots-metal")]
fn parse_min_output_chars(raw: Option<&str>) -> usize {
    raw.and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|&value| value > 0)
        .unwrap_or(11)
}

#[cfg(feature = "vision-dots-metal")]
fn preview_text(value: &str, max_chars: usize) -> String {
    let preview = value.chars().take(max_chars).collect::<String>();
    preview
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(feature = "vision-dots-metal")]
#[test]
fn parse_min_output_chars_defaults_to_11() {
    assert_eq!(parse_min_output_chars(None), 11);
    assert_eq!(parse_min_output_chars(Some("")), 11);
    assert_eq!(parse_min_output_chars(Some("0")), 11);
    assert_eq!(parse_min_output_chars(Some("abc")), 11);
}

#[cfg(feature = "vision-dots-metal")]
#[test]
fn parse_min_output_chars_accepts_positive_override() {
    assert_eq!(parse_min_output_chars(Some("1")), 1);
    assert_eq!(parse_min_output_chars(Some("12")), 12);
}
