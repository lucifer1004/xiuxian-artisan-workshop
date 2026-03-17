use std::path::{Path, PathBuf};
use std::sync::Mutex;

use candle_core::DType;
use deepseek_ocr_core::{
    DecodeParameters, ModelLoadArgs, OcrEngine, VisionSettings,
    runtime::{DeviceKind, prepare_device_and_dtype},
};
use deepseek_ocr_dsq::{DsqReader, DsqTensorDType};
use deepseek_ocr_infer_deepseek::{LowPrecisionLoadPolicy, load_model_with_low_precision_policy};
use deepseek_ocr_infer_dots::load_model as load_dots_model;
use deepseek_ocr_infer_paddleocr::load_model as load_paddleocr_model;
use tokenizers::Tokenizer;

use super::retry::panic_payload_to_string;
use crate::llm::vision::deepseek::config;
use crate::llm::vision::deepseek::dsq_alignment::required_qoffset_alignment;
use crate::llm::vision::deepseek::model_kind::VisionModelKind;
use crate::llm::vision::deepseek::native::env::{
    parse_env_bool, parse_env_f32, parse_env_f64, parse_env_string, parse_env_u32, parse_env_usize,
    resolve_snapshot_path, resolve_weights_path,
};
use crate::llm::vision::deepseek::util::sanitize_error_string;

use super::core::DeepseekEngine;
use super::dsq_repair::{DsqRepairResult, repair_dsq_if_needed};

const UPSTREAM_DEFAULT_BASE_SIZE: u32 = 1_024;
const UPSTREAM_DEFAULT_IMAGE_SIZE: u32 = 640;
const UPSTREAM_DEFAULT_CROP_MODE: bool = true;

impl DeepseekEngine {
    pub(super) fn load_for_device(
        model_root: &str,
        requested_device: DeviceKind,
    ) -> Result<Self, String> {
        let model_kind = resolve_model_kind_for_model_root(model_root);
        Self::load_for_device_with_kind(model_root, requested_device, model_kind)
    }

