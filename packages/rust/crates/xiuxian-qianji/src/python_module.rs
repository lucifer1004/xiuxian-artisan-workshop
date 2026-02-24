use crate::engine::QianjiEngine;
use crate::executors::MockMechanism;
use crate::scheduler::QianjiScheduler;
use pyo3::prelude::*;
use std::sync::Arc;

#[pyclass(name = "QianjiEngine")]
pub struct PyQianjiEngine {
    pub inner: QianjiEngine,
}

#[pymethods]
impl PyQianjiEngine {
    #[new]
    pub fn new() -> Self {
        Self {
            inner: QianjiEngine::new(),
        }
    }

    /// Adds a mock node for testing from Python.
    /// In production, we'd add real mechanisms like Seeker/Annotator.
    pub fn add_mock_node(&mut self, id: String, weight: f32) -> usize {
        let mech = Arc::new(MockMechanism {
            name: id.clone(),
            weight,
        });
        self.inner.add_mechanism(&id, mech).index()
    }

    pub fn add_link(&mut self, from: usize, to: usize, label: Option<String>, weight: f32) {
        use petgraph::stable_graph::NodeIndex;
        self.inner.add_link(
            NodeIndex::new(from),
            NodeIndex::new(to),
            label.as_deref(),
            weight,
        );
    }
}

#[pyclass(name = "QianjiScheduler")]
pub struct PyQianjiScheduler {
    pub inner: QianjiScheduler,
}

#[pymethods]
impl PyQianjiScheduler {
    #[new]
    pub fn new(engine: &PyQianjiEngine) -> Self {
        // Cloning the engine into the scheduler
        // In a real scenario, we might want to share ownership better
        Self {
            inner: QianjiScheduler::new(QianjiEngine {
                graph: engine.inner.graph.clone(),
            }),
        }
    }

    /// Runs the scheduler asynchronously from Python.
    pub fn run(&self, py: Python<'_>, context_json: String) -> PyResult<String> {
        let context: serde_json::Value = serde_json::from_str(&context_json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        py.allow_threads(|| {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt
                .block_on(self.inner.run(context))
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

            serde_json::to_string(&result)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
        })
    }
}

#[pymodule]
fn _xiuxian_qianji(py: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyQianjiEngine>()?;
    m.add_class::<PyQianjiScheduler>()?;
    Ok(())
}
