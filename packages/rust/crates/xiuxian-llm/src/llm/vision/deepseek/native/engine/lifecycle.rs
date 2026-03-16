//! Simplified engine lifecycle using upstream deepseek-ocr loaders directly.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use deepseek_ocr_core::runtime::DeviceKind;

use crate::llm::error::LlmResult;
use crate::llm::vision::PreparedVisionImage;
use crate::llm::vision::deepseek::runtime::DeepseekRuntime;
use crate::llm::vision::deepseek::util::{internal_error, sanitize_error_string};

use super::batch_lane::infer_with_batch_lane;
use super::core::DeepseekEngine;

static ENGINE_SLOT: OnceLock<Mutex<Option<CachedEngine>>> = OnceLock::new();

/// Flag to force CPU fallback after Metal errors.
static FORCE_CPU_FALLBACK: AtomicBool = AtomicBool::new(false);

struct CachedEngine {
    model_root: Arc<str>,
    device_kind: CachedDeviceKind,
    loaded: Result<Arc<DeepseekEngine>, Arc<str>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CachedDeviceKind {
    Cpu,
    Metal,
    Cuda,
}

pub(crate) fn infer(
    runtime: &DeepseekRuntime,
    prepared: &PreparedVisionImage,
    stop_signal: Option<Arc<AtomicBool>>,
) -> LlmResult<Option<String>> {
    let use_cpu = FORCE_CPU_FALLBACK.load(Ordering::Acquire);
    let is_cached = has_cached_engine(runtime, use_cpu);
    tracing::debug!(
        event = "llm.vision.deepseek.infer_start",
        width = prepared.width,
        height = prepared.height,
        scale = prepared.scale,
        engine_cached = is_cached,
        "DeepSeek OCR: Starting inference"
    );

    let engine = get_or_create_engine(runtime, use_cpu)?;

    match infer_with_batch_lane(engine, prepared, stop_signal.clone()) {
        Ok(markdown) => Ok(markdown),
        Err(error) => {
            let error_text = error.to_string();
            if should_retry_with_cpu_fallback(error_text.as_str()) && !use_cpu {
                FORCE_CPU_FALLBACK.store(true, Ordering::Release);
                tracing::warn!(
                    event = "llm.vision.deepseek.infer.retry_cpu_fallback",
                    error = %sanitize_error_string(error_text),
                    "DeepSeek OCR decode hit a Metal resource error; retrying on CPU"
                );
                let cpu_engine = get_or_create_engine(runtime, true)?;
                infer_with_batch_lane(cpu_engine, prepared, stop_signal)
            } else {
                Err(error)
            }
        }
    }
}

pub(crate) fn prewarm(runtime: &DeepseekRuntime) -> LlmResult<()> {
    let started = Instant::now();
    let engine = get_or_create_engine(runtime, false)?;
    let dummy = PreparedVisionImage::create_dummy(1, 1);
    if let Err(error) = engine.warmup_once(&dummy) {
        tracing::warn!(
            event = "llm.vision.deepseek.engine.prewarm_failed",
            elapsed_ms = started.elapsed().as_millis(),
            error = %sanitize_error_string(error),
            "DeepSeek OCR dummy prewarm inference failed"
        );
    } else {
        tracing::info!(
            event = "llm.vision.deepseek.engine.prewarm_completed",
            elapsed_ms = started.elapsed().as_millis(),
            "DeepSeek OCR prewarm completed"
        );
    }
    Ok(())
}

fn get_or_create_engine(
    runtime: &DeepseekRuntime,
    force_cpu: bool,
) -> LlmResult<Arc<DeepseekEngine>> {
    let model_root = match runtime {
        DeepseekRuntime::Configured { model_root } => model_root.as_ref(),
        DeepseekRuntime::Disabled { reason } => {
            return Err(internal_error(format!(
                "deepseek runtime disabled: {reason}"
            )));
        }
        DeepseekRuntime::RemoteHttp { .. } => {
            return Err(internal_error(
                "get_or_create_engine called on RemoteHttp runtime",
            ));
        }
    };

    let requested_device = requested_device_kind(force_cpu);
    let cached_device_kind = CachedDeviceKind::from(requested_device);
    let slot = ENGINE_SLOT.get_or_init(|| Mutex::new(None));
    let mut guard = slot
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    if let Some(entry) = guard.as_ref()
        && should_reuse_cached_engine(entry, model_root, cached_device_kind)
    {
        return clone_cached_engine_result(&entry.loaded);
    }

    let started = Instant::now();
    tracing::info!(
        event = "llm.vision.deepseek.engine.loading",
        model_root = %model_root,
        device = ?requested_device,
        "DeepSeek OCR: Loading engine"
    );

    let loaded = DeepseekEngine::load_for_device(model_root, requested_device)
        .map(Arc::new)
        .map_err(Arc::<str>::from);

    match &loaded {
        Ok(_) => {
            tracing::info!(
                event = "llm.vision.deepseek.engine.loaded",
                model_root = %model_root,
                elapsed_ms = started.elapsed().as_millis(),
                "DeepSeek OCR engine loaded"
            );
        }
        Err(error) => {
            tracing::error!(
                event = "llm.vision.deepseek.engine.load_failed",
                model_root = %model_root,
                elapsed_ms = started.elapsed().as_millis(),
                error = %error,
                "DeepSeek OCR engine load failed"
            );
        }
    }

    *guard = Some(CachedEngine {
        model_root: Arc::from(model_root),
        device_kind: cached_device_kind,
        loaded: loaded.clone(),
    });
    clone_cached_engine_result(&loaded)
}

fn parse_device_kind() -> DeviceKind {
    crate::llm::vision::deepseek::native::env::parse_device_kind()
}

fn has_cached_engine(runtime: &DeepseekRuntime, force_cpu: bool) -> bool {
    let model_root = match runtime {
        DeepseekRuntime::Configured { model_root } => model_root.as_ref(),
        DeepseekRuntime::Disabled { .. } | DeepseekRuntime::RemoteHttp { .. } => return false,
    };
    let cached_device_kind = CachedDeviceKind::from(requested_device_kind(force_cpu));
    ENGINE_SLOT
        .get()
        .and_then(|slot| {
            let guard = slot
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            guard
                .as_ref()
                .filter(|entry| should_reuse_cached_engine(entry, model_root, cached_device_kind))
                .map(|_| ())
        })
        .is_some()
}

fn requested_device_kind(force_cpu: bool) -> DeviceKind {
    requested_device_kind_with(parse_device_kind(), force_cpu)
}

fn requested_device_kind_with(requested_device: DeviceKind, force_cpu: bool) -> DeviceKind {
    if force_cpu {
        DeviceKind::Cpu
    } else {
        requested_device
    }
}

fn should_reuse_cached_engine(
    entry: &CachedEngine,
    model_root: &str,
    requested_device: CachedDeviceKind,
) -> bool {
    entry.model_root.as_ref() == model_root && entry.device_kind == requested_device
}

fn clone_cached_engine_result(
    loaded: &Result<Arc<DeepseekEngine>, Arc<str>>,
) -> LlmResult<Arc<DeepseekEngine>> {
    match loaded {
        Ok(engine) => Ok(Arc::clone(engine)),
        Err(error) => Err(internal_error(format!(
            "deepseek engine initialization failed: {}",
            error.as_ref()
        ))),
    }
}

impl CachedDeviceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Metal => "metal",
            Self::Cuda => "cuda",
        }
    }

    fn parse_for_tests(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "cpu" => Self::Cpu,
            "metal" => Self::Metal,
            "cuda" => Self::Cuda,
            other => panic!("unsupported test device kind: {other}"),
        }
    }
}