    pub(super) fn load_for_device_with_kind(
        model_root: &str,
        requested_device: DeviceKind,
        model_kind: VisionModelKind,
    ) -> Result<Self, String> {
        tracing::debug!(
            event = "llm.vision.deepseek.engine.load_start",
            model_root = %model_root,
            requested_device = ?requested_device,
            model_kind = %model_kind.as_str(),
            "DeepSeek OCR engine load started"
        );

        let model_paths = resolve_model_paths(model_root, model_kind)?;

        if model_paths.require_quantized && model_paths.snapshot_path.is_none() {
            return Err(
                "DeepSeek OCR quantized snapshot is required but none was found. \
Set XIUXIAN_VISION_SNAPSHOT_PATH (or place exactly one .dsq file under model root), \
or set XIUXIAN_VISION_REQUIRE_QUANTIZED=0 to allow unquantized loading."
                    .to_string(),
            );
        }

        let (device, maybe_dtype) = prepare_device(requested_device)?;
        let dtype = resolve_model_load_dtype(maybe_dtype, device.is_metal() || device.is_cuda());
        let decode = decode_parameters_from_env();
        let vision = vision_settings_from_env();
        let max_tiles = max_tiles_from_env();
        let low_precision_load_policy = low_precision_load_policy_from_env();

        tracing::info!(
            event = "llm.vision.deepseek.engine.device_selected",
            requested_device = ?requested_device,
            device = ?device,
            dtype = ?dtype,
            model_root = %model_root,
            quantized = model_paths.snapshot_path.is_some(),
            "DeepSeek OCR engine selected execution device and dtype"
        );
        tracing::info!(
            event = "llm.vision.deepseek.engine.effective_config",
            model_root = %model_root,
            model_kind = %model_paths.model_kind.as_str(),
            config_path = %model_paths.config_path.display(),
            tokenizer_path = %model_paths.tokenizer_path.display(),
            weights_path = %model_paths.weights_path.display(),
            snapshot_path = model_paths
                .snapshot_path
                .as_ref()
                .map_or_else(|| "<none>".to_string(), |value| value.display().to_string()),
            device = ?device,
            dtype = ?dtype,
            base_size = vision.base_size,
            image_size = vision.image_size,
            crop_mode = vision.crop_mode,
            max_tiles,
            max_new_tokens = decode.max_new_tokens,
            decode_temperature = decode.temperature,
            decode_top_p = ?decode.top_p,
            decode_top_k = ?decode.top_k,
            decode_repetition_penalty = decode.repetition_penalty,
            decode_use_cache = decode.use_cache,
            preload_language_f32_aux = low_precision_load_policy.preload_language_f32_aux,
            preload_vision_f32_aux = low_precision_load_policy.preload_vision_f32_aux,
            preload_linear_weight_f32 = low_precision_load_policy.preload_linear_weight_f32,
            promote_language_input_f32 = low_precision_load_policy.promote_language_input_f32,
            lazy_moe_experts = low_precision_load_policy.lazy_moe_experts,
            lazy_clip_transformer_layers = low_precision_load_policy.lazy_clip_transformer_layers,
            require_quantized = model_paths.require_quantized,
            "DeepSeek OCR effective config resolved"
        );

        let model = load_model_with_dtype(
            model_paths.config_path.as_path(),
            model_paths.weights_path.as_path(),
            model_paths.snapshot_path.as_deref(),
            &device,
            dtype,
            model_paths.model_kind,
            low_precision_load_policy,
        )
        .map_err(sanitize_error_string)?;

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

        Ok(Self {
            model: Mutex::new(model),
            tokenizer,
            vision,
            max_tiles,
            decode,
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
    let config_path = root.join("config.json");
    let tokenizer_path = root.join("tokenizer.json");
    let weights_path = resolve_weights_path(root, model_kind)?;
    Ok(ModelPaths {
        model_kind,
        config_path,
        tokenizer_path,
        weights_path,
        snapshot_path,
        require_quantized,
    })
}

fn prepare_device(
    requested_device: DeviceKind,
) -> Result<(candle_core::Device, Option<DType>), String> {
    let prepare_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        prepare_device_and_dtype(requested_device, None)
    }));

    match prepare_result {
        Ok(result) => result.map_err(sanitize_error_string),
        Err(payload) => Err(sanitize_error_string(format!(
            "DeepSeek {:?} device initialization panicked: {}",
            requested_device,
            panic_payload_to_string(&payload)
        ))),
    }
}

