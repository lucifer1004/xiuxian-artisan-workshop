//! xiuxian-qianji: The Thousand Mechanisms Engine.
//!
//! A high-performance, probabilistic DAG executor based on petgraph.
//! Follows Rust 2024 Edition standards.

/// Contract definitions for nodes, instructions, and manifests.
pub mod contracts;
/// Core graph engine based on petgraph.
pub mod engine;
/// Unified error handling.
pub mod error;
/// Built-in node execution mechanisms.
pub mod executors;
/// Formal logic and safety auditing.
pub mod safety;
/// Asynchronous synaptic-flow scheduler.
pub mod scheduler;

#[cfg(feature = "pyo3")]
/// Python bindings via PyO3.
pub mod python_module;

pub use contracts::{FlowInstruction, NodeStatus, QianjiManifest, QianjiMechanism, QianjiOutput};
pub use engine::QianjiEngine;
pub use engine::compiler::QianjiCompiler;
pub use safety::QianjiSafetyGuard;
pub use scheduler::QianjiScheduler;

#[cfg(feature = "llm")]
/// Shared LLM client trait object type when `llm` feature is enabled.
pub type QianjiLlmClient = dyn xiuxian_llm::llm::LlmClient;

#[cfg(not(feature = "llm"))]
/// Placeholder trait object type when `llm` feature is disabled.
pub type QianjiLlmClient = dyn std::any::Any + Send + Sync;

/// Built-in research manifest for high-precision calibration.
pub const RESEARCH_TRINITY_TOML: &str = include_str!("../resources/research_trinity.toml");

/// Convenient entry point for deploying standard Qianji pipelines.
pub struct QianjiApp;

impl QianjiApp {
    /// Creates a standard high-precision research scheduler.
    ///
    /// This pipeline integrates Wendao knowledge search, Qianhuan persona annotation,
    /// and Synapse-Audit adversarial calibration.
    ///
    /// # Errors
    ///
    /// Returns [`error::QianjiError`] when the manifest compilation fails due to invalid
    /// topology, unsupported mechanism configuration, or dependency-related runtime checks.
    pub fn create_research_pipeline(
        index: std::sync::Arc<xiuxian_wendao::LinkGraphIndex>,
        orchestrator: std::sync::Arc<xiuxian_qianhuan::ThousandFacesOrchestrator>,
        registry: std::sync::Arc<xiuxian_qianhuan::PersonaRegistry>,
        llm_client: Option<std::sync::Arc<QianjiLlmClient>>,
    ) -> Result<QianjiScheduler, error::QianjiError> {
        let compiler = QianjiCompiler::new(index, orchestrator, registry, llm_client);
        let engine = compiler.compile(RESEARCH_TRINITY_TOML)?;
        Ok(QianjiScheduler::new(engine))
    }
}
