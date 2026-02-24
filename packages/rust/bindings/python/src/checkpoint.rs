//! Checkpoint Store - Python Bindings for LanceDB State Persistence
//!
//! Provides checkpoint persistence for LangGraph workflows.

use omni_vector::CheckpointStore;
use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use tokio::sync::Mutex as AsyncMutex;

/// Global connection pool: path -> Arc<Mutex<CheckpointStore>>
/// Ensures same path reuses the same store instance (connection复用)
static STORE_CACHE: OnceLock<Mutex<HashMap<String, Arc<AsyncMutex<CheckpointStore>>>>> =
    OnceLock::new();
const CHECKPOINT_SCHEMA_ID: &str = "omni.checkpoint.record.v1";
static CHECKPOINT_SCHEMA_META: OnceLock<Result<(jsonschema::JSONSchema, String), String>> =
    OnceLock::new();

struct CheckpointInputView<'a> {
    table_name: &'a str,
    checkpoint_id: &'a str,
    thread_id: &'a str,
    content: &'a str,
    timestamp: f64,
    parent_id: Option<&'a str>,
    embedding: Option<&'a [f32]>,
    metadata: Option<&'a str>,
}

/// Get or create the global runtime for Python bindings
fn get_runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => panic!("Failed to create Tokio runtime for Python bindings: {error}"),
        }
    })
}

fn resolve_checkpoint_schema_json() -> Result<serde_json::Value, String> {
    let schema_raw = xiuxian_wendao::schemas::get_schema(CHECKPOINT_SCHEMA_ID)
        .ok_or_else(|| format!("Unknown checkpoint schema identifier: {CHECKPOINT_SCHEMA_ID}"))?;
    serde_json::from_str::<serde_json::Value>(schema_raw)
        .map_err(|e| format!("Invalid checkpoint schema JSON for {CHECKPOINT_SCHEMA_ID}: {e}"))
}

fn checkpoint_schema_validator() -> PyResult<&'static jsonschema::JSONSchema> {
    let init = CHECKPOINT_SCHEMA_META.get_or_init(|| {
        let schema_json = resolve_checkpoint_schema_json()?;
        let schema_id = schema_json
            .get("$id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| format!("Checkpoint schema missing '$id' for {CHECKPOINT_SCHEMA_ID}"))?;
        let validator = jsonschema::JSONSchema::compile(&schema_json)
            .map_err(|e| format!("Failed to compile checkpoint schema: {e}"))?;
        Ok((validator, schema_id))
    });

    match init {
        Ok((validator, _)) => Ok(validator),
        Err(message) => Err(pyo3::exceptions::PyRuntimeError::new_err(message.clone())),
    }
}

fn checkpoint_schema_id() -> PyResult<&'static str> {
    let _ = checkpoint_schema_validator()?;
    match CHECKPOINT_SCHEMA_META.get() {
        Some(Ok((_, schema_id))) => Ok(schema_id.as_str()),
        Some(Err(message)) => Err(pyo3::exceptions::PyRuntimeError::new_err(message.clone())),
        None => Err(pyo3::exceptions::PyRuntimeError::new_err(
            "checkpoint schema metadata not initialized",
        )),
    }
}

fn validate_checkpoint_input(input: &CheckpointInputView<'_>) -> PyResult<()> {
    let validator = checkpoint_schema_validator()?;
    let schema_instance = serde_json::json!({
        "checkpoint_id": input.checkpoint_id,
        "thread_id": input.thread_id,
        "timestamp": input.timestamp,
        "content": input.content,
        "parent_id": input.parent_id,
        "embedding": input.embedding,
        "metadata": input.metadata,
    });
    if let Err(errors) = validator.validate(&schema_instance) {
        let first = errors.into_iter().next().map_or_else(
            || "unknown checkpoint schema error".to_string(),
            |err| err.to_string(),
        );
        return Err(pyo3::exceptions::PyValueError::new_err(format!(
            "checkpoint schema violation: {first}"
        )));
    }

    if input.table_name.trim().is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "checkpoint table_name must be non-empty",
        ));
    }
    if input.checkpoint_id.trim().is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "checkpoint_id must be non-empty",
        ));
    }
    if input.thread_id.trim().is_empty() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "thread_id must be non-empty",
        ));
    }
    if !input.timestamp.is_finite() {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "timestamp must be finite",
        ));
    }
    if let Some(parent) = input.parent_id
        && parent == input.checkpoint_id
    {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "parent_id cannot equal checkpoint_id",
        ));
    }
    serde_json::from_str::<serde_json::Value>(input.content).map_err(|e| {
        pyo3::exceptions::PyValueError::new_err(format!("content must be valid JSON text: {e}"))
    })?;
    if let Some(meta) = input.metadata {
        let parsed = serde_json::from_str::<serde_json::Value>(meta).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("metadata must be valid JSON: {e}"))
        })?;
        if !parsed.is_object() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "metadata must be a JSON object",
            ));
        }
    }
    if let Some(values) = input.embedding {
        for value in values {
            if !value.is_finite() {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "embedding contains non-finite values",
                ));
            }
        }
    }
    Ok(())
}

