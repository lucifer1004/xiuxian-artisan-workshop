//! Python bindings for the memory engine primitives.

use std::sync::Arc;

use pyo3::prelude::*;
use pyo3::types::PyModule;
use serde_json::json;

use crate::{
    Episode, EpisodeStore, IntentEncoder, QTable, StoreConfig, TwoPhaseConfig, TwoPhaseSearch,
};

fn to_py_value_error(error: impl std::fmt::Display) -> PyErr {
    pyo3::exceptions::PyValueError::new_err(error.to_string())
}

fn episode_to_json(episode: &Episode) -> serde_json::Value {
    json!({
        "id": episode.id,
        "intent": episode.intent,
        "intent_embedding": episode.intent_embedding,
        "experience": episode.experience,
        "outcome": episode.outcome,
        "q_value": episode.q_value,
        "retrieval_count": episode.retrieval_count,
        "success_count": episode.success_count,
        "failure_count": episode.failure_count,
        "created_at": episode.created_at,
        "updated_at": episode.updated_at,
        "scope": episode.scope,
    })
}

fn serialize_json(value: &serde_json::Value) -> PyResult<String> {
    serde_json::to_string(value).map_err(to_py_value_error)
}

/// Python wrapper for a memory episode.
#[pyclass]
#[derive(Debug, Clone)]
pub struct PyEpisode {
    pub(crate) inner: Episode,
}

#[pymethods]
impl PyEpisode {
    #[new]
    #[pyo3(signature = (id, intent, intent_embedding, experience, outcome, scope=None))]
    fn new(
        id: String,
        intent: String,
        intent_embedding: Vec<f32>,
        experience: String,
        outcome: String,
        scope: Option<String>,
    ) -> Self {
        let inner = match scope {
            Some(scope) => {
                Episode::new_scoped(id, intent, intent_embedding, experience, outcome, scope)
            }
            None => Episode::new(id, intent, intent_embedding, experience, outcome),
        };
        Self { inner }
    }

    #[getter]
    fn id(&self) -> String {
        self.inner.id.clone()
    }

    #[getter]
    fn intent(&self) -> String {
        self.inner.intent.clone()
    }

    #[getter]
    fn intent_embedding(&self) -> Vec<f32> {
        self.inner.intent_embedding.clone()
    }

    #[getter]
    fn experience(&self) -> String {
        self.inner.experience.clone()
    }

    #[getter]
    fn outcome(&self) -> String {
        self.inner.outcome.clone()
    }

    #[getter]
    fn q_value(&self) -> f32 {
        self.inner.q_value
    }

    #[getter]
    fn success_count(&self) -> u32 {
        self.inner.success_count
    }

    #[getter]
    fn retrieval_count(&self) -> u32 {
        self.inner.retrieval_count
    }

    #[getter]
    fn failure_count(&self) -> u32 {
        self.inner.failure_count
    }

    #[getter]
    fn created_at(&self) -> i64 {
        self.inner.created_at
    }

    #[getter]
    fn updated_at(&self) -> i64 {
        self.inner.updated_at
    }

    #[getter]
    fn scope(&self) -> String {
        self.inner.scope.clone()
    }

    fn utility(&self) -> f32 {
        self.inner.utility()
    }

    fn total_uses(&self) -> u32 {
        self.inner.total_uses()
    }

    fn is_validated(&self) -> bool {
        self.inner.is_validated()
    }

    fn to_json(&self) -> PyResult<String> {
        serialize_json(&episode_to_json(&self.inner))
    }
}

/// Python-facing store configuration.
#[pyclass]
#[derive(Debug, Clone)]
pub struct PyStoreConfig {
    /// Storage path for persisted memory state.
    #[pyo3(get, set)]
    pub path: String,
    /// Embedding dimension used by the store encoder.
    #[pyo3(get, set)]
    pub embedding_dim: usize,
    /// Backing table name for persisted state files.
    #[pyo3(get, set)]
    pub table_name: String,
}

impl Default for PyStoreConfig {
    fn default() -> Self {
        let config = StoreConfig::default();
        Self {
            path: config.path,
            embedding_dim: config.embedding_dim,
            table_name: config.table_name,
        }
    }
}

