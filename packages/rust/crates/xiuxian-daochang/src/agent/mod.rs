//! Core agent logic and loop implementation.

pub(crate) mod admission;
pub(crate) mod bootstrap;
mod consolidation;
mod context_budget;
mod context_budget_state;
mod embedding_runtime;
mod feedback;
mod injection;
pub(crate) mod logging;
mod mcp;
mod mcp_pool_state;
pub(crate) mod mcp_startup;
mod memory;
pub(crate) mod memory_recall;
pub(crate) mod memory_recall_feedback;
pub(crate) mod memory_recall_metrics;
pub(crate) mod memory_recall_state;
mod memory_state;
pub(crate) mod memory_stream_consumer;
pub mod native_tools;
pub mod notification;
mod omega;
mod persistence;
pub(crate) mod reflection;
mod reflection_runtime_state;
pub(crate) mod session_context;
mod system_prompt_injection_state;
mod turn_execution;
mod turn_support;
pub(crate) mod zhenfa;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use tokio::sync::RwLock;

use xiuxian_llm::embedding::runtime::EmbeddingRuntime;
use xiuxian_memory_engine::EpisodeStore;
use xiuxian_qianhuan::{HotReloadDriver, ManifestationManager};
pub use xiuxian_zhixing::ZhixingHeyi;

use crate::config::AgentConfig;
use crate::embedding::EmbeddingClient;
use crate::llm::LlmClient;
use crate::session::{BoundedSessionStore, SessionStore};
use memory_state::{MemoryStateBackend, MemoryStateLoadStatus};
pub use native_tools::NativeToolRegistry;
use reflection::PolicyHintDirective;

pub(crate) use admission::DownstreamAdmissionRuntimeSnapshot;
pub use bootstrap::ServiceMountRecord;
pub use consolidation::summarise_drained_turns;
pub use context_budget::prune_messages_for_token_budget;
pub use context_budget_state::{SessionContextBudgetClassSnapshot, SessionContextBudgetSnapshot};
pub use memory_recall_metrics::{MemoryRecallLatencyBucketsSnapshot, MemoryRecallMetricsSnapshot};
pub use memory_recall_state::{SessionMemoryRecallDecision, SessionMemoryRecallSnapshot};
pub use memory_state::MemoryRuntimeStatusSnapshot;
pub use session_context::{
    SessionContextMode, SessionContextSnapshotInfo, SessionContextStats, SessionContextWindowInfo,
};

/// Explicit session-level recall feedback direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionRecallFeedbackDirection {
    /// Feedback direction up.
    Up,
    /// Feedback direction down.
    Down,
}

/// Result of applying explicit session-level recall feedback.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SessionRecallFeedbackUpdate {
    /// Bias before the update.
    pub previous_bias: f32,
    /// Bias after the update.
    pub updated_bias: f32,
    /// Direction applied.
    pub direction: SessionRecallFeedbackDirection,
}

/// Agent: config + session store (or bounded session) + LLM client + optional MCP pool + optional memory.
pub struct Agent {
    config: AgentConfig,
    session: SessionStore,
    /// Idle-time threshold for auto reset policy (milliseconds). None disables idle reset.
    session_reset_idle_timeout_ms: Option<u64>,
    /// Last observed activity timestamp by session scope.
    session_last_activity_unix_ms: Arc<RwLock<HashMap<String, u64>>>,
    /// When set, session history is bounded; context built from recent turns.
    bounded_session: Option<BoundedSessionStore>,
    /// When set (and window enabled), consolidation stores episodes into xiuxian-memory-engine.
    memory_store: Option<Arc<EpisodeStore>>,
    /// Memory persistence backend for episode/Q state snapshots.
    memory_state_backend: Option<Arc<MemoryStateBackend>>,
    /// Startup load status for memory state persistence.
    memory_state_load_status: MemoryStateLoadStatus,
    /// Embedding client for semantic memory recall/store.
    embedding_client: Option<EmbeddingClient>,
    /// Embedding runtime policy guard (timeout/cooldown/repair).
    embedding_runtime: Option<Arc<EmbeddingRuntime>>,
    /// Most recent context-budget report by logical session id.
    context_budget_snapshots: Arc<RwLock<HashMap<String, SessionContextBudgetSnapshot>>>,
    /// Process-level memory recall metrics snapshot (for diagnostics dashboards).
    memory_recall_metrics: Arc<RwLock<memory_recall_metrics::MemoryRecallMetricsState>>,
    /// Runtime manifestation manager (owns prompt injection cache/state).
    manifestation_manager: Option<Arc<ManifestationManager>>,
    /// One-shot next-turn policy hints derived from reflection lifecycle.
    reflection_policy_hints: Arc<RwLock<HashMap<String, PolicyHintDirective>>>,
    /// Counter used by periodic memory decay policy.
    memory_decay_turn_counter: Arc<AtomicU64>,
    downstream_admission_policy: admission::DownstreamAdmissionPolicy,
    downstream_admission_metrics: admission::DownstreamAdmissionMetrics,
    llm: LlmClient,
    mcp: Option<crate::mcp::McpClientPool>,
    heyi: Option<Arc<ZhixingHeyi>>,
    native_tools: Arc<NativeToolRegistry>,
    zhenfa_tools: Option<Arc<zhenfa::ZhenfaToolBridge>>,
    memory_stream_consumer_task: Option<tokio::task::JoinHandle<()>>,
    _hot_reload_driver: Option<HotReloadDriver>,
    /// Bootstrap-time service mount records for runtime diagnostics and reporting.
    service_mount_records: Arc<RwLock<Vec<ServiceMountRecord>>>,
}