fn timeline_timestamp_to_millis(timestamp: f64) -> Option<i64> {
    if !timestamp.is_finite() {
        return None;
    }
    format!("{timestamp:.0}").parse::<i64>().ok()
}

/// Timeline event for time-travel visualization.
/// V2.1: Aligned with TUI Visual Debugger requirements.
#[pyclass]
#[derive(Clone, Debug)]
pub struct PyTimelineEvent {
    /// Unique checkpoint identifier.
    #[pyo3(get)]
    pub checkpoint_id: String,
    /// Workflow thread identifier.
    #[pyo3(get)]
    pub thread_id: String,
    /// Monotonic workflow step number.
    #[pyo3(get)]
    pub step: i32,
    /// Event timestamp in Unix milliseconds.
    #[pyo3(get)]
    pub timestamp: f64,
    /// Human-friendly checkpoint preview text.
    #[pyo3(get)]
    pub preview: String,
    /// Parent checkpoint identifier when this is a branch.
    #[pyo3(get)]
    pub parent_checkpoint_id: Option<String>,
    /// Optional checkpoint reason tag.
    #[pyo3(get)]
    pub reason: Option<String>,
}

#[pymethods]
impl PyTimelineEvent {
    /// Format timestamp as ISO string for display
    fn iso_timestamp(&self) -> String {
        let Some(millis) = timeline_timestamp_to_millis(self.timestamp) else {
            return self.timestamp.to_string();
        };
        chrono::DateTime::from_timestamp_millis(millis)
            .map_or_else(|| self.timestamp.to_string(), |dt| dt.to_rfc3339())
    }

    /// Get relative time string (e.g., "2 minutes ago")
    fn relative_time(&self) -> String {
        let now_ms = chrono::Utc::now().timestamp_millis();
        let event_ms = timeline_timestamp_to_millis(self.timestamp).unwrap_or(now_ms);
        let diff_ms = now_ms.saturating_sub(event_ms);
        let secs = diff_ms / 1_000;

        if secs < 60 {
            format!("{secs}s ago")
        } else if secs < 3_600 {
            format!("{}m ago", secs / 60)
        } else if secs < 86_400 {
            format!("{}h ago", secs / 3_600)
        } else {
            let tenths = diff_ms / 8_640_000;
            format!("{}.{}d ago", tenths / 10, tenths % 10)
        }
    }

    /// Serialize to JSON for TUI socket communication
    fn to_json(&self) -> String {
        serde_json::json!({
            "checkpoint_id": self.checkpoint_id,
            "thread_id": self.thread_id,
            "step": self.step,
            "timestamp": self.timestamp,
            "preview": self.preview,
            "parent_checkpoint_id": self.parent_checkpoint_id,
            "reason": self.reason
        })
        .to_string()
    }

    /// Convert to Python Dict for debugging
    fn to_dict(&self, py: Python) -> PyResult<Py<PyAny>> {
        let dict = pyo3::types::PyDict::new(py);
        dict.set_item("checkpoint_id", &self.checkpoint_id)?;
        dict.set_item("thread_id", &self.thread_id)?;
        dict.set_item("step", self.step)?;
        dict.set_item("timestamp", self.timestamp)?;
        dict.set_item("preview", &self.preview)?;
        dict.set_item("parent_checkpoint_id", &self.parent_checkpoint_id)?;
        dict.set_item("reason", &self.reason)?;
        Ok(dict.into())
    }
}

