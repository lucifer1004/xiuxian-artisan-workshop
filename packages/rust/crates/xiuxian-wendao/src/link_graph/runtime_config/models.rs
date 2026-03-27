use super::constants::{
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_AGENT_ID,
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_EVIDENCE_PREFIX,
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_IDEMPOTENCY_SCAN_LIMIT,
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_PERSIST_RETRY_ATTEMPTS,
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_PERSIST_SUGGESTIONS_DEFAULT,
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_RELATION,
    DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_WORKER_TIME_BUDGET_MS,
    DEFAULT_LINK_GRAPH_AGENTIC_EXPANSION_MAX_CANDIDATES,
    DEFAULT_LINK_GRAPH_AGENTIC_EXPANSION_MAX_PAIRS_PER_WORKER,
    DEFAULT_LINK_GRAPH_AGENTIC_EXPANSION_MAX_WORKERS,
    DEFAULT_LINK_GRAPH_AGENTIC_EXPANSION_TIME_BUDGET_MS,
    DEFAULT_LINK_GRAPH_AGENTIC_SEARCH_PROVISIONAL_LIMIT,
    DEFAULT_LINK_GRAPH_AGENTIC_SUGGESTED_LINK_MAX_ENTRIES, DEFAULT_LINK_GRAPH_CANDIDATE_MULTIPLIER,
    DEFAULT_LINK_GRAPH_COACTIVATION_ALPHA_SCALE, DEFAULT_LINK_GRAPH_COACTIVATION_ENABLED,
    DEFAULT_LINK_GRAPH_COACTIVATION_HOP_DECAY_SCALE, DEFAULT_LINK_GRAPH_COACTIVATION_MAX_HOPS,
    DEFAULT_LINK_GRAPH_COACTIVATION_MAX_NEIGHBORS_PER_DIRECTION,
    DEFAULT_LINK_GRAPH_COACTIVATION_TOUCH_QUEUE_DEPTH, DEFAULT_LINK_GRAPH_HYBRID_MIN_HITS,
    DEFAULT_LINK_GRAPH_HYBRID_MIN_TOP_SCORE, DEFAULT_LINK_GRAPH_JULIA_ANALYZER_LAUNCHER_PATH,
    DEFAULT_LINK_GRAPH_JULIA_DEPLOYMENT_ARTIFACT_SCHEMA_VERSION, DEFAULT_LINK_GRAPH_MAX_SOURCES,
    DEFAULT_LINK_GRAPH_RELATED_MAX_CANDIDATES, DEFAULT_LINK_GRAPH_RELATED_MAX_PARTITIONS,
    DEFAULT_LINK_GRAPH_RELATED_TIME_BUDGET_MS, DEFAULT_LINK_GRAPH_RETRIEVAL_MODE,
    DEFAULT_LINK_GRAPH_ROWS_PER_SOURCE, DEFAULT_LINK_GRAPH_SEMANTIC_IGNITION_BACKEND,
    DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX,
};
use crate::link_graph::models::LinkGraphRetrievalMode;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone)]
pub(crate) struct LinkGraphCacheRuntimeConfig {
    pub valkey_url: String,
    pub key_prefix: String,
    pub ttl_seconds: Option<u64>,
}