impl From<PyStoreConfig> for StoreConfig {
    fn from(value: PyStoreConfig) -> Self {
        Self {
            path: value.path,
            embedding_dim: value.embedding_dim,
            table_name: value.table_name,
        }
    }
}

#[pymethods]
impl PyStoreConfig {
    #[new]
    #[pyo3(signature = (path=None, embedding_dim=None, table_name=None))]
    fn new(path: Option<String>, embedding_dim: Option<usize>, table_name: Option<String>) -> Self {
        let defaults = Self::default();
        Self {
            path: path.unwrap_or(defaults.path),
            embedding_dim: embedding_dim.unwrap_or(defaults.embedding_dim),
            table_name: table_name.unwrap_or(defaults.table_name),
        }
    }
}

/// Python wrapper for the intent encoder.
#[pyclass]
#[derive(Clone)]
pub struct PyIntentEncoder {
    pub(crate) inner: Arc<IntentEncoder>,
}

#[pymethods]
impl PyIntentEncoder {
    #[new]
    #[pyo3(signature = (dimension=384))]
    fn new(dimension: usize) -> Self {
        Self {
            inner: Arc::new(IntentEncoder::new(dimension)),
        }
    }

    fn encode(&self, intent: &str) -> Vec<f32> {
        self.inner.encode(intent)
    }

    fn cosine_similarity(&self, a: Vec<f32>, b: Vec<f32>) -> f32 {
        let a = a.into_boxed_slice();
        let b = b.into_boxed_slice();
        self.inner.cosine_similarity(a.as_ref(), b.as_ref())
    }

    fn dimension(&self) -> usize {
        self.inner.dimension()
    }
}

/// Python wrapper for the Q-table.
#[pyclass]
#[derive(Clone)]
pub struct PyQTable {
    pub(crate) inner: Arc<QTable>,
}

#[pymethods]
impl PyQTable {
    #[new]
    #[pyo3(signature = (learning_rate=None, discount_factor=None))]
    fn new(learning_rate: Option<f32>, discount_factor: Option<f32>) -> Self {
        let table = match (learning_rate, discount_factor) {
            (Some(learning_rate), Some(discount_factor)) => {
                QTable::with_params(learning_rate, discount_factor)
            }
            _ => QTable::new(),
        };
        Self {
            inner: Arc::new(table),
        }
    }

    fn update(&self, episode_id: &str, reward: f32) -> f32 {
        self.inner.update(episode_id, reward)
    }

    fn get_q(&self, episode_id: &str) -> f32 {
        self.inner.get_q(episode_id)
    }

    fn init_episode(&self, episode_id: &str) {
        self.inner.init_episode(episode_id);
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn get_all_ids(&self) -> Vec<String> {
        self.inner.get_all_ids()
    }
}

/// Python-facing two-phase search configuration.
#[pyclass]
#[derive(Debug, Clone)]
pub struct PyTwoPhaseConfig {
    /// Phase 1 candidate count.
    #[pyo3(get, set)]
    pub k1: usize,
    /// Phase 2 result count.
    #[pyo3(get, set)]
    pub k2: usize,
    /// Reranking weight for Q-values.
    #[pyo3(get, set)]
    pub lambda: f32,
}

impl Default for PyTwoPhaseConfig {
    fn default() -> Self {
        let config = TwoPhaseConfig::default();
        Self {
            k1: config.k1,
            k2: config.k2,
            lambda: config.lambda,
        }
    }
}

impl From<PyTwoPhaseConfig> for TwoPhaseConfig {
    fn from(value: PyTwoPhaseConfig) -> Self {
        Self {
            k1: value.k1,
            k2: value.k2,
            lambda: value.lambda,
        }
    }
}

#[pymethods]
impl PyTwoPhaseConfig {
    #[new]
    #[pyo3(signature = (k1=None, k2=None, lambda=None))]
    fn new(k1: Option<usize>, k2: Option<usize>, lambda: Option<f32>) -> Self {
        let defaults = Self::default();
        Self {
            k1: k1.unwrap_or(defaults.k1),
            k2: k2.unwrap_or(defaults.k2),
            lambda: lambda.unwrap_or(defaults.lambda),
        }
    }
}

/// Python wrapper for the two-phase search engine.
#[pyclass]
pub struct PyTwoPhaseSearch {
    pub(crate) inner: TwoPhaseSearch,
}

#[pymethods]
impl PyTwoPhaseSearch {
    #[new]
    #[pyo3(signature = (q_table=None, encoder=None, config=None))]
    fn new(
        q_table: Option<PyRef<'_, PyQTable>>,
        encoder: Option<PyRef<'_, PyIntentEncoder>>,
        config: Option<PyTwoPhaseConfig>,
    ) -> Self {
        let q_table =
            q_table.map_or_else(|| Arc::new(QTable::new()), |table| Arc::clone(&table.inner));
        let encoder = encoder.map_or_else(
            || Arc::new(IntentEncoder::default()),
            |encoder| Arc::clone(&encoder.inner),
        );
        let config = config.map(Into::into).unwrap_or_default();
        Self {
            inner: TwoPhaseSearch::new(q_table, encoder, config),
        }
    }

