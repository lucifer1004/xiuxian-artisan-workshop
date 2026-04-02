use crate::agent::Agent;
use crate::agent::admission::{DownstreamAdmissionMetrics, DownstreamAdmissionPolicy};
use crate::agent::bootstrap::native_tools::mount_native_tool_cauldron;
use crate::agent::bootstrap::service_mount::ServiceMountCatalog;
use crate::agent::memory_state::{MemoryStateBackend, MemoryStateLoadStatus};
use crate::config::AgentConfig;
use crate::config::load_runtime_settings;
use crate::embedding::EmbeddingClient;
use crate::llm::LlmClient;
use crate::observability::SessionEvent;
use crate::session::{BoundedSessionStore, SessionStore};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use xiuxian_llm::embedding::runtime::EmbeddingRuntime;
use xiuxian_memory_engine::{EpisodeStore, StoreConfig};

impl Agent {
    /// Build agent from config. Connects to configured external tool servers when present.
    ///
    /// # Errors
    /// Returns an error if session, external tool, or memory backends fail to initialize.
    pub async fn from_config(config: AgentConfig) -> Result<Self> {
        let api_key = config.resolve_api_key();
        let llm = LlmClient::new(config.inference_url.clone(), config.model.clone(), api_key);
        let session = SessionStore::new()?;
        let bounded_session = match config.window_max_turns {
            Some(max_turns) => Some(BoundedSessionStore::new_with_limits(
                max_turns,
                config.summary_max_segments,
                config.summary_max_chars,
            )?),
            None => None,
        };
        Self::build_with_backends(config, llm, session, bounded_session).await
    }

    #[doc(hidden)]
    /// # Errors
    /// Returns an error if session, external tool, or memory backends fail to initialize.
    pub async fn from_config_with_session_backends_for_test(
        config: AgentConfig,
        session: SessionStore,
        bounded_session: Option<BoundedSessionStore>,
    ) -> Result<Self> {
        let api_key = config.resolve_api_key();
        let llm = LlmClient::new(config.inference_url.clone(), config.model.clone(), api_key);
        Self::build_with_backends(config, llm, session, bounded_session).await
    }

    async fn build_with_backends(
        config: AgentConfig,
        llm: LlmClient,
        session: SessionStore,
        bounded_session: Option<BoundedSessionStore>,
    ) -> Result<Self> {
        let tool_runtime =
            crate::agent::tool_startup::connect_tool_pool_if_configured(&config).await?;
        let (memory_store, memory_state_backend, memory_state_load_status) =
            init_memory_backends(&config)?;
        let session_reset_idle_timeout_ms = load_runtime_settings()
            .session
            .reset_idle_timeout_mins
            .map(|minutes| minutes.saturating_mul(60_000));

        let embedding_client = config.memory.as_ref().map(|memory_cfg| {
            let base_url = memory_cfg
                .embedding_base_url
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .or_else(|| {
                    std::env::var("OMNI_AGENT_EMBED_BASE_URL")
                        .ok()
                        .map(|value| value.trim().to_string())
                        .filter(|value| !value.is_empty())
                })
                .unwrap_or_else(|| "http://127.0.0.1:3002".to_string());
            EmbeddingClient::new_with_backend_and_tuning(
                &base_url,
                15,
                memory_cfg.embedding_backend.as_deref(),
                memory_cfg.embedding_batch_max_size,
                memory_cfg.embedding_batch_max_concurrency,
            )
        });
        let memory_stream_consumer_task = config.memory.as_ref().and_then(|memory_cfg| {
            crate::agent::memory_stream_consumer::spawn_memory_stream_consumer(
                memory_cfg,
                session.redis_runtime_snapshot(),
            )
        });
        let memory_embed_timeout = duration_from_env_ms(
            "OMNI_AGENT_MEMORY_EMBED_TIMEOUT_MS",
            duration_to_u64_millis(crate::agent::DEFAULT_MEMORY_EMBED_TIMEOUT),
            crate::agent::MIN_MEMORY_EMBED_TIMEOUT_MS,
            crate::agent::MAX_MEMORY_EMBED_TIMEOUT_MS,
        );
        let memory_embed_timeout_cooldown = duration_from_env_ms(
            "OMNI_AGENT_MEMORY_EMBED_TIMEOUT_COOLDOWN_MS",
            duration_to_u64_millis(crate::agent::DEFAULT_MEMORY_EMBED_TIMEOUT_COOLDOWN),
            0,
            crate::agent::MAX_MEMORY_EMBED_COOLDOWN_MS,
        );
        let embedding_runtime = config.memory.as_ref().map(|_| {
            Arc::new(EmbeddingRuntime::new(
                memory_embed_timeout,
                memory_embed_timeout_cooldown,
            ))
        });
        let mut native_tools = crate::agent::native_tools::registry::NativeToolRegistry::new();
        let mut service_mounts = ServiceMountCatalog::new();
        mount_native_tool_cauldron(None, None, &mut native_tools, &mut service_mounts);

        Ok(Self {
            config,
            session,
            session_reset_idle_timeout_ms,
            session_last_activity_unix_ms: Arc::new(RwLock::new(HashMap::new())),
            bounded_session,
            memory_store,
            memory_state_backend,
            memory_state_load_status,
            embedding_client,
            embedding_runtime,
            context_budget_snapshots: Arc::new(RwLock::new(HashMap::new())),
            memory_recall_metrics: Arc::new(RwLock::new(
                crate::agent::memory_recall_metrics::MemoryRecallMetricsState::default(),
            )),
            memory_recall_feedback: Arc::new(RwLock::new(HashMap::new())),
            system_prompt_injection: Arc::new(RwLock::new(HashMap::new())),
            reflection_policy_hints: Arc::new(RwLock::new(HashMap::new())),
            memory_decay_turn_counter: Arc::new(AtomicU64::new(0)),
            native_tools: Arc::new(native_tools),
            manifestation_manager: None,
            heyi: None,
            zhenfa_tools: None,
            downstream_admission_policy: DownstreamAdmissionPolicy::from_env(),
            downstream_admission_metrics: DownstreamAdmissionMetrics::default(),
            memory_embed_timeout,
            memory_embed_timeout_cooldown,
            memory_embed_timeout_cooldown_until_ms: AtomicU64::new(0),
            llm,
            tool_runtime,
            memory_stream_consumer_task,
            _hot_reload_driver: None,
            service_mount_records: Arc::new(RwLock::new(Vec::new())),
        })
    }
}