impl LinkGraphCacheRuntimeConfig {
    pub(crate) fn from_parts(
        valkey_url: &str,
        key_prefix: Option<&str>,
        ttl_seconds: Option<u64>,
    ) -> Self {
        let resolved_url = valkey_url.trim().to_string();
        let resolved_prefix = key_prefix
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX)
            .to_string();
        Self {
            valkey_url: resolved_url,
            key_prefix: resolved_prefix,
            ttl_seconds: ttl_seconds.filter(|value| *value > 0),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LinkGraphRelatedRuntimeConfig {
    pub max_candidates: usize,
    pub max_partitions: usize,
    pub time_budget_ms: f64,
}

impl Default for LinkGraphRelatedRuntimeConfig {
    fn default() -> Self {
        Self {
            max_candidates: DEFAULT_LINK_GRAPH_RELATED_MAX_CANDIDATES,
            max_partitions: DEFAULT_LINK_GRAPH_RELATED_MAX_PARTITIONS,
            time_budget_ms: DEFAULT_LINK_GRAPH_RELATED_TIME_BUDGET_MS,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LinkGraphCoactivationRuntimeConfig {
    pub enabled: bool,
    pub alpha_scale: f64,
    pub max_neighbors_per_direction: usize,
    pub max_hops: usize,
    pub max_total_propagations: usize,
    pub hop_decay_scale: f64,
    pub touch_queue_depth: usize,
}

impl Default for LinkGraphCoactivationRuntimeConfig {
    fn default() -> Self {
        Self {
            enabled: DEFAULT_LINK_GRAPH_COACTIVATION_ENABLED,
            alpha_scale: DEFAULT_LINK_GRAPH_COACTIVATION_ALPHA_SCALE,
            max_neighbors_per_direction:
                DEFAULT_LINK_GRAPH_COACTIVATION_MAX_NEIGHBORS_PER_DIRECTION,
            max_hops: DEFAULT_LINK_GRAPH_COACTIVATION_MAX_HOPS,
            max_total_propagations: DEFAULT_LINK_GRAPH_COACTIVATION_MAX_NEIGHBORS_PER_DIRECTION
                .saturating_mul(2),
            hop_decay_scale: DEFAULT_LINK_GRAPH_COACTIVATION_HOP_DECAY_SCALE,
            touch_queue_depth: DEFAULT_LINK_GRAPH_COACTIVATION_TOUCH_QUEUE_DEPTH,
        }
    }
}

pub struct LinkGraphRetrievalPolicyRuntimeConfig {
    pub mode: LinkGraphRetrievalMode,
    pub candidate_multiplier: usize,
    pub max_sources: usize,
    pub hybrid_min_hits: usize,
    pub hybrid_min_top_score: f64,
    pub graph_rows_per_source: usize,
    pub semantic_ignition: LinkGraphSemanticIgnitionRuntimeConfig,
    pub julia_rerank: LinkGraphJuliaRerankRuntimeConfig,
}

impl Default for LinkGraphRetrievalPolicyRuntimeConfig {
    fn default() -> Self {
        Self {
            mode: LinkGraphRetrievalMode::from_alias(DEFAULT_LINK_GRAPH_RETRIEVAL_MODE)
                .unwrap_or_default(),
            candidate_multiplier: DEFAULT_LINK_GRAPH_CANDIDATE_MULTIPLIER,
            max_sources: DEFAULT_LINK_GRAPH_MAX_SOURCES,
            hybrid_min_hits: DEFAULT_LINK_GRAPH_HYBRID_MIN_HITS,
            hybrid_min_top_score: DEFAULT_LINK_GRAPH_HYBRID_MIN_TOP_SCORE,
            graph_rows_per_source: DEFAULT_LINK_GRAPH_ROWS_PER_SOURCE,
            semantic_ignition: LinkGraphSemanticIgnitionRuntimeConfig::default(),
            julia_rerank: LinkGraphJuliaRerankRuntimeConfig::default(),
        }
    }
}

/// Runtime-selectable backend for semantic ignition enrichment.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkGraphSemanticIgnitionBackend {
    /// Disable semantic ignition enrichment.
    #[default]
    Disabled,
    /// Use precomputed vectors with the Rust vector store.
    VectorStore,
    /// Resolve embeddings through an OpenAI-compatible endpoint, then query the vector store.
    OpenAiCompatible,
}

impl LinkGraphSemanticIgnitionBackend {
    /// Parse stable aliases used in runtime configuration.
    #[must_use]
    pub fn from_alias(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "off" | "none" | "disabled" => Some(Self::Disabled),
            "vector" | "vector_store" | "vector-store" | "xiuxian_vector" | "xiuxian-vector" => {
                Some(Self::VectorStore)
            }
            "openai" | "openai_compatible" | "openai-compatible" | "glm" | "glm_openai" => {
                Some(Self::OpenAiCompatible)
            }
            _ => None,
        }
    }
}

/// Runtime knobs for semantic ignition enrichment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkGraphSemanticIgnitionRuntimeConfig {
    /// Selected semantic ignition backend.
    pub backend: LinkGraphSemanticIgnitionBackend,
    /// Base path used to open the vector store.
    pub vector_store_path: Option<String>,
    /// Logical table name within the vector store.
    pub table_name: Option<String>,
    /// OpenAI-compatible embeddings base URL.
    pub embedding_base_url: Option<String>,
    /// Optional embedding model name.
    pub embedding_model: Option<String>,
}

impl Default for LinkGraphSemanticIgnitionRuntimeConfig {
    fn default() -> Self {
        Self {
            backend: LinkGraphSemanticIgnitionBackend::from_alias(
                DEFAULT_LINK_GRAPH_SEMANTIC_IGNITION_BACKEND,
            )
            .unwrap_or_default(),
            vector_store_path: None,
            table_name: None,
            embedding_base_url: None,
            embedding_model: None,
        }
    }
}

/// Runtime knobs for remote Julia rerank over Arrow IPC.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LinkGraphJuliaRerankRuntimeConfig {
    /// Base URL for the WendaoArrow-compatible Julia service.
    pub base_url: Option<String>,
    /// Arrow IPC request route.
    pub route: Option<String>,
    /// Health-check route.
    pub health_route: Option<String>,
    /// WendaoArrow schema version expected by the runtime.
    pub schema_version: Option<String>,
    /// Optional request timeout in seconds.
    pub timeout_secs: Option<u64>,
    /// Optional analyzer-owned service mode for generic analyzer launchers.
    pub service_mode: Option<String>,
    /// Optional analyzer-owned TOML path passed to the Julia service launcher.
    pub analyzer_config_path: Option<String>,
    /// Optional analyzer-owned strategy name for local or managed Julia services.
    pub analyzer_strategy: Option<String>,
    /// Optional analyzer vector weight for linear-blend strategies.
    pub vector_weight: Option<f64>,
    /// Optional analyzer similarity weight for linear-blend strategies.
    pub similarity_weight: Option<f64>,
}

/// Additive analyzer-owned launch inputs resolved from `julia_rerank` runtime config.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LinkGraphJuliaAnalyzerServiceDescriptor {
    /// Generic analyzer service mode, usually `stream` or `table`.
    pub service_mode: Option<String>,
    /// Optional path to analyzer-local TOML configuration.
    pub analyzer_config_path: Option<String>,
    /// Optional analyzer strategy override.
    pub analyzer_strategy: Option<String>,
    /// Optional analyzer vector weight.
    pub vector_weight: Option<f64>,
    /// Optional analyzer similarity weight.
    pub similarity_weight: Option<f64>,
}

