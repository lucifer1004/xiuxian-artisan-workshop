use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use deepseek_ocr_core::runtime::DeviceKind;

use super::super::super::super::preprocess::PreparedVisionImage;
use super::super::super::model_kind::VisionModelKind;
use super::super::super::runtime::{DeepseekRuntime, resolve_model_root_for_kind};
use super::super::super::util::{internal_error, sanitize_error_string};
use super::super::env::{parse_env_bool, parse_env_string, parse_env_u32, parse_env_u64};
use super::batch_lane::infer_with_batch_lane;
use crate::llm::error::LlmResult;

use super::core::DeepseekEngine;

static ENGINE_PRIMARY: OnceLock<Result<Arc<DeepseekEngine>, Arc<str>>> = OnceLock::new();
static ENGINE_DOTS: OnceLock<Result<Arc<DeepseekEngine>, Arc<str>>> = OnceLock::new();

// CPU fallback engines - also cached to prevent memory explosion
static ENGINE_PRIMARY_CPU: OnceLock<Result<Arc<DeepseekEngine>, Arc<str>>> = OnceLock::new();
static ENGINE_DOTS_CPU: OnceLock<Result<Arc<DeepseekEngine>, Arc<str>>> = OnceLock::new();

static FORCE_CPU_FALLBACK_PRIMARY: AtomicBool = AtomicBool::new(false);
static FORCE_CPU_FALLBACK_DOTS: AtomicBool = AtomicBool::new(false);

const DEFAULT_AUTO_COMPLEX_MIN_TILES: u32 = 8;
const DEFAULT_AUTO_COMPLEX_MIN_PIXELS: u64 = 2_500_000;
const DEFAULT_IMAGE_SIZE: u32 = 448;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RouteSelection {
    Primary,
    Dots,
}

pub(crate) fn infer(
    runtime: &DeepseekRuntime,
    prepared: &PreparedVisionImage,
    stop_signal: Option<Arc<AtomicBool>>,
) -> LlmResult<Option<String>> {
    let preferred_route = select_route(prepared);

    let primary_cached = ENGINE_PRIMARY.get().is_some();
    let dots_cached = ENGINE_DOTS.get().is_some();
    let primary_cpu_cached = ENGINE_PRIMARY_CPU.get().is_some();
    let dots_cpu_cached = ENGINE_DOTS_CPU.get().is_some();

    eprintln!(
        "[MEMORY TRACE] infer() START: primary_cached={} dots_cached={} primary_cpu_cached={} dots_cpu_cached={}",
        primary_cached, dots_cached, primary_cpu_cached, dots_cpu_cached
    );

    tracing::info!(
        event = "llm.vision.deepseek.infer_start",
        width = prepared.width,
        height = prepared.height,
        scale = prepared.scale,
        preferred_route = ?preferred_route,
        primary_cached = primary_cached,
        dots_cached = dots_cached,
        primary_cpu_cached = primary_cpu_cached,
        dots_cpu_cached = dots_cpu_cached,
        "[MEMORY TRACE] Starting inference - check cache status"
    );

    let mut effective_route = preferred_route;

    if cpu_fallback_flag(effective_route).load(Ordering::Acquire) {
        tracing::info!(
            event = "llm.vision.deepseek.infer.force_cpu_fallback",
            route = ?effective_route,
            "DeepSeek OCR forcing CPU engine due to prior Metal resource failure"
        );
        let (_, fallback) = get_cpu_engine_for_route_or_fallback(runtime, effective_route)?;
        return infer_with_batch_lane(fallback, prepared, stop_signal);
    }

    let (resolved_route, engine) = get_engine_for_route_or_fallback(runtime, preferred_route)?;
    effective_route = resolved_route;
    match infer_with_batch_lane(engine, prepared, stop_signal.clone()) {
        Ok(markdown) => Ok(markdown),
        Err(error) => {
            let error_text = error.to_string();
            if should_retry_with_cpu_fallback(error_text.as_str()) {
                cpu_fallback_flag(effective_route).store(true, Ordering::Release);
                tracing::warn!(
                    event = "llm.vision.deepseek.infer.retry_cpu_fallback",
                    route = ?effective_route,
                    error = %sanitize_error_string(error_text),
                    "DeepSeek OCR decode hit a Metal resource error; retrying once on CPU engine"
                );
                let (_, fallback) = get_cpu_engine_for_route_or_fallback(runtime, effective_route)?;
                return infer_with_batch_lane(fallback, prepared, stop_signal);
            }
            Err(error)
        }
    }
}