/// Python wrapper for CheckpointStore (LanceDB-based state persistence)
#[pyclass]
pub struct PyCheckpointStore {
    // Cached store instance - reused across calls (path/dimension stored in Rust store)
    store: Arc<AsyncMutex<CheckpointStore>>,
}

#[pymethods]
impl PyCheckpointStore {
    #[staticmethod]
    fn checkpoint_schema_id() -> PyResult<String> {
        checkpoint_schema_id().map(ToString::to_string)
    }

    #[new]
    fn new(path: String, dimension: Option<usize>) -> PyResult<Self> {
        let dimension = dimension.unwrap_or(1536);

        // Get or create the global cache
        let cache_mutex = STORE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        let mut cache = cache_mutex.lock().map_err(|e| {
            pyo3::exceptions::PyRuntimeError::new_err(format!("Poisoned cache lock: {e}"))
        })?;

        // Check if store already exists for this path
        if let Some(store) = cache.get(&path) {
            return Ok(PyCheckpointStore {
                store: store.clone(),
            });
        }

        // Create new store
        let rt = get_runtime();
        let store = rt.block_on(async {
            CheckpointStore::new(&path, Some(dimension))
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
        })?;

        let arc_store = Arc::new(AsyncMutex::new(store));
        cache.insert(path.clone(), arc_store.clone());

        Ok(PyCheckpointStore { store: arc_store })
    }