/// Resolved Julia service launch manifest derived from runtime configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkGraphJuliaAnalyzerLaunchManifest {
    /// Launcher path relative to the repository root.
    pub launcher_path: String,
    /// Ordered analyzer-owned CLI args.
    pub args: Vec<String>,
}

/// Serializable deployment artifact for a resolved Julia rerank service.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LinkGraphJuliaDeploymentArtifact {
    /// Artifact-level schema version for deployment inspection surfaces.
    pub artifact_schema_version: String,
    /// RFC3339 timestamp recording when the deployment artifact was rendered.
    pub generated_at: String,
    /// Resolved Julia service base URL.
    pub base_url: Option<String>,
    /// Arrow IPC route expected by the service.
    pub route: Option<String>,
    /// Health-check route expected by the service.
    pub health_route: Option<String>,
    /// WendaoArrow schema version expected by Rust.
    pub schema_version: Option<String>,
    /// Optional request timeout in seconds.
    pub timeout_secs: Option<u64>,
    /// Resolved analyzer launch manifest.
    pub launch: LinkGraphJuliaAnalyzerLaunchManifest,
}

impl LinkGraphJuliaRerankRuntimeConfig {
    /// Build the analyzer-owned launch descriptor from runtime configuration.
    #[must_use]
    pub fn analyzer_service_descriptor(&self) -> LinkGraphJuliaAnalyzerServiceDescriptor {
        LinkGraphJuliaAnalyzerServiceDescriptor {
            service_mode: self.service_mode.clone(),
            analyzer_config_path: self.analyzer_config_path.clone(),
            analyzer_strategy: self.analyzer_strategy.clone(),
            vector_weight: self.vector_weight,
            similarity_weight: self.similarity_weight,
        }
    }

    /// Build the analyzer launch manifest from runtime configuration.
    #[must_use]
    pub fn analyzer_launch_manifest(&self) -> LinkGraphJuliaAnalyzerLaunchManifest {
        let descriptor = self.analyzer_service_descriptor();
        let mut args = Vec::new();

        if let Some(service_mode) = descriptor.service_mode {
            args.push("--service-mode".to_string());
            args.push(service_mode);
        }
        if let Some(config_path) = descriptor.analyzer_config_path {
            args.push("--analyzer-config".to_string());
            args.push(config_path);
        }
        if let Some(strategy) = descriptor.analyzer_strategy {
            args.push("--analyzer-strategy".to_string());
            args.push(strategy);
        }
        if let Some(vector_weight) = descriptor.vector_weight {
            args.push("--vector-weight".to_string());
            args.push(vector_weight.to_string());
        }
        if let Some(similarity_weight) = descriptor.similarity_weight {
            args.push("--similarity-weight".to_string());
            args.push(similarity_weight.to_string());
        }

        LinkGraphJuliaAnalyzerLaunchManifest {
            launcher_path: DEFAULT_LINK_GRAPH_JULIA_ANALYZER_LAUNCHER_PATH.to_string(),
            args,
        }
    }