    #[pyo3(signature = (episodes_json, intent, k1=None, k2=None, lambda=None))]
    fn search_json(
        &self,
        episodes_json: &str,
        intent: &str,
        k1: Option<usize>,
        k2: Option<usize>,
        lambda: Option<f32>,
    ) -> PyResult<String> {
        let episodes: Vec<Episode> =
            serde_json::from_str(episodes_json).map_err(to_py_value_error)?;
        let results = self.inner.search(&episodes, intent, k1, k2, lambda);
        let payload = json!(
            results
                .into_iter()
                .map(
                    |(episode, score)| json!({"episode": episode_to_json(&episode), "score": score})
                )
                .collect::<Vec<_>>()
        );
        serialize_json(&payload)
    }
}

/// Python wrapper for the episode store.
#[pyclass]
pub struct PyEpisodeStore {
    pub(crate) inner: EpisodeStore,
}

#[pymethods]
impl PyEpisodeStore {
    #[new]
    #[pyo3(signature = (config=None))]
    fn new(config: Option<PyStoreConfig>) -> Self {
        let config = config.map(Into::into).unwrap_or_default();
        Self {
            inner: EpisodeStore::new(config),
        }
    }

    fn store_episode(&self, episode: &PyEpisode) -> PyResult<String> {
        self.inner
            .store(episode.inner.clone())
            .map_err(to_py_value_error)
    }

    fn store_episode_for_scope(&self, scope: &str, episode: &PyEpisode) -> PyResult<String> {
        self.inner
            .store_for_scope(scope, episode.inner.clone())
            .map_err(to_py_value_error)
    }

    fn get_episode_json(&self, episode_id: &str) -> PyResult<Option<String>> {
        self.inner
            .get(episode_id)
            .map(|episode| serialize_json(&episode_to_json(&episode)))
            .transpose()
    }

    fn get_all_episodes_json(&self) -> PyResult<String> {
        let episodes = self
            .inner
            .get_all()
            .into_iter()
            .map(|episode| episode_to_json(&episode))
            .collect::<Vec<_>>();
        serialize_json(&json!(episodes))
    }

    fn recall_json(&self, intent: &str, top_k: usize) -> PyResult<String> {
        let payload = self
            .inner
            .recall(intent, top_k)
            .into_iter()
            .map(|(episode, score)| json!({"episode": episode_to_json(&episode), "score": score}))
            .collect::<Vec<_>>();
        serialize_json(&json!(payload))
    }

    fn two_phase_recall_json(
        &self,
        intent: &str,
        k1: usize,
        k2: usize,
        lambda: f32,
    ) -> PyResult<String> {
        let payload = self
            .inner
            .two_phase_recall(intent, k1, k2, lambda)
            .into_iter()
            .map(|(episode, score)| json!({"episode": episode_to_json(&episode), "score": score}))
            .collect::<Vec<_>>();
        serialize_json(&json!(payload))
    }

