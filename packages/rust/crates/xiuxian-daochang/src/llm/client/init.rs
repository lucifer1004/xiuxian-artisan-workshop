use std::sync::Arc;
use std::sync::Once;
use std::time::{Duration, Instant};

use tokio::sync::Semaphore;
use xiuxian_llm::llm::vision::deepseek::prewarm_deepseek_ocr;
use xiuxian_llm::llm::vision::{DeepseekRuntime, get_deepseek_runtime};
use xiuxian_macros::env_non_empty;

use crate::config::load_runtime_settings;
use crate::llm::backend::{extract_api_base_from_inference_url, parse_backend_mode};
use crate::llm::client::LlmClient;
#[cfg(feature = "agent-provider-litellm")]
use crate::llm::compat::litellm::LiteLlmRuntime;
#[cfg(feature = "agent-provider-litellm")]
use crate::llm::compat::litellm_ocr::mark_deepseek_ocr_runtime_prewarmed;
use crate::llm::providers::{ProviderSettings, resolve_provider_settings};

static DEEPSEEK_VISION_STARTUP_PROBE_ONCE: Once = Once::new();

impl LlmClient {
    pub fn new(inference_url: String, model: String, api_key: Option<String>) -> Self {
        let runtime_settings = load_runtime_settings();
        let env_backend = env_non_empty!("OMNI_AGENT_LLM_BACKEND");
        let (backend_mode, backend_source) = if let Some(raw) = env_backend.as_deref() {
            (parse_backend_mode(Some(raw)), "env")
        } else {
            let settings_backend = runtime_settings
                .agent
                .llm_backend
                .as_deref()
                .map(str::trim)
                .map(ToString::to_string)
                .filter(|raw| !raw.is_empty());
            if let Some(raw) = settings_backend.as_deref() {
                (parse_backend_mode(Some(raw)), "settings")
            } else {
                (parse_backend_mode(None), "default")
            }
        };
        let provider_settings = resolve_provider_settings(&runtime_settings, model);
        let ProviderSettings {
            mode: litellm_provider_mode,
            wire_api: litellm_wire_api,
            source: litellm_provider_source,
            api_key: provider_api_key,
            api_key_env: litellm_api_key_env,
            minimax_api_base,
            model,
            timeout_secs: inference_timeout_secs,
            max_tokens: inference_max_tokens,
            max_in_flight: inference_max_in_flight,
        } = provider_settings;
        let api_key = provider_api_key.or(api_key);
        let in_flight_gate = inference_max_in_flight.map(|limit| Arc::new(Semaphore::new(limit)));
        let inference_api_base = extract_api_base_from_inference_url(&inference_url);
        tracing::info!(
            llm_backend = backend_mode.as_str(),
            llm_backend_source = backend_source,
            litellm_provider = litellm_provider_mode.as_str(),
            litellm_wire_api = litellm_wire_api.as_str(),
            litellm_provider_source = litellm_provider_source,
            litellm_api_key_env = %litellm_api_key_env,
            minimax_api_base = %minimax_api_base,
            inference_timeout_secs = inference_timeout_secs,
            inference_max_tokens = inference_max_tokens,
            inference_max_in_flight = inference_max_in_flight,
            model = %model,
            inference_api_base = %inference_api_base,
            "llm backend selected"
        );
        Self {
            client: build_http_client(),
            inference_url,
            #[cfg(feature = "agent-provider-litellm")]
            inference_api_base,
            model,
            api_key,
            backend_mode,
            backend_source,
            litellm_provider_mode,
            litellm_wire_api,
            litellm_provider_source,
            #[cfg(feature = "agent-provider-litellm")]
            litellm_api_key_env,
            #[cfg(feature = "agent-provider-litellm")]
            minimax_api_base,
            inference_timeout_secs,
            inference_max_tokens,
            inference_max_in_flight,
            in_flight_gate,
            #[cfg(feature = "agent-provider-litellm")]
            litellm_runtime: LiteLlmRuntime::new(),
        }
    }

    /// Active backend mode label (`litellm_rs` or `http`).
    pub fn backend_mode(&self) -> &'static str {
        self.backend_mode.as_str()
    }

    /// Backend source label (`env`, `settings`, or `default`).
    pub fn backend_source(&self) -> &'static str {
        self.backend_source
    }

    /// Active litellm provider mode (`openai` or `minimax`).
    pub fn litellm_provider_mode(&self) -> &'static str {
        self.litellm_provider_mode.as_str()
    }

    /// Active wire protocol (`chat_completions` or `responses`).
    pub fn litellm_wire_api(&self) -> &'static str {
        self.litellm_wire_api.as_str()
    }

    /// litellm provider source (`env`, `runtime_settings`, `default`).
    pub fn litellm_provider_source(&self) -> &'static str {
        self.litellm_provider_source
    }
}

/// Run the `DeepSeek` vision startup probe at most once per process.
///
/// `trigger` is a stable label describing the startup path (for example
/// `telegram`, `discord`, or `gateway`) used only for observability logs.
pub fn run_deepseek_vision_startup_probe_once(trigger: &'static str) {
    DEEPSEEK_VISION_STARTUP_PROBE_ONCE.call_once(|| {
        log_deepseek_vision_startup_probe(trigger);
    });
}

