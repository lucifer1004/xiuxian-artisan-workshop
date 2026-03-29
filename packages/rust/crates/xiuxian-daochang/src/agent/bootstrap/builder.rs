use super::super::hot_reload::start_hot_reload_driver;
use super::super::memory::build_memory_runtime;
use super::super::qianhuan::init_persona_registries;
use super::super::service_mount::{ServiceMountCatalog, ServiceMountMeta};
use super::super::zhenfa::{
    build_global_link_graph_index, build_skill_vfs_resolver, init_zhenfa_tool_bridge,
};
use super::super::zhixing::{init_zhixing_runtime, mount_zhixing_services, resolve_project_root};
use super::native_tools::mount_native_tool_cauldron;
use super::session::{build_bounded_session_store, resolve_session_reset_idle_timeout_ms};
use super::tool_dispatch::init_tool_client_and_mount;
use crate::agent::Agent;
use crate::agent::zhenfa::ZhenfaRuntimeDeps;
use crate::config::{AgentConfig, load_runtime_settings, load_xiuxian_config};
use crate::llm::LlmClient;
use crate::session::SessionStore;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::RwLock;

impl Agent {
    /// Build agent from config.
    ///
    /// # Errors
    /// Returns an error when session backends, external tool startup, or memory backends
    /// fail to initialize.
    pub async fn from_config(config: AgentConfig) -> Result<Self> {
        let api_key = config.resolve_api_key();
        let llm = LlmClient::new(config.inference_url.clone(), config.model.clone(), api_key);
        let session = SessionStore::new()?;
        let bounded_session = build_bounded_session_store(&config)?;
        Self::build_with_backends(config, llm, session, bounded_session).await
    }

    #[doc(hidden)]
    pub async fn from_config_with_session_backends_for_test(
        config: AgentConfig,
        session: SessionStore,
        bounded_session: Option<crate::session::BoundedSessionStore>,
    ) -> Result<Self> {
        let api_key = config.resolve_api_key();
        let llm = LlmClient::new(config.inference_url.clone(), config.model.clone(), api_key);
        Self::build_with_backends(config, llm, session, bounded_session).await
    }

    async fn build_with_backends(
        config: AgentConfig,
        llm: LlmClient,
        session: SessionStore,
        bounded_session: Option<crate::session::BoundedSessionStore>,
    ) -> Result<Self> {
        let mut service_mounts = ServiceMountCatalog::new();
        let tool_runtime = init_tool_client_and_mount(&config, &mut service_mounts).await?;

        let runtime_settings = load_runtime_settings();
        let session_reset_idle_timeout_ms =
            resolve_session_reset_idle_timeout_ms(&runtime_settings);
        let xiuxian_toml_cfg = load_xiuxian_config();
        let project_root = resolve_project_root();
        let persona_registries =
            init_persona_registries(&project_root, &xiuxian_toml_cfg, &mut service_mounts);
        let memory_runtime =
            build_memory_runtime(&config, &session, &runtime_settings, &mut service_mounts)?;

        let mut native_tools = super::super::super::NativeToolRegistry::new();
        let zhixing_runtime = init_zhixing_runtime(&persona_registries, &mut service_mounts);
        let global_link_graph_index = build_global_link_graph_index(
            &xiuxian_toml_cfg,
            zhixing_runtime.as_ref(),
            &mut service_mounts,
        );
        let shared_skill_vfs_resolver = build_skill_vfs_resolver(&mut service_mounts);
        let zhenfa_deps = ZhenfaRuntimeDeps {
            manifestation_manager: zhixing_runtime
                .as_ref()
                .map(|runtime| Arc::clone(&runtime.manifestation_manager)),
            link_graph_index: global_link_graph_index.clone(),
            skill_vfs_resolver: shared_skill_vfs_resolver.as_ref().map(Arc::clone),
            embedding_client: memory_runtime.embedding_client.as_ref().map(Arc::clone),
            memory_store: memory_runtime.memory_store.clone(),
            memory_state_backend: memory_runtime.memory_state_backend.clone(),
        };
        let zhenfa_tools =
            init_zhenfa_tool_bridge(&xiuxian_toml_cfg, &zhenfa_deps, &mut service_mounts);
        let heyi = if let Some(ref runtime_bundle) = zhixing_runtime {
            mount_zhixing_services(&runtime_bundle.heyi, &mut service_mounts);
            Some(Arc::clone(&runtime_bundle.heyi))
        } else {
            service_mounts.skipped(
                "zhixing.timer_watcher",
                "scheduler",
                ServiceMountMeta::default().detail("heyi runtime unavailable"),
            );
            None
        };
        mount_native_tool_cauldron(
            heyi.as_ref(),
            shared_skill_vfs_resolver.as_ref(),
            &mut native_tools,
            &mut service_mounts,
        );
        let hot_reload_driver = start_hot_reload_driver(
            zhixing_runtime.as_ref(),
            &xiuxian_toml_cfg,
            &mut service_mounts,
        )
        .await;

        let service_mount_records = Arc::new(RwLock::new(service_mounts.finish()));

        Ok(Self {
            config,
            session,
            session_reset_idle_timeout_ms,
            session_last_activity_unix_ms: Arc::new(RwLock::new(HashMap::new())),
            bounded_session,
            memory_store: memory_runtime.memory_store,
            memory_state_backend: memory_runtime.memory_state_backend,
            memory_state_load_status: memory_runtime.memory_state_load_status,
            embedding_client: memory_runtime.embedding_client,
            embedding_runtime: memory_runtime.embedding_runtime,
            context_budget_snapshots: Arc::new(RwLock::new(HashMap::new())),
            memory_recall_metrics: Arc::new(RwLock::new(
                super::super::super::memory_recall_metrics::MemoryRecallMetricsState::default(),
            )),
            manifestation_manager: zhixing_runtime
                .as_ref()
                .map(|runtime| Arc::clone(&runtime.manifestation_manager)),
            reflection_policy_hints: Arc::new(RwLock::new(HashMap::new())),
            memory_decay_turn_counter: Arc::new(AtomicU64::new(0)),
            downstream_admission_policy:
                super::super::super::admission::DownstreamAdmissionPolicy::from_env(),
            downstream_admission_metrics:
                super::super::super::admission::DownstreamAdmissionMetrics::default(),
            llm,
            tool_runtime,
            heyi,
            native_tools: Arc::new(native_tools),
            zhenfa_tools,
            memory_stream_consumer_task: memory_runtime.memory_stream_consumer_task,
            _hot_reload_driver: hot_reload_driver,
            service_mount_records,
        })
    }
}