pub(crate) fn prewarm(runtime: &DeepseekRuntime) -> LlmResult<()> {
    let started = Instant::now();
    let engine = get_primary_engine(runtime)?;
    let dummy = PreparedVisionImage::create_dummy(1, 1);
    if let Err(error) = engine.warmup_once(&dummy) {
        tracing::warn!(
            event = "llm.vision.deepseek.engine.prewarm_dummy_failed",
            elapsed_ms = started.elapsed().as_millis(),
            error = %sanitize_error_string(error),
            "DeepSeek OCR dummy prewarm inference failed; runtime will continue"
        );
    } else {
        tracing::info!(
            event = "llm.vision.deepseek.engine.prewarm_dummy_completed",
            elapsed_ms = started.elapsed().as_millis(),
            "DeepSeek OCR dummy prewarm inference completed"
        );
    }

    if auto_route_enabled()
        && let Ok(dots_engine) = get_dots_engine()
    {
        if let Err(error) = dots_engine.warmup_once(&dummy) {
            tracing::warn!(
                event = "llm.vision.deepseek.engine.prewarm_dots_failed",
                elapsed_ms = started.elapsed().as_millis(),
                error = %sanitize_error_string(error),
                "DeepSeek OCR Dots prewarm failed; runtime will continue"
            );
        } else {
            tracing::info!(
                event = "llm.vision.deepseek.engine.prewarm_dots_completed",
                elapsed_ms = started.elapsed().as_millis(),
                "DeepSeek OCR Dots prewarm completed"
            );
        }
    }
    Ok(())
}

fn get_primary_engine(runtime: &DeepseekRuntime) -> LlmResult<Arc<DeepseekEngine>> {
    let model_root = match runtime {
        DeepseekRuntime::Configured { model_root } => model_root.as_ref(),
        DeepseekRuntime::Disabled { reason } => {
            return Err(internal_error(format!(
                "deepseek runtime disabled: {}",
                reason
            )));
        }
        DeepseekRuntime::RemoteHttp { .. } => {
            return Err(internal_error(
                "get_primary_engine called on RemoteHttp runtime",
            ));
        }
    };

    let is_cached = ENGINE_PRIMARY.get().is_some();

    eprintln!(
        "[MEMORY TRACE] get_primary_engine() model_root={} cached={}",
        model_root, is_cached
    );

    tracing::info!(
        event = "llm.vision.deepseek.engine.get_primary_start",
        model_root = %model_root,
        engine_cached = is_cached,
        "[MEMORY TRACE] Getting primary engine - CACHED={} (if false, will load from disk)",
        is_cached
    );

    let entry = ENGINE_PRIMARY.get_or_init(|| {
        eprintln!("[MEMORY TRACE] *** LOADING ENGINE FROM DISK ***");

        tracing::warn!(
            event = "llm.vision.deepseek.engine.loading_from_disk",
            model_root = %model_root,
            "[MEMORY TRACE] *** ENGINE NOT CACHED - LOADING FROM DISK - MAJOR MEMORY ALLOCATION STARTING ***"
        );

        let started = Instant::now();
        let loaded = DeepseekEngine::load(model_root)
            .map(Arc::new)
            .map_err(Arc::<str>::from);
        match &loaded {
            Ok(_) => {
                eprintln!(
                    "[MEMORY TRACE] Engine loaded OK in {}ms",
                    started.elapsed().as_millis()
                );

                tracing::info!(
                    event = "llm.vision.deepseek.engine.init.completed",
                    model_root,
                    elapsed_ms = started.elapsed().as_millis(),
                    "[MEMORY TRACE] Engine initialized successfully"
                );
            }
            Err(error) => {
                eprintln!(
                    "[MEMORY TRACE] Engine load FAILED: {}",
                    error
                );

                tracing::error!(
                    event = "llm.vision.deepseek.engine.init.failed",
                    model_root,
                    elapsed_ms = started.elapsed().as_millis(),
                    error = %error,
                    "[MEMORY TRACE] Engine initialization failed"
                );
            }
        }
        loaded
    });

    match entry {
        Ok(engine) => {
            eprintln!("[MEMORY TRACE] Engine ready");

            tracing::info!(
                event = "llm.vision.deepseek.engine.get_primary_success",
                model_root = %model_root,
                "[MEMORY TRACE] Primary engine ready"
            );
            Ok(Arc::clone(engine))
        }
        Err(error) => Err(internal_error(format!(
            "deepseek primary engine initialization failed: {}",
            error.as_ref()
        ))),
    }
}