fn log_deepseek_vision_startup_probe(trigger: &'static str) {
    let runtime = get_deepseek_runtime();
    match runtime.as_ref() {
        DeepseekRuntime::Configured { model_root } => {
            tracing::info!(
                event = "agent.llm.vision.deepseek.startup_probe",
                status = "enabled",
                trigger,
                model_root = %model_root,
                "DeepSeek OCR startup probe finished: runtime enabled"
            );
            if deepseek_startup_prewarm_enabled() {
                tracing::info!(
                    event = "agent.llm.vision.deepseek.startup_prewarm",
                    status = "scheduled",
                    trigger,
                    model_root = %model_root,
                    "DeepSeek OCR startup prewarm scheduled in background"
                );
                spawn_deepseek_startup_prewarm(trigger, model_root, Arc::clone(&runtime));
            } else {
                tracing::info!(
                    event = "agent.llm.vision.deepseek.startup_prewarm",
                    status = "skipped",
                    trigger,
                    reason = "disabled_by_env",
                    "DeepSeek OCR startup prewarm skipped by OMNI_AGENT_DEEPSEEK_OCR_PREWARM"
                );
            }
        }
        DeepseekRuntime::Disabled { reason } => {
            tracing::warn!(
                event = "agent.llm.vision.deepseek.startup_probe",
                status = "disabled",
                trigger,
                reason = %reason,
                "DeepSeek OCR startup probe finished: runtime disabled"
            );
        }
    }
}

fn spawn_deepseek_startup_prewarm(
    trigger: &'static str,
    model_root: &str,
    runtime: Arc<DeepseekRuntime>,
) {
    let thread_model_root = model_root.to_string();
    let spawn_result = std::thread::Builder::new()
        .name("deepseek-startup-prewarm".to_string())
        .spawn(move || {
            let started = Instant::now();
            let prewarm_outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                prewarm_deepseek_ocr(runtime.as_ref())
            }));
            match prewarm_outcome {
                Ok(Ok(())) => {
                    #[cfg(feature = "agent-provider-litellm")]
                    mark_deepseek_ocr_runtime_prewarmed();
                    tracing::info!(
                        event = "agent.llm.vision.deepseek.startup_prewarm",
                        status = "ready",
                        trigger,
                        elapsed_ms = started.elapsed().as_millis(),
                        model_root = %thread_model_root,
                        "DeepSeek OCR startup prewarm completed"
                    );
                }
                Ok(Err(error)) => {
                    tracing::warn!(
                        event = "agent.llm.vision.deepseek.startup_prewarm",
                        status = "failed",
                        trigger,
                        elapsed_ms = started.elapsed().as_millis(),
                        model_root = %thread_model_root,
                        error = %error,
                        "DeepSeek OCR startup prewarm failed; runtime stays best-effort"
                    );
                }
                Err(payload) => {
                    tracing::warn!(
                        event = "agent.llm.vision.deepseek.startup_prewarm",
                        status = "panicked",
                        trigger,
                        elapsed_ms = started.elapsed().as_millis(),
                        model_root = %thread_model_root,
                        panic = %panic_payload_to_message(payload.as_ref()),
                        "DeepSeek OCR startup prewarm panicked; runtime stays best-effort"
                    );
                }
            }
        });
    if let Err(error) = spawn_result {
        tracing::warn!(
            event = "agent.llm.vision.deepseek.startup_prewarm",
            status = "spawn_failed",
            trigger,
            model_root = %model_root,
            error = %error,
            "DeepSeek OCR startup prewarm worker failed to start"
        );
    }
}

fn panic_payload_to_message(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        return (*message).to_string();
    }
    if let Some(message) = payload.downcast_ref::<String>() {
        return message.clone();
    }
    "unknown panic payload".to_string()
}

fn deepseek_startup_prewarm_enabled() -> bool {
    env_non_empty!("OMNI_AGENT_DEEPSEEK_OCR_PREWARM")
        .map(|raw| raw.trim().to_ascii_lowercase())
        .is_none_or(|raw| !matches!(raw.as_str(), "0" | "false" | "no" | "off"))
}

fn build_http_client() -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .pool_idle_timeout(Duration::from_secs(90))
        .pool_max_idle_per_host(64)
        .tcp_nodelay(true);
    if !system_proxy_enabled() {
        builder = builder.no_proxy();
    }
    match builder.build() {
        Ok(client) => client,
        Err(error) => {
            tracing::warn!(
                error = %error,
                "failed to build tuned llm http client; falling back to default client"
            );
            reqwest::Client::new()
        }
    }
}

fn system_proxy_enabled() -> bool {
    env_non_empty!("OMNI_AGENT_HTTP_ENABLE_SYSTEM_PROXY")
        .map(|raw| raw.trim().to_ascii_lowercase())
        .is_some_and(|raw| matches!(raw.as_str(), "1" | "true" | "yes" | "on"))
}
