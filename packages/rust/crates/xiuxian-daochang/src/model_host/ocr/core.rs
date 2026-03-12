use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use tokio::sync::Semaphore;
use xiuxian_llm::llm::vision::DeepseekRuntime;

use crate::env_parse::{parse_env_bool, parse_env_u32, parse_env_u64};

static OCR_GATE: std::sync::OnceLock<Arc<Semaphore>> = std::sync::OnceLock::new();
static OCR_RUNTIME_PREWARMED: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Copy)]
pub(crate) struct OcrTimeoutConfig {
    pub(crate) duration: Duration,
}

pub(crate) enum OcrRequestAdmission {
    Allowed {
        #[allow(dead_code)]
        guard: tokio::sync::OwnedSemaphorePermit,
    },
    Rejected,
}

pub(crate) fn deepseek_ocr_gate() -> Arc<Semaphore> {
    OCR_GATE
        .get_or_init(|| {
            let max_concurrency = parse_env_u32("XIUXIAN_VISION_OCR_MAX_CONCURRENCY").unwrap_or(1);
            Arc::new(Semaphore::new(max_concurrency as usize))
        })
        .clone()
}

pub(crate) async fn admit_deepseek_ocr_request_or_log(
    runtime: &DeepseekRuntime,
) -> OcrRequestAdmission {
    if !runtime.is_enabled() {
        return OcrRequestAdmission::Rejected;
    }

    let gate = deepseek_ocr_gate();
    match gate.try_acquire_owned() {
        Ok(guard) => OcrRequestAdmission::Allowed { guard },
        Err(_) => {
            tracing::warn!(
                event = "agent.llm.vision.deepseek.ocr.gate_rejected",
                "DeepSeek OCR request rejected because max concurrency is reached"
            );
            OcrRequestAdmission::Rejected
        }
    }
}

pub(crate) fn deepseek_ocr_max_dimension() -> u32 {
    // OPTIMIZATION: Reduced from 2048 to 1024 to stay within Metal buffer limits for multi-tile images.
    parse_env_u32("XIUXIAN_VISION_OCR_MAX_DIMENSION").unwrap_or(1024)
}

pub(crate) fn resolve_deepseek_ocr_timeout() -> OcrTimeoutConfig {
    let seconds = parse_env_u64("XIUXIAN_VISION_OCR_TIMEOUT_SECONDS").unwrap_or(120);
    OcrTimeoutConfig {
        duration: Duration::from_secs(seconds),
    }
}

pub(crate) fn resolve_deepseek_ocr_gate_timeout() -> Duration {
    let seconds = parse_env_u64("XIUXIAN_VISION_OCR_GATE_TIMEOUT_SECONDS").unwrap_or(30);
    Duration::from_secs(seconds)
}

pub(crate) fn log_deepseek_ocr_runtime_status(runtime: &DeepseekRuntime) -> bool {
    match runtime {
        DeepseekRuntime::Configured { model_root } => {
            tracing::debug!(
                event = "agent.llm.vision.deepseek.ocr.runtime_ready",
                model_root = %model_root,
                "DeepSeek OCR native runtime is configured and ready"
            );
            true
        }
        DeepseekRuntime::RemoteHttp { base_url } => {
            tracing::debug!(
                event = "agent.llm.vision.deepseek.ocr.runtime_ready_remote",
                base_url = %base_url,
                "DeepSeek OCR remote runtime is configured and ready"
            );
            true
        }
        DeepseekRuntime::Disabled { reason } => {
            tracing::info!(
                event = "agent.llm.vision.deepseek.ocr.runtime_disabled",
                reason = %reason,
                "DeepSeek OCR runtime is disabled; skipping image inference"
            );
            false
        }
    }
}

pub(crate) fn deepseek_ocr_runtime_prewarmed() -> bool {
    OCR_RUNTIME_PREWARMED.load(Ordering::Acquire)
}

pub(crate) fn mark_deepseek_ocr_runtime_prewarmed() {
    OCR_RUNTIME_PREWARMED.store(true, Ordering::Release);
}
