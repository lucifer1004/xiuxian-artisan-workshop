use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use candle_core::DType;
use deepseek_ocr_core::{
    DecodeParameters, ModelLoadArgs, OcrEngine, VisionSettings,
    runtime::{DeviceKind, default_dtype_for_device, prepare_device_and_dtype},
};
use deepseek_ocr_dsq::{DsqReader, DsqTensorDType};
use deepseek_ocr_infer_deepseek::load_model as load_deepseek_model;
use deepseek_ocr_infer_dots::load_model as load_dots_model;
use deepseek_ocr_infer_paddleocr::load_model as load_paddleocr_model;
use tokenizers::Tokenizer;

use super::super::super::dsq_alignment::required_qoffset_alignment;
use super::super::super::model_kind::VisionModelKind;
use super::super::super::util::sanitize_error_string;
use super::super::env::{
    parse_device_kind, parse_env_bool, parse_env_f32, parse_env_f64, parse_env_string,
    parse_env_u32, parse_env_usize, resolve_snapshot_path, resolve_weights_path,
};
use super::retry::panic_payload_to_string;

use super::core::DeepseekEngine;
use super::dsq_repair::{DsqRepairResult, repair_dsq_if_needed};

impl DeepseekEngine {
    pub(super) fn load(model_root: &str) -> Result<Self, String> {
        Self::load_for_device(model_root, parse_device_kind())
    }

    pub(super) fn load_for_device(
        model_root: &str,
        requested_device: DeviceKind,
    ) -> Result<Self, String> {
        let model_kind = resolve_model_kind_for_model_root(model_root);
        Self::load_for_device_with_kind(model_root, requested_device, model_kind)
    }

    pub(super) fn load_with_kind(
        model_root: &str,
        model_kind: VisionModelKind,
    ) -> Result<Self, String> {
        Self::load_for_device_with_kind(model_root, parse_device_kind(), model_kind)
    }

    pub(super) fn load_for_device_with_kind(
        model_root: &str,
        requested_device: DeviceKind,
        model_kind: VisionModelKind,
    ) -> Result<Self, String> {
        let model_paths = resolve_model_paths(model_root, model_kind)?;
        if model_paths.require_quantized && model_paths.snapshot_path.is_none() {
            return Err(
                "DeepSeek OCR quantized snapshot is required but none was found. \
Set XIUXIAN_VISION_SNAPSHOT_PATH (or place exactly one .dsq file under model root), \
or set XIUXIAN_VISION_REQUIRE_QUANTIZED=0 to allow unquantized loading."
                    .to_string(),
            );
        }
        let (device, maybe_dtype) = prepare_device_with_fallback(requested_device)?;
        let preferred_dtype = preferred_dtype_for_device(&device);
        let fallback_dtype = maybe_dtype.unwrap_or_else(|| default_dtype_for_device(&device));
        let (model, dtype) = match load_model_with_dtype(
            model_paths.config_path.as_path(),
            model_paths.weights_path.as_path(),
            model_paths.snapshot_path.as_deref(),
            &device,
            preferred_dtype,
            model_paths.model_kind,
        ) {
            Ok(model) => (model, preferred_dtype),
            Err(error) if preferred_dtype != fallback_dtype => {
                tracing::warn!(
                    event = "llm.vision.deepseek.engine.dtype_fallback",
                    preferred = ?preferred_dtype,
                    fallback = ?fallback_dtype,
                    error = %sanitize_error_string(error),
                    "DeepSeek OCR preferred dtype load failed; retrying with backend fallback dtype"
                );
                let model = load_model_with_dtype(
                    model_paths.config_path.as_path(),
                    model_paths.weights_path.as_path(),
                    model_paths.snapshot_path.as_deref(),
                    &device,
                    fallback_dtype,
                    model_paths.model_kind,
                )?;
                (model, fallback_dtype)
            }
            Err(error) => return Err(sanitize_error_string(error)),
        };

        tracing::info!(
            event = "llm.vision.deepseek.engine.device_selected",
            device = ?device,
            dtype = ?dtype,
            preferred_dtype = ?preferred_dtype,
            "DeepSeek OCR engine selected execution device and dtype"
        );

        let tokenizer =
            Tokenizer::from_file(model_paths.tokenizer_path).map_err(sanitize_error_string)?;

        tracing::info!(
            event = "llm.vision.deepseek.engine.loaded",
            model_root = %model_root,
            model_kind = %model_paths.model_kind.as_str(),
            weights_path = %model_paths.weights_path.display(),
            snapshot_path = %model_paths
                .snapshot_path
                .as_ref()
                .map_or_else(|| "<none>".to_string(), |value| value.display().to_string()),
            quantized = model_paths.snapshot_path.is_some(),
            "DeepSeek OCR model engine loaded"
        );

        let decode = decode_parameters_from_env();
        let vision = vision_settings_from_env();
        let max_tiles = max_tiles_from_env();

        Ok(Self {
            model: Mutex::new(model),
            tokenizer,
            vision,
            max_tiles,
            decode,
            model_root: Arc::from(model_root.to_string()),
        })
    }
}