impl From<DeviceKind> for CachedDeviceKind {
    fn from(value: DeviceKind) -> Self {
        match value {
            DeviceKind::Cpu => Self::Cpu,
            DeviceKind::Metal => Self::Metal,
            DeviceKind::Cuda => Self::Cuda,
        }
    }
}

fn should_retry_with_cpu_fallback(error_text: &str) -> bool {
    let lower = error_text.to_ascii_lowercase();
    lower.contains("metal") || lower.contains("cuda") || lower.contains("out of memory")
}

pub(crate) fn should_retry_with_cpu_fallback_for_tests(error_text: &str) -> bool {
    should_retry_with_cpu_fallback(error_text)
}

pub(crate) fn resolve_engine_device_label_with_for_tests(
    requested_device: &str,
    force_cpu: bool,
) -> &'static str {
    let requested_device = parse_device_kind_for_tests(requested_device);
    CachedDeviceKind::from(requested_device_kind_with(requested_device, force_cpu)).as_str()
}

pub(crate) fn should_reuse_engine_cache_for_tests(
    cached_model_root: &str,
    cached_device: &str,
    requested_model_root: &str,
    requested_device: &str,
    force_cpu: bool,
) -> bool {
    cached_model_root == requested_model_root
        && CachedDeviceKind::parse_for_tests(cached_device)
            == CachedDeviceKind::from(requested_device_kind_with(
                parse_device_kind_for_tests(requested_device),
                force_cpu,
            ))
}

fn parse_device_kind_for_tests(value: &str) -> DeviceKind {
    match value.trim().to_ascii_lowercase().as_str() {
        "cpu" => DeviceKind::Cpu,
        "metal" => DeviceKind::Metal,
        "cuda" => DeviceKind::Cuda,
        other => panic!("unsupported test device kind: {other}"),
    }
}
