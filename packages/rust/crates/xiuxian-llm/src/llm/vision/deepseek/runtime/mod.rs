use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

mod model_root;

use super::config;
#[cfg(feature = "vision-dots")]
use super::dsq_alignment::required_qoffset_alignment;
use super::model_kind::VisionModelKind;
#[cfg(feature = "vision-dots")]
use super::native::{local_runtime_may_use_metal, resolve_snapshot_path_with};
use super::remote_http::validate_ocr_http_base_url;
use super::util::non_empty_env;
#[cfg(feature = "vision-dots")]
use deepseek_ocr_dsq::DsqReader;

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
    /// `DeepSeek` OCR is delegated to a shared HTTP runtime.
    RemoteHttp {
        /// Base URL of the shared OCR gateway.
        base_url: Arc<str>,
    },
}

impl DeepseekRuntime {
    /// Returns whether `DeepSeek` OCR runtime is configured and enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Configured { .. } | Self::RemoteHttp { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LocalRuntimeSafetyDecision {
    Safe,
    UnsafeAllowedByOverride,
    UnsafeBlocked(String),
}

static DEEPSEEK_RUNTIME: OnceLock<Arc<DeepseekRuntime>> = OnceLock::new();

/// Returns the process-wide `DeepSeek` OCR runtime cache.
#[must_use]
pub fn get_deepseek_runtime() -> Arc<DeepseekRuntime> {
    Arc::clone(DEEPSEEK_RUNTIME.get_or_init(load_deepseek_runtime))
}