struct ModelPaths {
    model_kind: VisionModelKind,
    config_path: PathBuf,
    tokenizer_path: PathBuf,
    weights_path: PathBuf,
    snapshot_path: Option<PathBuf>,
    require_quantized: bool,
}

fn resolve_model_paths(
    model_root: &str,
    model_kind: VisionModelKind,
) -> Result<ModelPaths, String> {
    let root = Path::new(model_root);
    let require_quantized = require_quantized_snapshot(model_kind);
    let snapshot_path = resolve_effective_snapshot_path(root, require_quantized)?;
    Ok(ModelPaths {
        model_kind,
        config_path: root.join("config.json"),
        tokenizer_path: root.join("tokenizer.json"),
        weights_path: resolve_weights_path(root, model_kind)?,
        snapshot_path,
        require_quantized,
    })
}

fn prepare_device_with_fallback(
    requested_device: DeviceKind,
) -> Result<(candle_core::Device, Option<DType>), String> {
    let prepare_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        prepare_device_and_dtype(requested_device, None)
    }));
    match prepare_result {
        Ok(Ok(values)) => Ok(values),
        Ok(Err(error)) => {
            if matches!(requested_device, DeviceKind::Cpu) {
                return Err(sanitize_error_string(error));
            }
            tracing::warn!(
                event = "llm.vision.deepseek.engine.device_fallback",
                requested = ?requested_device,
                error = %sanitize_error_string(error),
                fallback = "cpu",
                "DeepSeek OCR requested device init failed; falling back to CPU"
            );
            prepare_device_and_dtype(DeviceKind::Cpu, None).map_err(sanitize_error_string)
        }
        Err(payload) => {
            if matches!(requested_device, DeviceKind::Cpu) {
                return Err(sanitize_error_string(format!(
                    "DeepSeek CPU device initialization panicked: {}",
                    panic_payload_to_string(&payload)
                )));
            }
            tracing::warn!(
                event = "llm.vision.deepseek.engine.device_fallback_panic",
                requested = ?requested_device,
                panic = %panic_payload_to_string(&payload),
                fallback = "cpu",
                "DeepSeek OCR requested device panicked during init; falling back to CPU"
            );
            prepare_device_and_dtype(DeviceKind::Cpu, None).map_err(sanitize_error_string)
        }
    }
}

fn load_model_with_dtype(
    config_path: &Path,
    weights_path: &Path,
    snapshot_path: Option<&Path>,
    device: &candle_core::Device,
    dtype: DType,
    model_kind: VisionModelKind,
) -> Result<Box<dyn OcrEngine>, String> {
    let load_args = ModelLoadArgs {
        kind: model_kind.as_core_kind(),
        config_path: Some(config_path),
        weights_path: Some(weights_path),
        snapshot_path,
        device: device.clone(),
        dtype,
    };
    let result = match model_kind {
        VisionModelKind::Deepseek => load_deepseek_model(load_args),
        VisionModelKind::PaddleOcrVl => load_paddleocr_model(load_args),
        VisionModelKind::DotsOcr => load_dots_model(load_args),
    };
    result.map_err(sanitize_error_string)
}