    /// Build the serializable deployment artifact from runtime configuration.
    #[must_use]
    pub fn deployment_artifact(&self) -> LinkGraphJuliaDeploymentArtifact {
        LinkGraphJuliaDeploymentArtifact {
            artifact_schema_version: DEFAULT_LINK_GRAPH_JULIA_DEPLOYMENT_ARTIFACT_SCHEMA_VERSION
                .to_string(),
            generated_at: Utc::now().to_rfc3339(),
            base_url: self.base_url.clone(),
            route: self.route.clone(),
            health_route: self.health_route.clone(),
            schema_version: self.schema_version.clone(),
            timeout_secs: self.timeout_secs,
            launch: self.analyzer_launch_manifest(),
        }
    }
}

impl LinkGraphJuliaDeploymentArtifact {
    /// Render the deployment artifact as pretty TOML.
    ///
    /// # Errors
    ///
    /// Returns an error when the deployment artifact cannot be serialized into
    /// TOML.
    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    /// Render the deployment artifact as pretty JSON.
    ///
    /// # Errors
    ///
    /// Returns an error when the deployment artifact cannot be serialized into
    /// JSON.
    pub fn to_json_string(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Persist the deployment artifact to a TOML file.
    ///
    /// Parent directories are created when they do not yet exist.
    ///
    /// # Errors
    ///
    /// Returns an error when serialization fails, when parent directories
    /// cannot be created, or when the artifact file cannot be written.
    pub fn write_toml_file<P>(&self, path: P) -> std::io::Result<()>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }

        let encoded = self.to_toml_string().map_err(std::io::Error::other)?;
        std::fs::write(path, encoded)
    }

    /// Persist the deployment artifact to a JSON file.
    ///
    /// Parent directories are created when they do not yet exist.
    ///
    /// # Errors
    ///
    /// Returns an error when serialization fails, when parent directories
    /// cannot be created, or when the artifact file cannot be written.
    pub fn write_json_file<P>(&self, path: P) -> std::io::Result<()>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }

        let encoded = self.to_json_string().map_err(std::io::Error::other)?;
        std::fs::write(path, encoded)
    }
}

#[derive(Debug, Clone)]
pub struct LinkGraphAgenticRuntimeConfig {
    pub suggested_link_max_entries: usize,
    pub suggested_link_ttl_seconds: Option<u64>,
    pub search_include_provisional_default: bool,
    pub search_provisional_limit: usize,
    pub expansion_max_workers: usize,
    pub expansion_max_candidates: usize,
    pub expansion_max_pairs_per_worker: usize,
    pub expansion_time_budget_ms: f64,
    pub execution_worker_time_budget_ms: f64,
    pub execution_persist_suggestions_default: bool,
    pub execution_persist_retry_attempts: usize,
    pub execution_idempotency_scan_limit: usize,
    pub execution_relation: String,
    pub execution_agent_id: String,
    pub execution_evidence_prefix: String,
}

impl Default for LinkGraphAgenticRuntimeConfig {
    fn default() -> Self {
        Self {
            suggested_link_max_entries: DEFAULT_LINK_GRAPH_AGENTIC_SUGGESTED_LINK_MAX_ENTRIES,
            suggested_link_ttl_seconds: None,
            search_include_provisional_default: false,
            search_provisional_limit: DEFAULT_LINK_GRAPH_AGENTIC_SEARCH_PROVISIONAL_LIMIT,
            expansion_max_workers: DEFAULT_LINK_GRAPH_AGENTIC_EXPANSION_MAX_WORKERS,
            expansion_max_candidates: DEFAULT_LINK_GRAPH_AGENTIC_EXPANSION_MAX_CANDIDATES,
            expansion_max_pairs_per_worker:
                DEFAULT_LINK_GRAPH_AGENTIC_EXPANSION_MAX_PAIRS_PER_WORKER,
            expansion_time_budget_ms: DEFAULT_LINK_GRAPH_AGENTIC_EXPANSION_TIME_BUDGET_MS,
            execution_worker_time_budget_ms:
                DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_WORKER_TIME_BUDGET_MS,
            execution_persist_suggestions_default:
                DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_PERSIST_SUGGESTIONS_DEFAULT,
            execution_persist_retry_attempts:
                DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_PERSIST_RETRY_ATTEMPTS,
            execution_idempotency_scan_limit:
                DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_IDEMPOTENCY_SCAN_LIMIT,
            execution_relation: DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_RELATION.to_string(),
            execution_agent_id: DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_AGENT_ID.to_string(),
            execution_evidence_prefix: DEFAULT_LINK_GRAPH_AGENTIC_EXECUTION_EVIDENCE_PREFIX
                .to_string(),
        }
    }
}

#[derive(Debug, Clone, Default)]
/// Resolved `LinkGraph` index scope derived from runtime configuration.
pub struct LinkGraphIndexRuntimeConfig {
    /// Relative include directories used for index scope.
    pub include_dirs: Vec<String>,
    /// Relative directory names excluded from indexing.
    pub exclude_dirs: Vec<String>,
}
