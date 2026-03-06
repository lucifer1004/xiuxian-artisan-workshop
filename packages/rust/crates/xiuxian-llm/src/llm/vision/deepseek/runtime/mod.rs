use std::sync::{Arc, OnceLock};

mod model_root;

/// Process-wide `DeepSeek` OCR runtime descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeepseekRuntime {
    /// `DeepSeek` OCR is disabled and a reason is provided.
    Disabled {
        /// Human-readable disabled reason for diagnostics.
        reason: Arc<str>,
    },
    /// `DeepSeek` OCR is configured via model root path.
    Configured {
        /// Filesystem root containing `DeepSeek` OCR model artifacts.
        model_root: Arc<str>,
    },
}

impl DeepseekRuntime {
    /// Returns whether `DeepSeek` OCR runtime is configured and enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Configured { .. })
    }
}

static DEEPSEEK_RUNTIME: OnceLock<Arc<DeepseekRuntime>> = OnceLock::new();

/// Returns the process-wide `DeepSeek` OCR runtime cache.
#[must_use]
pub fn get_deepseek_runtime() -> Arc<DeepseekRuntime> {
    Arc::clone(DEEPSEEK_RUNTIME.get_or_init(load_deepseek_runtime))
}

fn load_deepseek_runtime() -> Arc<DeepseekRuntime> {
    if let Some(model_root) = model_root::resolve_model_root() {
        tracing::info!(
            event = "llm.vision.deepseek.runtime.enabled",
            model_root = %model_root,
            "DeepSeek OCR runtime enabled"
        );
        Arc::new(DeepseekRuntime::Configured {
            model_root: Arc::from(model_root),
        })
    } else {
        let reason = Arc::from(
            "DeepSeek model root is not configured (XIUXIAN_VISION_MODEL_PATH, llm.vision.deepseek.model_root, or model-kind defaults under PRJ_CACHE_HOME/models and PRJ_DATA_HOME/models)",
        );
        tracing::warn!(
            event = "llm.vision.deepseek.runtime.disabled",
            reason = %reason,
            "DeepSeek OCR runtime disabled"
        );
        Arc::new(DeepseekRuntime::Disabled { reason })
    }
}

pub(crate) use self::model_root::{
    normalize_model_root, resolve_model_root_for_kind, resolve_model_root_with,
};