type MemoryBackendInit = (
    Option<Arc<EpisodeStore>>,
    Option<Arc<MemoryStateBackend>>,
    MemoryStateLoadStatus,
);

fn init_memory_backends(config: &AgentConfig) -> Result<MemoryBackendInit> {
    let Some(memory_cfg) = config.memory.as_ref() else {
        return Ok((None, None, MemoryStateLoadStatus::NotConfigured));
    };

    let backend = MemoryStateBackend::from_config(memory_cfg)?;
    tracing::info!(
        event = SessionEvent::MemoryBackendInitialized.as_str(),
        configured_backend = %memory_cfg.persistence_backend,
        backend = backend.backend_name(),
        strict_startup = backend.strict_startup(),
        store_path = %memory_cfg.path,
        table_name = %memory_cfg.table_name,
        embedding_dim = memory_cfg.embedding_dim,
        "memory persistence backend initialized"
    );
    let store = EpisodeStore::new(StoreConfig {
        path: memory_cfg.path.clone(),
        embedding_dim: memory_cfg.embedding_dim,
        table_name: memory_cfg.table_name.clone(),
    });
    let load_started = Instant::now();
    let load_status = match backend.load(&store) {
        Ok(()) => {
            tracing::debug!(
                event = SessionEvent::MemoryStateLoadSucceeded.as_str(),
                backend = backend.backend_name(),
                strict_startup = backend.strict_startup(),
                episodes = store.len(),
                q_values = store.q_table.len(),
                duration_ms = load_started.elapsed().as_millis(),
                "memory state loaded from persistence backend"
            );
            MemoryStateLoadStatus::Loaded
        }
        Err(error) => {
            let duration_ms = load_started.elapsed().as_millis();
            if backend.strict_startup() {
                tracing::error!(
                    event = SessionEvent::MemoryStateLoadFailed.as_str(),
                    backend = backend.backend_name(),
                    strict_startup = true,
                    continue_startup = false,
                    duration_ms,
                    error = %error,
                    "strict memory backend load failed during startup"
                );
                return Err(error).context("strict valkey memory backend failed during startup");
            }
            tracing::warn!(
                event = SessionEvent::MemoryStateLoadFailed.as_str(),
                backend = backend.backend_name(),
                strict_startup = false,
                continue_startup = true,
                duration_ms,
                error = %error,
                "failed to load persisted memory state; continuing with empty memory"
            );
            MemoryStateLoadStatus::LoadFailedContinue
        }
    };

    Ok((Some(Arc::new(store)), Some(Arc::new(backend)), load_status))
}

fn duration_to_u64_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn duration_from_env_ms(name: &str, default_ms: u64, min_ms: u64, max_ms: u64) -> Duration {
    let parsed = std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
        .unwrap_or(default_ms);
    let capped = parsed.min(max_ms);
    let sanitized = if capped < min_ms { min_ms } else { capped };
    Duration::from_millis(sanitized)
}