fn get_dots_engine() -> LlmResult<Arc<DeepseekEngine>> {
    let model_root = dots_model_root()?;

    let entry = ENGINE_DOTS.get_or_init(|| {
        let started = Instant::now();
        let loaded = DeepseekEngine::load_with_kind(model_root.as_str(), VisionModelKind::DotsOcr)
            .map(Arc::new)
            .map_err(Arc::<str>::from);
        match &loaded {
            Ok(_) => {
                tracing::info!(
                    event = "llm.vision.deepseek.engine.init_dots.completed",
                    model_root = %model_root,
                    elapsed_ms = started.elapsed().as_millis(),
                    "DeepSeek OCR Dots engine initialized"
                );
            }
            Err(error) => {
                tracing::error!(
                    event = "llm.vision.deepseek.engine.init_dots.failed",
                    model_root = %model_root,
                    elapsed_ms = started.elapsed().as_millis(),
                    error = %error,
                    "DeepSeek OCR Dots engine initialization failed"
                );
            }
        }
        loaded
    });

    match entry {
        Ok(engine) => Ok(Arc::clone(engine)),
        Err(error) => Err(internal_error(format!(
            "deepseek dots engine initialization failed: {}",
            error.as_ref()
        ))),
    }
}

fn dots_model_root() -> LlmResult<String> {
    resolve_model_root_for_kind(VisionModelKind::DotsOcr)
        .ok_or_else(|| internal_error("deepseek dots model root is not configured"))
}

fn get_engine_for_route_or_fallback(
    runtime: &DeepseekRuntime,
    route: RouteSelection,
) -> LlmResult<(RouteSelection, Arc<DeepseekEngine>)> {
    tracing::debug!(
        event = "llm.vision.deepseek.engine.route_resolve_start",
        route = ?route,
        "DeepSeek OCR: Resolving engine for route"
    );

    match route {
        RouteSelection::Primary => get_primary_engine(runtime).map(|engine| (route, engine)),
        RouteSelection::Dots => match get_dots_engine() {
            Ok(engine) => Ok((route, engine)),
            Err(error) => {
                tracing::warn!(
                    event = "llm.vision.deepseek.route.dots_fallback_primary",
                    error = %sanitize_error_string(error),
                    "Dots route selected but Dots engine unavailable; falling back to primary engine"
                );
                get_primary_engine(runtime).map(|engine| (RouteSelection::Primary, engine))
            }
        },
    }
}

fn get_cpu_engine_for_route_or_fallback(
    runtime: &DeepseekRuntime,
    route: RouteSelection,
) -> LlmResult<(RouteSelection, Arc<DeepseekEngine>)> {
    // Use cached CPU fallback engine to prevent memory explosion from repeated loads
    match route {
        RouteSelection::Primary => {
            let model_root = match runtime {
                DeepseekRuntime::Configured { model_root } => model_root.as_ref(),
                _ => {
                    return Err(internal_error(
                        "get_cpu_engine called on non-configured runtime",
                    ));
                }
            };

            let is_cached = ENGINE_PRIMARY_CPU.get().is_some();
            tracing::warn!(
                event = "llm.vision.deepseek.engine.cpu_fallback_start",
                route = ?route,
                engine_cached = is_cached,
                "[MEMORY TRACE] CPU fallback requested - CACHED={} (if false, will load new engine)",
                is_cached
            );

            let entry = ENGINE_PRIMARY_CPU.get_or_init(|| {
                tracing::error!(
                    event = "llm.vision.deepseek.engine.cpu_fallback_loading",
                    route = ?route,
                    "[MEMORY TRACE] *** CPU ENGINE NOT CACHED - LOADING NEW ENGINE FOR CPU FALLBACK ***"
                );

                let start = std::time::Instant::now();
                let loaded = DeepseekEngine::load_for_device(model_root, DeviceKind::Cpu)
                    .map(Arc::new)
                    .map_err(Arc::<str>::from);
                match &loaded {
                    Ok(_) => {
                        tracing::info!(
                            event = "llm.vision.deepseek.engine.cpu_fallback_loaded",
                            route = ?route,
                            elapsed_ms = start.elapsed().as_millis(),
                            "[MEMORY TRACE] CPU fallback engine loaded and cached"
                        );
                    }
                    Err(error) => {
                        tracing::error!(
                            event = "llm.vision.deepseek.engine.cpu_fallback_failed",
                            route = ?route,
                            error = %error,
                            elapsed_ms = start.elapsed().as_millis(),
                            "[MEMORY TRACE] CPU fallback engine load FAILED"
                        );
                    }
                }
                loaded
            });

            match entry {
                Ok(engine) => Ok((route, Arc::clone(engine))),
                Err(error) => Err(internal_error(format!(
                    "deepseek CPU fallback engine initialization failed: {}",
                    error.as_ref()
                ))),
            }
        }
        RouteSelection::Dots => {
            let model_root = dots_model_root()?;

            let is_cached = ENGINE_DOTS_CPU.get().is_some();
            tracing::warn!(
                event = "llm.vision.deepseek.engine.cpu_fallback_start",
                route = ?route,
                engine_cached = is_cached,
                "[MEMORY TRACE] CPU Dots fallback requested - CACHED={}",
                is_cached
            );

            let entry = ENGINE_DOTS_CPU.get_or_init(|| {
                tracing::error!(
                    event = "llm.vision.deepseek.engine.cpu_fallback_loading",
                    route = ?route,
                    "[MEMORY TRACE] *** CPU DOTS ENGINE NOT CACHED - LOADING NEW ENGINE ***"
                );

                let start = std::time::Instant::now();
                let loaded = DeepseekEngine::load_for_device_with_kind(
                    model_root.as_str(),
                    DeviceKind::Cpu,
                    VisionModelKind::DotsOcr,
                )
                .map(Arc::new)
                .map_err(Arc::<str>::from);
                match &loaded {
                    Ok(_) => {
                        tracing::info!(
                            event = "llm.vision.deepseek.engine.cpu_fallback_loaded",
                            route = ?route,
                            elapsed_ms = start.elapsed().as_millis(),
                            "[MEMORY TRACE] CPU Dots fallback engine loaded and cached"
                        );
                    }
                    Err(error) => {
                        tracing::error!(
                            event = "llm.vision.deepseek.engine.cpu_fallback_failed",
                            route = ?route,
                            error = %error,
                            elapsed_ms = start.elapsed().as_millis(),
                            "[MEMORY TRACE] CPU Dots fallback engine load FAILED"
                        );
                    }
                }
                loaded
            });

            match entry {
                Ok(engine) => Ok((route, Arc::clone(engine))),
                Err(error) => Err(internal_error(format!(
                    "deepseek CPU Dots fallback engine initialization failed: {}",
                    error.as_ref()
                ))),
            }
        }
    }
}