fn decode_parameters_from_env() -> DecodeParameters {
    let mut decode = DecodeParameters {
        max_new_tokens: parse_env_usize("XIUXIAN_VISION_OCR_MAX_NEW_TOKENS")
            .or_else(|| parse_env_usize("XIUXIAN_VISION_MAX_NEW_TOKENS"))
            .unwrap_or(1_024),
        ..DecodeParameters::default()
    };
    if let Some(value) = parse_env_f64("XIUXIAN_VISION_OCR_TEMPERATURE")
        .or_else(|| parse_env_f64("XIUXIAN_VISION_TEMPERATURE"))
    {
        decode.temperature = normalize_temperature(value, decode.temperature);
    }
    if let Some(value) =
        parse_env_f64("XIUXIAN_VISION_OCR_TOP_P").or_else(|| parse_env_f64("XIUXIAN_VISION_TOP_P"))
    {
        decode.top_p = normalize_top_p(value);
    }
    if let Some(value) = parse_env_usize("XIUXIAN_VISION_OCR_TOP_K")
        .or_else(|| parse_env_usize("XIUXIAN_VISION_TOP_K"))
    {
        decode.top_k = normalize_top_k(value);
    }
    if let Some(value) = parse_env_f32("XIUXIAN_VISION_OCR_REPETITION_PENALTY")
        .or_else(|| parse_env_f32("XIUXIAN_VISION_REPETITION_PENALTY"))
    {
        decode.repetition_penalty = normalize_repetition_penalty(value, decode.repetition_penalty);
    }
    if let Some(value) = parse_env_bool("XIUXIAN_VISION_OCR_USE_CACHE")
        .or_else(|| parse_env_bool("XIUXIAN_VISION_USE_CACHE"))
    {
        decode.use_cache = value;
    }
    decode
}

fn vision_settings_from_env() -> VisionSettings {
    VisionSettings {
        base_size: parse_env_u32("XIUXIAN_VISION_BASE_SIZE").unwrap_or(448),
        image_size: parse_env_u32("XIUXIAN_VISION_IMAGE_SIZE").unwrap_or(448),
        crop_mode: parse_env_bool("XIUXIAN_VISION_CROP_MODE").unwrap_or(true),
    }
}

fn max_tiles_from_env() -> usize {
    let max_tiles = parse_env_u32("XIUXIAN_VISION_MAX_TILES")
        .unwrap_or(12)
        .max(1);
    usize::try_from(max_tiles).unwrap_or(usize::MAX)
}

fn require_quantized_snapshot(model_kind: VisionModelKind) -> bool {
    let env_value = std::env::var("XIUXIAN_VISION_REQUIRE_QUANTIZED").ok();
    require_quantized_snapshot_with(env_value.as_deref(), model_kind)
}

fn require_quantized_snapshot_with(value: Option<&str>, model_kind: VisionModelKind) -> bool {
    value
        .map(|raw| raw.trim().to_ascii_lowercase())
        .map_or(model_kind == VisionModelKind::Deepseek, |raw| {
            !matches!(raw.as_str(), "0" | "false" | "no" | "off")
        })
}

pub(crate) fn require_quantized_snapshot_with_for_tests(value: Option<&str>) -> bool {
    require_quantized_snapshot_with(value, VisionModelKind::Deepseek)
}

fn resolve_effective_snapshot_path(
    model_root: &Path,
    require_quantized: bool,
) -> Result<Option<PathBuf>, String> {
    let Some(snapshot_path) = resolve_snapshot_path(model_root) else {
        return Ok(None);
    };

    match validate_snapshot_alignment(snapshot_path.as_path()) {
        Ok(()) => Ok(Some(snapshot_path)),
        Err(error) => {
            tracing::info!(
                event = "llm.vision.deepseek.snapshot.repair_started",
                path = %snapshot_path.display(),
                "DeepSeek OCR snapshot is unaligned; attempting automatic Rust-native repair"
            );

            match repair_dsq_if_needed(snapshot_path.as_path()) {
                DsqRepairResult::Repaired => {
                    tracing::info!(
                        event = "llm.vision.deepseek.snapshot.repaired",
                        path = %snapshot_path.display(),
                        "DeepSeek OCR snapshot successfully repaired and aligned"
                    );
                    Ok(Some(snapshot_path))
                }
                DsqRepairResult::AlreadyAligned => {
                    // This shouldn't happen if validate failed, but handle it gracefully
                    Ok(Some(snapshot_path))
                }
                DsqRepairResult::Failed(repair_error) => {
                    if require_quantized {
                        Err(format!(
                            "DeepSeek OCR snapshot alignment validation failed, and automatic repair also failed: {}. \
Original error: {}",
                            repair_error, error
                        ))
                    } else {
                        tracing::warn!(
                            event = "llm.vision.deepseek.snapshot.repair_failed",
                            path = %snapshot_path.display(),
                            error = %repair_error,
                            fallback = "safetensors",
                            "DeepSeek OCR snapshot repair failed; falling back to unquantized weights"
                        );
                        Ok(None)
                    }
                }
            }
        }
    }
}