fn load_model_with_dtype(
    config_path: &Path,
    weights_path: &Path,
    snapshot_path: Option<&Path>,
    device: &candle_core::Device,
    dtype: DType,
    model_kind: VisionModelKind,
    low_precision_load_policy: LowPrecisionLoadPolicy,
) -> Result<Box<dyn OcrEngine>, String> {
    let load_start = std::time::Instant::now();

    tracing::info!(
        event = "llm.vision.deepseek.engine.load_model_start",
        config_path = %config_path.display(),
        weights_path = %weights_path.display(),
        snapshot_path = snapshot_path.map_or_else(|| "none".to_string(), |p| p.display().to_string()),
        device = ?device,
        dtype = ?dtype,
        model_kind = %model_kind.as_str(),
        preload_language_f32_aux = low_precision_load_policy.preload_language_f32_aux,
        preload_vision_f32_aux = low_precision_load_policy.preload_vision_f32_aux,
        preload_linear_weight_f32 = low_precision_load_policy.preload_linear_weight_f32,
        promote_language_input_f32 = low_precision_load_policy.promote_language_input_f32,
        lazy_moe_experts = low_precision_load_policy.lazy_moe_experts,
        lazy_clip_transformer_layers = low_precision_load_policy.lazy_clip_transformer_layers,
        "DeepSeek OCR invoking model loader"
    );

    let load_args = ModelLoadArgs {
        kind: model_kind.as_core_kind(),
        config_path: Some(config_path),
        weights_path: Some(weights_path),
        snapshot_path,
        device: device.clone(),
        dtype,
    };

    let result = match model_kind {
        VisionModelKind::Deepseek => {
            load_model_with_low_precision_policy(load_args, low_precision_load_policy)
        }
        VisionModelKind::PaddleOcrVl => load_paddleocr_model(load_args),
        VisionModelKind::DotsOcr => load_dots_model(load_args),
    };

    let elapsed = load_start.elapsed();

    match &result {
        Ok(_) => tracing::info!(
            event = "llm.vision.deepseek.engine.load_model_success",
            model_kind = %model_kind.as_str(),
            elapsed_ms = elapsed.as_millis(),
            "DeepSeek OCR model loader completed"
        ),
        Err(e) => tracing::error!(
            event = "llm.vision.deepseek.engine.load_model_failed",
            model_kind = %model_kind.as_str(),
            elapsed_ms = elapsed.as_millis(),
            error = %sanitize_error_string(e.to_string()),
            "DeepSeek OCR model loader failed"
        ),
    }

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

fn default_vision_settings() -> VisionSettings {
    VisionSettings {
        base_size: UPSTREAM_DEFAULT_BASE_SIZE,
        image_size: UPSTREAM_DEFAULT_IMAGE_SIZE,
        crop_mode: UPSTREAM_DEFAULT_CROP_MODE,
    }
}

fn vision_settings_from_env() -> VisionSettings {
    let defaults = default_vision_settings();
    VisionSettings {
        base_size: parse_env_u32("XIUXIAN_VISION_BASE_SIZE").unwrap_or(defaults.base_size),
        image_size: parse_env_u32("XIUXIAN_VISION_IMAGE_SIZE").unwrap_or(defaults.image_size),
        crop_mode: parse_env_bool("XIUXIAN_VISION_CROP_MODE").unwrap_or(defaults.crop_mode),
    }
}

pub(crate) fn resolve_vision_settings_with_for_tests(
    base_size: Option<u32>,
    image_size: Option<u32>,
    crop_mode: Option<bool>,
) -> (u32, u32, bool) {
    let defaults = default_vision_settings();
    let resolved = VisionSettings {
        base_size: base_size.unwrap_or(defaults.base_size),
        image_size: image_size.unwrap_or(defaults.image_size),
        crop_mode: crop_mode.unwrap_or(defaults.crop_mode),
    };
    (resolved.base_size, resolved.image_size, resolved.crop_mode)
}

fn max_tiles_from_env() -> usize {
    let max_tiles = parse_env_u32("XIUXIAN_VISION_MAX_TILES")
        .unwrap_or(12)
        .max(1);
    usize::try_from(max_tiles).unwrap_or(usize::MAX)
}

fn low_precision_load_policy_from_env() -> LowPrecisionLoadPolicy {
    LowPrecisionLoadPolicy {
        preload_language_f32_aux: parse_env_bool("XIUXIAN_VISION_PRELOAD_LANGUAGE_F32_AUX")
            .unwrap_or(true),
        preload_vision_f32_aux: parse_env_bool("XIUXIAN_VISION_PRELOAD_VISION_F32_AUX")
            .unwrap_or(true),
        preload_linear_weight_f32: parse_env_bool("XIUXIAN_VISION_PRELOAD_LINEAR_WEIGHT_F32")
            .unwrap_or(true),
        promote_language_input_f32: parse_env_bool("XIUXIAN_VISION_PROMOTE_LANGUAGE_INPUT_F32")
            .unwrap_or(true),
        lazy_moe_experts: parse_env_bool("XIUXIAN_VISION_LAZY_MOE_EXPERTS").unwrap_or(false),
        lazy_clip_transformer_layers: parse_env_bool("XIUXIAN_VISION_LAZY_CLIP_TRANSFORMER_LAYERS")
            .unwrap_or(false),
    }
}

pub(crate) fn resolve_low_precision_load_policy_for_tests(
    preload_language_f32_aux: Option<bool>,
    preload_vision_f32_aux: Option<bool>,
    preload_linear_weight_f32: Option<bool>,
    promote_language_input_f32: Option<bool>,
    lazy_moe_experts: Option<bool>,
    lazy_clip_transformer_layers: Option<bool>,
) -> (bool, bool, bool, bool, bool, bool) {
    let policy = LowPrecisionLoadPolicy {
        preload_language_f32_aux: preload_language_f32_aux.unwrap_or(true),
        preload_vision_f32_aux: preload_vision_f32_aux.unwrap_or(true),
        preload_linear_weight_f32: preload_linear_weight_f32.unwrap_or(true),
        promote_language_input_f32: promote_language_input_f32.unwrap_or(true),
        lazy_moe_experts: lazy_moe_experts.unwrap_or(false),
        lazy_clip_transformer_layers: lazy_clip_transformer_layers.unwrap_or(false),
    };
    (
        policy.preload_language_f32_aux,
        policy.preload_vision_f32_aux,
        policy.preload_linear_weight_f32,
        policy.promote_language_input_f32,
        policy.lazy_moe_experts,
        policy.lazy_clip_transformer_layers,
    )
}

fn require_quantized_snapshot(model_kind: VisionModelKind) -> bool {
    match parse_env_bool("XIUXIAN_VISION_REQUIRE_QUANTIZED") {
        Some(value) => {
            require_quantized_snapshot_with(Some(if value { "true" } else { "false" }), model_kind)
        }
        None => require_quantized_snapshot_with(None, model_kind),
    }
}

fn require_quantized_snapshot_with(value: Option<&str>, model_kind: VisionModelKind) -> bool {
    value
        .map(|raw| raw.trim().to_ascii_lowercase())
        .map_or(model_kind == VisionModelKind::DotsOcr, |raw| {
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
                            "DeepSeek OCR snapshot alignment validation failed, and automatic repair also failed: {repair_error}. \
Original error: {error}"
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
    resolve_model_kind_for_model_root_with(value, model_root).as_str()
}

fn resolve_model_load_dtype(maybe_dtype: Option<DType>, accelerated_backend: bool) -> DType {
    maybe_dtype.unwrap_or({
        if accelerated_backend {
            DType::F16
        } else {
            DType::F32
        }
    })
}

pub(crate) fn resolve_model_load_dtype_label_for_tests(
    prepared_dtype: Option<&str>,
    accelerated_backend: bool,
) -> &'static str {
    let maybe_dtype = prepared_dtype.map(parse_dtype_for_tests);
    dtype_label(resolve_model_load_dtype(maybe_dtype, accelerated_backend))
}

fn parse_dtype_for_tests(value: &str) -> DType {
    match value.trim().to_ascii_lowercase().as_str() {
        "f32" => DType::F32,
        "f16" => DType::F16,
        "bf16" => DType::BF16,
        other => panic!("unsupported test dtype: {other}"),
    }
}

fn dtype_label(dtype: DType) -> &'static str {
    match dtype {
        DType::F32 => "f32",
        DType::F16 => "f16",
        DType::BF16 => "bf16",
        other => panic!("unsupported test dtype label: {other:?}"),
    }
}

fn resolve_model_kind_for_model_root(model_root: &str) -> VisionModelKind {
    let configured = parse_env_string("XIUXIAN_VISION_MODEL_KIND").or_else(config::model_kind);
    resolve_model_kind_for_model_root_with(configured.as_deref(), Path::new(model_root))
}

pub(crate) fn resolve_model_kind_for_model_root_label_from_sources_for_tests(
    env_value: Option<&str>,
    config_value: Option<&str>,
    model_root: &Path,
) -> &'static str {
    resolve_model_kind_for_model_root_with(env_value.or(config_value), model_root).as_str()
}

fn resolve_model_kind_for_model_root_with(
    configured_raw: Option<&str>,
    model_root: &Path,
) -> VisionModelKind {
    let configured = parse_model_kind_with(configured_raw);
    let explicit = configured_raw
        .and_then(|value| (!value.eq_ignore_ascii_case("auto")).then_some(value))
        .and_then(VisionModelKind::parse)
        .is_some();
    if !explicit
        && configured == VisionModelKind::Deepseek
        && model_root_looks_like_dots(model_root)
    {
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