/// Test-facing recall outcome bridge for memory credit routines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TestRecallOutcome {
    /// Recall feedback indicates success.
    Success,
    /// Recall feedback indicates failure.
    Failure,
}

/// Test-facing recall credit candidate record.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TestRecalledEpisodeCandidate {
    /// Episode identifier.
    pub episode_id: String,
    /// Recall score.
    pub score: f32,
}

/// Test-facing recall credit update record.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TestRecallCreditUpdate {
    /// Episode identifier.
    pub episode_id: String,
    /// Recall score.
    pub score: f32,
    /// Credit weight.
    pub weight: f32,
    /// Previous Q value.
    pub previous_q: f32,
    /// Effective reward used for update.
    pub effective_reward: f32,
    /// Updated Q value.
    pub updated_q: f32,
}

#[must_use]
pub(crate) fn test_should_apply_decay(
    decay_enabled: bool,
    decay_every_turns: usize,
    turn_index: u64,
) -> bool {
    memory::should_apply_decay(decay_enabled, decay_every_turns, turn_index)
}

#[must_use]
pub(crate) fn test_sanitize_decay_factor(raw: f32) -> f32 {
    memory::sanitize_decay_factor(raw)
}

#[must_use]
pub(crate) fn test_select_recall_credit_candidates(
    recalled: &[(xiuxian_memory_engine::Episode, f32)],
    max_candidates: usize,
) -> Vec<TestRecalledEpisodeCandidate> {
    memory::select_recall_credit_candidates(recalled, max_candidates)
        .into_iter()
        .map(|candidate| TestRecalledEpisodeCandidate {
            episode_id: candidate.episode_id,
            score: candidate.score,
        })
        .collect()
}

#[must_use]
pub(crate) fn test_apply_recall_credit(
    store: &EpisodeStore,
    candidates: &[TestRecalledEpisodeCandidate],
    outcome: TestRecallOutcome,
) -> Vec<TestRecallCreditUpdate> {
    let internal_candidates = candidates
        .iter()
        .map(|candidate| memory::RecalledEpisodeCandidate {
            episode_id: candidate.episode_id.clone(),
            score: candidate.score,
        })
        .collect::<Vec<_>>();
    let internal_outcome = match outcome {
        TestRecallOutcome::Success => memory_recall_feedback::RecallOutcome::Success,
        TestRecallOutcome::Failure => memory_recall_feedback::RecallOutcome::Failure,
    };
    memory::apply_recall_credit(store, &internal_candidates, internal_outcome)
        .into_iter()
        .map(|update| TestRecallCreditUpdate {
            episode_id: update.episode_id,
            score: update.score,
            weight: update.weight,
            previous_q: update.previous_q,
            effective_reward: update.effective_reward,
            updated_q: update.updated_q,
        })
        .collect()
}

impl Drop for Agent {
    fn drop(&mut self) {
        if let Some(task) = self.memory_stream_consumer_task.take() {
            task.abort();
        }
    }
}

impl Agent {
    /// Returns bootstrap-time mount records for all service wiring.
    pub async fn service_mount_records(&self) -> Vec<ServiceMountRecord> {
        self.service_mount_records.read().await.clone()
    }

    /// Returns the internal `ZhixingHeyi` orchestrator if initialized.
    #[must_use]
    pub fn get_heyi(&self) -> Option<Arc<ZhixingHeyi>> {
        self.heyi.clone()
    }
}