fn load_deepseek_runtime() -> Arc<DeepseekRuntime> {
    if let Some(base_url) = resolve_client_url() {
        tracing::info!(
            event = "llm.vision.deepseek.runtime.enabled_remote_http",
            base_url = %base_url,
            "DeepSeek OCR runtime enabled via shared HTTP gateway"
        );
        return Arc::new(DeepseekRuntime::RemoteHttp {
            base_url: Arc::from(base_url),
        });
    }

    if let Some(model_root) = model_root::resolve_model_root() {
        if let Some(reason) = resolve_local_runtime_safety_reason(model_root.as_str()) {
            tracing::warn!(
                event = "llm.vision.deepseek.runtime.disabled_unsafe_local_native",
                model_root = %model_root,
                reason = %reason,
                "DeepSeek OCR local runtime disabled by safety guard"
            );
            return Arc::new(DeepseekRuntime::Disabled {
                reason: Arc::<str>::from(reason),
            });
        }

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

fn resolve_client_url() -> Option<String> {
    let configured = non_empty_env("XIUXIAN_VISION_CLIENT_URL").or_else(config::client_url);
    configured.and_then(|raw| match validate_ocr_http_base_url(raw.as_str()) {
        Ok(value) => Some(value),
        Err(error) => {
            tracing::warn!(
                event = "llm.vision.deepseek.runtime.invalid_client_url",
                client_url = %raw,
                error = %error,
                "Ignoring invalid DeepSeek OCR shared gateway URL"
            );
            None
        }
    })
}

fn resolve_local_runtime_safety_reason(model_root: &str) -> Option<String> {
    #[cfg(not(feature = "vision-dots"))]
    {
        let _ = model_root;
        None
    }

    #[cfg(feature = "vision-dots")]
    {
        let configured_model_kind =
            non_empty_env("XIUXIAN_VISION_MODEL_KIND").or_else(config::model_kind);
        let explicit_snapshot_path = non_empty_env("XIUXIAN_VISION_SNAPSHOT_PATH")
            .or_else(config::snapshot_path)
            .map(PathBuf::from);
        let configured_device = non_empty_env("XIUXIAN_VISION_DEVICE").or_else(config::device);
        let quantized_requirement_disabled = quantized_requirement_explicitly_disabled();
        let decision = evaluate_local_runtime_safety(
            configured_model_kind.as_deref(),
            Path::new(model_root),
            explicit_snapshot_path.as_deref(),
            local_runtime_may_use_metal(),
            quantized_requirement_disabled,
            configured_device.as_deref(),
        );

        if matches!(
            decision,
            LocalRuntimeSafetyDecision::UnsafeAllowedByOverride
        ) {
            tracing::warn!(
                event = "llm.vision.deepseek.runtime.unsafe_unquantized_override",
                model_root = %model_root,
                device = configured_device.as_deref().unwrap_or("auto"),
                "DeepSeek OCR local runtime is using an explicit unquantized override without a usable DSQ snapshot on Metal"
            );
        }
        match decision {
            LocalRuntimeSafetyDecision::UnsafeBlocked(reason) => Some(reason),
            LocalRuntimeSafetyDecision::Safe
            | LocalRuntimeSafetyDecision::UnsafeAllowedByOverride => None,
        }
    }
}

fn evaluate_local_runtime_safety(
    configured_model_kind: Option<&str>,
    model_root: &Path,
    explicit_snapshot_path: Option<&Path>,
    may_use_metal: bool,
    quantized_requirement_disabled: bool,
    configured_device: Option<&str>,
) -> LocalRuntimeSafetyDecision {
    let resolved_model_kind =
        resolve_model_kind_for_model_root_with(configured_model_kind, model_root);
    let has_usable_snapshot = match resolve_snapshot_path_with(model_root, explicit_snapshot_path) {
        Some(path) => is_usable_quantized_snapshot(path.as_path()),
        None => false,
    };

    if resolved_model_kind != VisionModelKind::DotsOcr || has_usable_snapshot {
        return LocalRuntimeSafetyDecision::Safe;
    }
    if quantized_requirement_disabled {
        return LocalRuntimeSafetyDecision::UnsafeAllowedByOverride;
    }
    let device = configured_device.unwrap_or("auto");
    let host_runtime_hint = if may_use_metal {
        " this host may also select Metal unified memory"
    } else {
        ""
    };
    LocalRuntimeSafetyDecision::UnsafeBlocked(format!(
        "DeepSeek OCR local runtime is disabled because model_root={} resolves to DotsOCR without a usable .dsq snapshot. Unquantized local DotsOCR is not supported for production fallback on device='{}'; install a quantized .dsq snapshot or configure XIUXIAN_VISION_CLIENT_URL for a shared OCR gateway.{}",
        model_root.display(),
        device,
        host_runtime_hint,
    ))
}

fn quantized_requirement_explicitly_disabled() -> bool {
    std::env::var("XIUXIAN_VISION_REQUIRE_QUANTIZED")
        .ok()
        .map(|raw| raw.trim().to_ascii_lowercase())
        .is_some_and(|raw| matches!(raw.as_str(), "0" | "false" | "no" | "off"))
}

fn resolve_model_kind_for_model_root_with(
    configured_model_kind: Option<&str>,
    model_root: &Path,
) -> VisionModelKind {
    let configured = parse_model_kind_with(configured_model_kind);
    let explicit = configured_model_kind
        .and_then(|value| (!value.eq_ignore_ascii_case("auto")).then_some(value))
        .and_then(VisionModelKind::parse)
        .is_some();
    if !explicit
        && configured == VisionModelKind::Deepseek
        && model_root_looks_like_dots(model_root)
    {
        VisionModelKind::DotsOcr
    } else {
        configured
    }
}

fn parse_model_kind_with(raw: Option<&str>) -> VisionModelKind {
    if raw.is_some_and(|value| value.eq_ignore_ascii_case("auto")) {
        return VisionModelKind::DEFAULT;
    }
    raw.and_then(VisionModelKind::parse)
        .unwrap_or(VisionModelKind::DEFAULT)
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

fn is_usable_quantized_snapshot(path: &Path) -> bool {
    #[cfg(feature = "vision-dots")]
    {
        validate_quantized_snapshot_alignment(path).is_ok()
    }

    #[cfg(not(feature = "vision-dots"))]
    {
        path.is_file()
            && path
                .extension()
                .and_then(|value| value.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("dsq"))
    }
}

#[cfg(feature = "vision-dots")]
fn validate_quantized_snapshot_alignment(path: &Path) -> Result<(), ()> {
    if !path.is_file()
        || !path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("dsq"))
    {
        return Err(());
    }

    let reader = DsqReader::open(path).map_err(|_| ())?;
    for record in reader.records() {
        let alignment = required_qoffset_alignment(record.q_dtype);
        if record.q_offset % alignment != 0 {
            return Err(());
        }
    }
    Ok(())
}

pub(crate) use self::model_root::{normalize_model_root, resolve_model_root_with};
