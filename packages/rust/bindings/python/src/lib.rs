#![allow(
    clippy::needless_pass_by_value,
    reason = "PyO3 boundary functions intentionally accept owned Python values."
)]
#![allow(
    clippy::must_use_candidate,
    reason = "PyO3 exports are primarily consumed from Python, not Rust call sites."
)]
#![allow(
    clippy::doc_markdown,
    reason = "Python-facing docs intentionally include function names and mixed naming."
)]
#![allow(
    clippy::missing_errors_doc,
    reason = "PyO3 wrappers map errors into Python exceptions; Rustdoc # Errors is low-value here."
)]

//! `xiuxian-core-rs` - Python bindings for the actively maintained Rust core.
//!
//! The binding surface is intentionally limited to modules whose Rust APIs are
//! currently aligned with the workspace:
//! - `xiuxian-memory-engine`
//! - `xiuxian-window`

use pyo3::prelude::*;

pub use xiuxian_memory_engine::{
    PyEpisode, PyEpisodeStore, PyIntentEncoder, PyQTable, PyStoreConfig, PyTwoPhaseConfig,
    PyTwoPhaseSearch, create_episode, create_episode_store, create_episode_with_embedding,
    create_intent_encoder, create_q_table, create_two_phase_search, py_calculate_score,
    register_memory_module,
};
pub use xiuxian_window::PySessionWindow;

/// Python module initialization.
#[pymodule]
fn xiuxian_core_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    register_memory_module(m)?;
    m.add_class::<PySessionWindow>()?;
    Ok(())
}
