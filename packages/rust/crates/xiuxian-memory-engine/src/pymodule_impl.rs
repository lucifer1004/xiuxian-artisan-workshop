use pyo3::prelude::*;
use serde_json::{Value, json};

use crate::graph::{KnowledgeGraph, SkillDoc};
use crate::kg_cache;

use super::parsers::parse_relation_type;
use super::{PyEntity, PyRelation, PySkillDoc};

/// Python wrapper for KnowledgeGraph.
#[pyclass]
#[derive(Debug, Clone)]
pub struct PyKnowledgeGraph {
    pub(crate) inner: KnowledgeGraph,
}

#[pymethods]
impl PyKnowledgeGraph {
    #[new]
    fn new() -> Self {
        Self {
            inner: KnowledgeGraph::new(),
        }
    }

    fn add_entity(&self, entity: PyEntity) -> PyResult<()> {
        self.inner
            .add_entity(entity.inner)
            .map(|_| ())
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn add_relation(&self, relation: PyRelation) -> PyResult<()> {
        self.inner
            .add_relation(relation.inner)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn search_entities(&self, query: &str, limit: i32) -> Vec<PyEntity> {
        self.inner
            .search_entities(query, limit)
            .into_iter()
            .map(|e| PyEntity { inner: e })
            .collect()
    }

    fn get_entity(&self, entity_id: &str) -> Option<PyEntity> {
        self.inner
            .get_entity(entity_id)
            .map(|e| PyEntity { inner: e })
    }

    fn get_entity_by_name(&self, name: &str) -> Option<PyEntity> {
        self.inner
            .get_entity_by_name(name)
            .map(|e| PyEntity { inner: e })
    }

    fn get_relations(
        &self,
        entity_name: Option<&str>,
        relation_type: Option<&str>,
    ) -> Vec<PyRelation> {
        let rtype = relation_type.map(parse_relation_type);
        self.inner
            .get_relations(entity_name, rtype)
            .into_iter()
            .map(|r| PyRelation { inner: r })
            .collect()
    }

    fn multi_hop_search(&self, start_name: &str, max_hops: usize) -> Vec<PyEntity> {
        self.inner
            .multi_hop_search(start_name, max_hops)
            .into_iter()
            .map(|e| PyEntity { inner: e })
            .collect()
    }

    fn get_stats(&self) -> String {
        let stats = self.inner.get_stats();
        let value = json!({
            "total_entities": stats.total_entities,
            "total_relations": stats.total_relations,
            "entities_by_type": stats.entities_by_type,
            "relations_by_type": stats.relations_by_type,
        });
        serde_json::to_string(&value).unwrap_or_else(|_| "{}".to_string())
    }

    fn clear(&mut self) {
        self.inner.clear();
    }

    fn save_to_file(&self, path: &str) -> PyResult<()> {
        self.inner
            .save_to_file(path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    fn load_from_file(&mut self, path: &str) -> PyResult<()> {
        self.inner
            .load_from_file(path)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    /// Save the graph snapshot to Valkey using `scope_key`.
    ///
    /// Invalidates the KG cache for this scope so subsequent loads see fresh data.
    #[pyo3(signature = (scope_key, dimension=1024))]
    fn save_to_valkey(&self, scope_key: &str, dimension: usize) -> PyResult<()> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        runtime
            .block_on(self.inner.save_to_valkey(scope_key, dimension))
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        kg_cache::invalidate(scope_key);
        Ok(())
    }

    /// Load the graph snapshot from Valkey by `scope_key`.
    fn load_from_valkey(&mut self, scope_key: &str) -> PyResult<()> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        runtime
            .block_on(self.inner.load_from_valkey(scope_key))
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    fn export_as_json(&self) -> PyResult<String> {
        self.inner
            .export_as_json()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn get_all_entities_json(&self) -> PyResult<String> {
        let entities = self.inner.get_all_entities();
        let entities_json: Vec<Value> = entities
            .into_iter()
            .map(|e| {
                json!({
                    "id": e.id,
                    "name": e.name,
                    "entity_type": e.entity_type.to_string(),
                    "description": e.description,
                    "source": e.source,
                    "aliases": e.aliases,
                    "confidence": e.confidence,
                })
            })
            .collect();
        serde_json::to_string(&entities_json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Batch-register skill docs as entities and relations in the graph.
    fn register_skill_entities(&self, docs: Vec<PySkillDoc>) -> PyResult<String> {
        let skill_docs: Vec<SkillDoc> = docs.into_iter().map(|d| d.inner).collect();
        let result = self
            .inner
            .register_skill_entities(&skill_docs)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let value = json!({
            "entities_added": result.entities_added,
            "relations_added": result.relations_added,
            "status": "success",
        });
        serde_json::to_string(&value)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Register skill entities from a JSON string (convenience method).
    fn register_skill_entities_json(&self, json_str: &str) -> PyResult<String> {
        let parsed: Vec<Value> = serde_json::from_str(json_str)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;

        let mut skill_docs = Vec::with_capacity(parsed.len());
        for val in &parsed {
            let doc = SkillDoc {
                id: val
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                doc_type: val
                    .get("type")
                    .or_else(|| val.get("doc_type"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                skill_name: val
                    .get("skill_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                tool_name: val
                    .get("tool_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                content: val
                    .get("content")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                routing_keywords: val
                    .get("routing_keywords")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|x| x.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
            };
            skill_docs.push(doc);
        }

        let result = self
            .inner
            .register_skill_entities(&skill_docs)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        let value = json!({
            "entities_added": result.entities_added,
            "relations_added": result.relations_added,
            "status": "success",
        });
        serde_json::to_string(&value)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Query-time tool relevance scoring via KnowledgeGraph traversal.
    #[pyo3(signature = (query_terms, max_hops = 2, limit = 10))]
    #[allow(clippy::needless_pass_by_value)]
    fn query_tool_relevance(
        &self,
        query_terms: Vec<String>,
        max_hops: usize,
        limit: usize,
    ) -> PyResult<String> {
        let results = self
            .inner
            .query_tool_relevance(&query_terms, max_hops, limit);
        let json_arr: Vec<Value> = results
            .iter()
            .map(|(name, score)| json!([name, score]))
            .collect();
        serde_json::to_string(&json_arr)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn get_all_relations_json(&self) -> PyResult<String> {
        let relations = self.inner.get_all_relations();
        let relations_json: Vec<Value> = relations
            .into_iter()
            .map(|r| {
                json!({
                    "id": r.id,
                    "source": r.source,
                    "target": r.target,
                    "relation_type": r.relation_type.to_string(),
                    "description": r.description,
                    "source_doc": r.source_doc,
                    "confidence": r.confidence,
                })
            })
            .collect();
        serde_json::to_string(&relations_json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
}

/// Invalidate the in-process KG cache for the given scope key.
///
/// Call after evicting the knowledge vector store so the long-lived process
/// does not retain the graph in memory. Safe to call when cache is empty.
#[pyfunction]
pub fn invalidate_kg_cache(scope_key: &str) {
    kg_cache::invalidate(scope_key);
}

/// Load `KnowledgeGraph` from Valkey with caching.
///
/// Uses an in-process cache keyed by path. Avoids repeated disk reads
/// when the same scope key is accessed across multiple recalls.
/// Returns None only when backend returns empty and caller chooses to ignore it.
///
/// # Errors
///
/// Returns `PyErr` when Valkey loading fails.
#[pyfunction]
pub fn load_kg_from_valkey_cached(scope_key: &str) -> PyResult<Option<PyKnowledgeGraph>> {
    match kg_cache::load_from_valkey_cached(scope_key) {
        Ok(Some(graph)) => Ok(Some(PyKnowledgeGraph { inner: graph })),
        Ok(None) => Ok(None),
        Err(e) => Err(pyo3::exceptions::PyIOError::new_err(e.to_string())),
    }
}