fn validate_snapshot_alignment(snapshot_path: &Path) -> Result<(), String> {
    if !snapshot_path.exists() {
        return Err(sanitize_error_string(format!(
            "DeepSeek snapshot path does not exist: {}",
            snapshot_path.display()
        )));
    }
    if !snapshot_path.is_file() {
        return Err(sanitize_error_string(format!(
            "DeepSeek snapshot path is not a file: {}",
            snapshot_path.display()
        )));
    }

    let reader = DsqReader::open(snapshot_path).map_err(|error| {
        sanitize_error_string(format!(
            "DeepSeek snapshot is not a valid DSQ container at {}: {}",
            snapshot_path.display(),
            error
        ))
    })?;

    for record in reader.records() {
        let alignment = required_qoffset_alignment(record.q_dtype);
        if record.q_offset % alignment != 0 {
            return Err(sanitize_error_string(format!(
                "DeepSeek snapshot tensor `{}` has unaligned q_offset={} for {:?} (requires {}-byte alignment); \
this DSQ is incompatible with candle quantized loader and can abort the process",
                record.name, record.q_offset, record.q_dtype, alignment
            )));
        }
    }

    Ok(())
}

pub(crate) fn snapshot_qoffset_alignment_with_for_tests(offset: u64, dtype_code: u32) -> bool {
    DsqTensorDType::try_from(dtype_code)
        .map(|dtype| offset.is_multiple_of(required_qoffset_alignment(dtype)))
        .unwrap_or(false)
}

pub(crate) fn resolve_model_kind_label_with_for_tests(value: Option<&str>) -> &'static str {
    parse_model_kind_with(value).as_str()
}

pub(crate) fn resolve_model_kind_for_model_root_label_with_for_tests(
    value: Option<&str>,
    model_root: &Path,
) -> &'static str {
    resolve_model_kind_for_model_root_with(parse_model_kind_with(value), model_root).as_str()
}

fn preferred_dtype_for_device(device: &candle_core::Device) -> DType {
    if device.is_cpu() {
        DType::F32
    } else if device.is_metal() {
        DType::F16
    } else {
        DType::BF16
    }
}

fn resolve_model_kind() -> VisionModelKind {
    let configured = parse_env_string("XIUXIAN_VISION_MODEL_KIND");
    parse_model_kind_with(configured.as_deref())
}

fn resolve_model_kind_for_model_root(model_root: &str) -> VisionModelKind {
    let configured = resolve_model_kind();
    resolve_model_kind_for_model_root_with(configured, Path::new(model_root))
}

fn resolve_model_kind_for_model_root_with(
    configured: VisionModelKind,
    model_root: &Path,
) -> VisionModelKind {
    if configured == VisionModelKind::Deepseek && model_root_looks_like_dots(model_root) {
        tracing::info!(
            event = "llm.vision.deepseek.engine.model_kind_root_fallback",
            requested = VisionModelKind::Deepseek.as_str(),
            fallback = VisionModelKind::DotsOcr.as_str(),
            model_root = %model_root.display(),
            "Configured model kind resolved to DeepSeek but model root layout matches Dots OCR; using Dots OCR loader"
        );
        VisionModelKind::DotsOcr
    } else {
        configured
    }
}

fn model_root_looks_like_dots(model_root: &Path) -> bool {
    model_root.join("model.safetensors.index.json").is_file()
        || model_root
            .join("dots.ocr")
            .join("model.safetensors.index.json")
            .is_file()
        || model_root
            .file_name()
            .and_then(|value| value.to_str())
            .is_some_and(|name| name.to_ascii_lowercase().contains("dots"))
}

fn parse_model_kind_with(raw: Option<&str>) -> VisionModelKind {
    if raw.is_some_and(|value| value.eq_ignore_ascii_case("auto")) {
        return VisionModelKind::DEFAULT;
    }
    if let Some(kind) = raw.and_then(VisionModelKind::parse) {
        kind
    } else {
        if let Some(value) = raw {
            tracing::warn!(
                event = "llm.vision.deepseek.engine.invalid_model_kind",
                model_kind = %value,
                fallback = VisionModelKind::DEFAULT.as_str(),
                "Unknown XIUXIAN_VISION_MODEL_KIND or llm.vision.deepseek.model_kind; falling back to default model kind"
            );
        }
        VisionModelKind::DEFAULT
    }
}

fn normalize_temperature(value: f64, fallback: f64) -> f64 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        fallback
    }
}

fn normalize_top_p(value: f64) -> Option<f64> {
    if !value.is_finite() || value <= 0.0 {
        None
    } else if value < 1.0 {
        Some(value)
    } else {
        None
    }
}

fn normalize_top_k(value: usize) -> Option<usize> {
    (value > 0).then_some(value)
}

fn normalize_repetition_penalty(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}