    fn update_q(&self, episode_id: &str, reward: f32) -> f32 {
        self.inner.update_q(episode_id, reward)
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn stats_json(&self) -> PyResult<String> {
        let stats = self.inner.stats();
        let payload = json!({
            "total_episodes": stats.total_episodes,
            "validated_episodes": stats.validated_episodes,
            "avg_age_hours": stats.avg_age_hours,
            "q_table_size": stats.q_table_size,
        });
        serialize_json(&payload)
    }
}

/// Create a Python episode wrapper from raw episode fields.
#[pyfunction]
#[pyo3(signature = (id, intent, intent_embedding, experience, outcome, scope=None))]
#[must_use]
pub fn create_episode(
    id: String,
    intent: String,
    intent_embedding: Vec<f32>,
    experience: String,
    outcome: String,
    scope: Option<String>,
) -> PyEpisode {
    PyEpisode::new(id, intent, intent_embedding, experience, outcome, scope)
}

/// Create a Python episode wrapper and derive its embedding with an encoder.
#[pyfunction]
#[pyo3(signature = (id, intent, experience, outcome, encoder=None, scope=None))]
#[must_use]
pub fn create_episode_with_embedding(
    id: String,
    intent: String,
    experience: String,
    outcome: String,
    encoder: Option<PyRef<'_, PyIntentEncoder>>,
    scope: Option<String>,
) -> PyEpisode {
    let encoder = encoder.map_or_else(
        || Arc::new(IntentEncoder::default()),
        |encoder| Arc::clone(&encoder.inner),
    );
    let embedding = encoder.encode(&intent);
    PyEpisode::new(id, intent, embedding, experience, outcome, scope)
}

/// Create a Python episode store wrapper.
#[pyfunction]
#[pyo3(signature = (config=None))]
#[must_use]
pub fn create_episode_store(config: Option<PyStoreConfig>) -> PyEpisodeStore {
    PyEpisodeStore::new(config)
}

/// Create a Python intent encoder wrapper.
#[pyfunction]
#[pyo3(signature = (dimension=384))]
#[must_use]
pub fn create_intent_encoder(dimension: usize) -> PyIntentEncoder {
    PyIntentEncoder::new(dimension)
}

/// Create a Python Q-table wrapper.
#[pyfunction]
#[pyo3(signature = (learning_rate=None, discount_factor=None))]
#[must_use]
pub fn create_q_table(learning_rate: Option<f32>, discount_factor: Option<f32>) -> PyQTable {
    PyQTable::new(learning_rate, discount_factor)
}

/// Create a Python two-phase search wrapper.
#[pyfunction]
#[pyo3(signature = (q_table=None, encoder=None, config=None))]
#[must_use]
pub fn create_two_phase_search(
    q_table: Option<PyRef<'_, PyQTable>>,
    encoder: Option<PyRef<'_, PyIntentEncoder>>,
    config: Option<PyTwoPhaseConfig>,
) -> PyTwoPhaseSearch {
    PyTwoPhaseSearch::new(q_table, encoder, config)
}

/// Calculate the blended semantic and Q-value score.
#[pyfunction]
#[must_use]
pub fn calculate_score(similarity: f32, q_value: f32, lambda: f32) -> f32 {
    crate::calculate_score(similarity, q_value, lambda)
}

/// Register memory engine Python bindings into a Python module.
///
/// # Errors
///
/// Returns an error when one of the classes or functions cannot be registered
/// with the provided Python module.
pub fn register_memory_module(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyEpisode>()?;
    module.add_class::<PyEpisodeStore>()?;
    module.add_class::<PyIntentEncoder>()?;
    module.add_class::<PyQTable>()?;
    module.add_class::<PyStoreConfig>()?;
    module.add_class::<PyTwoPhaseConfig>()?;
    module.add_class::<PyTwoPhaseSearch>()?;
    module.add_function(wrap_pyfunction!(create_episode, module)?)?;
    module.add_function(wrap_pyfunction!(create_episode_store, module)?)?;
    module.add_function(wrap_pyfunction!(create_episode_with_embedding, module)?)?;
    module.add_function(wrap_pyfunction!(create_intent_encoder, module)?)?;
    module.add_function(wrap_pyfunction!(create_q_table, module)?)?;
    module.add_function(wrap_pyfunction!(create_two_phase_search, module)?)?;
    module.add_function(wrap_pyfunction!(calculate_score, module)?)?;
    Ok(())
}
