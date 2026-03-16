//! Minimal test to isolate memory usage during model loading.
//! This test directly uses upstream's load_dots_model without our wrapper code.

#[cfg(feature = "vision-dots-metal")]
use std::time::Instant;

#[cfg(feature = "vision-dots-metal")]
use anyhow::{Context, Result, bail};

#[cfg(feature = "vision-dots-metal")]
use candle_core::DType;

#[cfg(feature = "vision-dots-metal")]
use deepseek_ocr_core::{
    ModelKind, ModelLoadArgs,
    runtime::{DeviceKind, prepare_device_and_dtype},
};

#[cfg(feature = "vision-dots-metal")]
use deepseek_ocr_infer_dots::load_model as load_dots_model;

#[cfg(not(feature = "vision-dots-metal"))]
#[test]
fn minimal_load_requires_metal_feature() {
    eprintln!("Skipping minimal load test: enable `vision-dots-metal` feature.");
}

/// Minimal model load test - directly uses upstream load_dots_model.
#[cfg(feature = "vision-dots-metal")]
#[ignore]
#[tokio::test]
async fn test_minimal_model_load() -> Result<()> {
    let model_root = std::env::var("XIUXIAN_VISION_MODEL_PATH").unwrap_or_else(|_| {
        let current_dir = std::env::current_dir().expect("Failed to get current directory");
        let project_root = current_dir
            .ancestors()
            .nth(4)
            .expect("Failed to get project root")
            .to_string_lossy()
            .to_string();
        format!("{}/.data/models/dots-ocr", project_root)
    });

    eprintln!("[MINIMAL TEST] model_root: {}", model_root);

    let config_path = format!("{}/config.json", model_root);
    let weights_path = format!("{}/model.safetensors.index.json", model_root);
    let snapshot_path = format!("{}/dots.ocr.Q6_K.dsq", model_root);

    // Check paths exist
    for path in [&config_path, &weights_path, &snapshot_path] {
        if !std::path::Path::new(path).exists() {
            bail!("Path does not exist: {}", path);
        }
    }

    eprintln!("[MINIMAL TEST] All paths verified");

    // Parse device from env
    let device_str = std::env::var("XIUXIAN_VISION_DEVICE").unwrap_or_else(|_| "metal".to_string());
    let device_kind = match device_str.to_lowercase().as_str() {
        "cpu" => DeviceKind::Cpu,
        "metal" => DeviceKind::Metal,
        "cuda" => DeviceKind::Cuda,
        _ => DeviceKind::Metal,
    };

    eprintln!("[MINIMAL TEST] Using device: {:?}", device_kind);

    // Prepare device and dtype using upstream API
    let (device, maybe_dtype) =
        prepare_device_and_dtype(device_kind, None).context("Failed to prepare device")?;
    let dtype = maybe_dtype.unwrap_or(DType::F16);

    eprintln!("[MINIMAL TEST] Device: {:?}, dtype: {:?}", device, dtype);

    // Create model load args
    let load_args = ModelLoadArgs {
        kind: ModelKind::DotsOcr,
        config_path: Some(std::path::Path::new(&config_path)),
        weights_path: Some(std::path::Path::new(&weights_path)),
        snapshot_path: Some(std::path::Path::new(&snapshot_path)),
        device,
        dtype,
    };

    eprintln!("[MINIMAL TEST] About to call load_dots_model...");
    let started = Instant::now();

    // Directly call upstream load_dots_model
    let model = load_dots_model(load_args).context("Failed to load DotsOcr model")?;

    let elapsed = started.elapsed();
    eprintln!("[MINIMAL TEST] Model loaded in {:?}", elapsed);
    eprintln!("[MINIMAL TEST] Model kind: {:?}", model.kind());

    Ok(())
}