fn select_route(prepared: &PreparedVisionImage) -> RouteSelection {
    if !auto_route_enabled() {
        return RouteSelection::Primary;
    }
    if is_complex_image(prepared) {
        RouteSelection::Dots
    } else {
        RouteSelection::Primary
    }
}

fn auto_route_enabled() -> bool {
    parse_env_string("XIUXIAN_VISION_MODEL_KIND")
        .is_some_and(|value| value.eq_ignore_ascii_case("auto"))
}

fn is_complex_image(prepared: &PreparedVisionImage) -> bool {
    let min_tiles = parse_env_u32("XIUXIAN_VISION_AUTO_ROUTE_COMPLEX_MIN_TILES")
        .unwrap_or(DEFAULT_AUTO_COMPLEX_MIN_TILES)
        .max(1);
    let min_pixels = parse_env_u64("XIUXIAN_VISION_AUTO_ROUTE_COMPLEX_MIN_PIXELS")
        .unwrap_or(DEFAULT_AUTO_COMPLEX_MIN_PIXELS)
        .max(1);
    let image_size = parse_env_u32("XIUXIAN_VISION_IMAGE_SIZE")
        .unwrap_or(DEFAULT_IMAGE_SIZE)
        .max(1);
    let crop_mode = parse_env_bool("XIUXIAN_VISION_CROP_MODE").unwrap_or(true);
    let estimated_tiles =
        estimate_tile_count(prepared.width, prepared.height, image_size, crop_mode);
    let pixels = u64::from(prepared.width).saturating_mul(u64::from(prepared.height));
    pixels >= min_pixels || estimated_tiles >= usize::try_from(min_tiles).unwrap_or(usize::MAX)
}

fn estimate_tile_count(width: u32, height: u32, image_size: u32, crop_mode: bool) -> usize {
    if !crop_mode || image_size == 0 {
        return 1;
    }
    let tiles_w = width.saturating_add(image_size.saturating_sub(1)) / image_size;
    let tiles_h = height.saturating_add(image_size.saturating_sub(1)) / image_size;
    let local_tiles_u32 = tiles_w.saturating_mul(tiles_h).max(1);
    let local_tiles = usize::try_from(local_tiles_u32).unwrap_or(usize::MAX);
    if local_tiles > 1 {
        local_tiles.saturating_add(1)
    } else {
        local_tiles
    }
}

fn cpu_fallback_flag(route: RouteSelection) -> &'static AtomicBool {
    match route {
        RouteSelection::Primary => &FORCE_CPU_FALLBACK_PRIMARY,
        RouteSelection::Dots => &FORCE_CPU_FALLBACK_DOTS,
    }
}

fn should_retry_with_cpu_fallback(error_text: &str) -> bool {
    let lower = error_text.to_ascii_lowercase();
    lower.contains("metal") || lower.contains("cuda") || lower.contains("out of memory")
}

pub(crate) fn should_retry_with_cpu_fallback_for_tests(error_text: &str) -> bool {
    should_retry_with_cpu_fallback(error_text)
}