    /// Save a checkpoint
    #[allow(
        clippy::too_many_arguments,
        reason = "Python-facing API keeps explicit checkpoint fields for stable call sites."
    )]
    #[pyo3(signature = (table_name, checkpoint_id, thread_id, content, timestamp, parent_id, embedding, metadata))]
    fn save_checkpoint(
        &self,
        table_name: String,
        checkpoint_id: String,
        thread_id: String,
        content: String,
        timestamp: f64,
        parent_id: Option<String>,
        embedding: Option<Vec<f32>>,
        metadata: Option<String>,
    ) -> PyResult<()> {
        let input = CheckpointInputView {
            table_name: &table_name,
            checkpoint_id: &checkpoint_id,
            thread_id: &thread_id,
            content: &content,
            timestamp,
            parent_id: parent_id.as_deref(),
            embedding: embedding.as_deref(),
            metadata: metadata.as_deref(),
        };
        validate_checkpoint_input(&input)?;

        let record = omni_vector::CheckpointRecord {
            checkpoint_id,
            thread_id,
            parent_id,
            timestamp,
            content,
            embedding,
            metadata, // Pass metadata from Python
        };

        let store = self.store.clone();
        let rt = get_runtime();

        rt.block_on(async {
            let guard = store.lock().await;
            guard
                .save_checkpoint(&table_name, &record)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
        })
    }

    /// Get the latest checkpoint for a thread
    fn get_latest(&self, table_name: String, thread_id: String) -> PyResult<Option<String>> {
        let store = self.store.clone();
        let rt = get_runtime();

        rt.block_on(async {
            let mut guard = store.lock().await;
            guard
                .get_latest(&table_name, &thread_id)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
        })
    }

    /// Get checkpoint by ID
    fn get_by_id(&self, table_name: String, checkpoint_id: String) -> PyResult<Option<String>> {
        let store = self.store.clone();
        let rt = get_runtime();

        rt.block_on(async {
            let mut guard = store.lock().await;
            guard
                .get_by_id(&table_name, &checkpoint_id)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
        })
    }

    /// Get checkpoint history for a thread (newest first)
    fn get_history(
        &self,
        table_name: String,
        thread_id: String,
        limit: usize,
    ) -> PyResult<Vec<String>> {
        let store = self.store.clone();
        let rt = get_runtime();

        rt.block_on(async {
            let mut guard = store.lock().await;
            guard
                .get_history(&table_name, &thread_id, limit)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
        })
    }

    /// Delete all checkpoints for a thread
    fn delete_thread(&self, table_name: String, thread_id: String) -> PyResult<u32> {
        let store = self.store.clone();
        let rt = get_runtime();

        rt.block_on(async {
            let mut guard = store.lock().await;
            guard
                .delete_thread(&table_name, &thread_id)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
        })
    }

    /// Count checkpoints for a thread
    fn count(&self, table_name: String, thread_id: String) -> PyResult<u32> {
        let store = self.store.clone();
        let rt = get_runtime();

        rt.block_on(async {
            let mut guard = store.lock().await;
            guard
                .count(&table_name, &thread_id)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
        })
    }

    /// Search for similar checkpoints using vector similarity
    ///
    /// Returns a list of JSON strings: each contains content, metadata, and distance
    fn search(
        &self,
        table_name: String,
        query_vector: Vec<f32>,
        limit: usize,
        thread_id: Option<String>,
        filter_metadata: Option<String>,
    ) -> PyResult<Vec<String>> {
        let store = self.store.clone();
        let rt = get_runtime();

        let filter = filter_metadata
            .as_ref()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok());

        rt.block_on(async {
            let mut guard = store.lock().await;
            let results = guard
                .search(
                    &table_name,
                    &query_vector,
                    limit,
                    thread_id.as_deref(),
                    filter,
                )
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

            // Convert results to JSON strings
            let json_results: Vec<String> = results
                .into_iter()
                .map(|(content, metadata, distance)| {
                    serde_json::json!({
                        "content": content,
                        "metadata": metadata,
                        "distance": distance
                    })
                    .to_string()
                })
                .collect();

            Ok(json_results)
        })
    }

    /// Get timeline for time-travel visualization.
    ///
    /// Returns a list of PyTimelineEvent objects with previews and metadata.
    /// This method is optimized for fast timeline rendering - all parsing
    /// and preview generation happens in Rust.
    ///
    /// # Arguments
    /// * `table_name` - Name of the checkpoint table
    /// * `thread_id` - Thread ID to get timeline for
    /// * `limit` - Maximum number of events to return (default 20)
    ///
    /// # Returns
    /// List of PyTimelineEvent objects sorted by timestamp descending
    fn get_timeline(
        &self,
        table_name: String,
        thread_id: String,
        limit: Option<usize>,
    ) -> PyResult<Vec<PyTimelineEvent>> {
        let limit = limit.unwrap_or(20);
        let store = self.store.clone();
        let rt = get_runtime();

        rt.block_on(async {
            let mut guard = store.lock().await;
            let records = guard
                .get_timeline_records(&table_name, &thread_id, limit)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

            // Convert to PyTimelineEvent objects
            let events: Vec<PyTimelineEvent> = records
                .into_iter()
                .map(|record| PyTimelineEvent {
                    checkpoint_id: record.checkpoint_id,
                    thread_id: record.thread_id,
                    step: record.step,
                    timestamp: record.timestamp,
                    preview: record.preview,
                    parent_checkpoint_id: record.parent_checkpoint_id,
                    reason: record.reason,
                })
                .collect();

            Ok(events)
        })
    }

    /// Get checkpoint content by ID.
    fn get_checkpoint_content(
        &self,
        table_name: String,
        checkpoint_id: String,
    ) -> PyResult<Option<String>> {
        let store = self.store.clone();
        let rt = get_runtime();

        rt.block_on(async {
            let mut guard = store.lock().await;
            guard
                .get_by_id(&table_name, &checkpoint_id)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
        })
    }

    /// Cleanup orphan checkpoints (interrupted task remnants).
    #[pyo3(signature = (table_name, dry_run=false))]
    fn cleanup_orphan_checkpoints(&self, table_name: String, dry_run: bool) -> PyResult<u32> {
        if table_name.trim().is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "checkpoint table_name must be non-empty",
            ));
        }
        let store = self.store.clone();
        let rt = get_runtime();

        rt.block_on(async {
            let mut guard = store.lock().await;
            guard
                .cleanup_orphan_checkpoints(&table_name, dry_run)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
        })
    }

    /// Force recover a checkpoint table when schema/data drift is unrecoverable.
    fn force_recover_table(&self, table_name: String) -> PyResult<()> {
        if table_name.trim().is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "checkpoint table_name must be non-empty",
            ));
        }
        let store = self.store.clone();
        let rt = get_runtime();

        rt.block_on(async {
            let guard = store.lock().await;
            guard
                .force_recover(&table_name)
                .await
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
        })
    }
}

/// Create a new checkpoint store
#[pyfunction]
pub fn create_checkpoint_store(
    path: String,
    dimension: Option<usize>,
) -> PyResult<PyCheckpointStore> {
    PyCheckpointStore::new(path, dimension)
}
