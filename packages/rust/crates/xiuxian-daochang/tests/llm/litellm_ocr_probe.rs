use std::env;
use std::fs;
use std::path::Path;
use std::sync::Once;
use std::time::Instant;

use tracing_subscriber::EnvFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use xiuxian_daochang::test_support::infer_deepseek_ocr_truth_from_image_bytes;

static PROBE_TRACING_INIT: Once = Once::new();

#[tokio::test]
#[ignore = "manual probe: set XIUXIAN_OCR_PROBE_IMAGE=/path/to/image.png and run via nextest"]
async fn litellm_ocr_probe_single_image() {
    init_probe_tracing();
    let image_path = env::var("XIUXIAN_OCR_PROBE_IMAGE").unwrap_or_else(|_| {
        panic!("XIUXIAN_OCR_PROBE_IMAGE is required for litellm_ocr_probe_single_image")
    });
    let media_type = env::var("XIUXIAN_OCR_PROBE_MEDIA_TYPE")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| infer_media_type(Path::new(image_path.as_str())));

    let image_bytes = fs::read(image_path.as_str())
        .unwrap_or_else(|error| panic!("failed to read OCR probe image {image_path}: {error}"));
    let started = Instant::now();
    let markdown =
        infer_deepseek_ocr_truth_from_image_bytes(image_bytes, media_type.as_str()).await;
    let elapsed_ms = started.elapsed().as_millis();

    match markdown {
        Some(text) if !text.trim().is_empty() => {
            println!("[ocr-probe] elapsed_ms={elapsed_ms}");
            println!("[ocr-probe] media_type={media_type}");
            println!("[ocr-probe] markdown_start");
            println!("{text}");
            println!("[ocr-probe] markdown_end");
        }
        _ => {
            panic!(
                "OCR probe returned empty result (elapsed_ms={elapsed_ms}, media_type={media_type})"
            );
        }
    }
}

fn init_probe_tracing() {
    PROBE_TRACING_INIT.call_once(|| {
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("xiuxian_daochang::llm::compat::litellm_ocr=info"));
        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_target(true)
                    .with_test_writer(),
            )
            .try_init();
    });
}

fn infer_media_type(path: &Path) -> String {
    match path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
    {
        Some(ext) if ext == "png" => "image/png".to_string(),
        Some(ext) if ext == "jpg" || ext == "jpeg" => "image/jpeg".to_string(),
        Some(ext) if ext == "webp" => "image/webp".to_string(),
        Some(ext) if ext == "bmp" => "image/bmp".to_string(),
        _ => "image/png".to_string(),
    }
}
