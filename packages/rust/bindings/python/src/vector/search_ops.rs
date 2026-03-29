//! Search Operations - Vector and hybrid search helper functions
//!
//! Contains: search_optimized, search_hybrid, create_index,
//!           search_tools, load_tool_registry, scan_skill_tools_raw

use pyo3::{
    prelude::*,
    types::{PyAny, PyDict, PyList},
};
use serde::Deserialize;
use serde_json::{Map as JsonMap, Value as JsonValue};
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;
use xiuxian_vector::{SearchOptions, ToolSearchOptions, VectorStore};

fn json_value_to_py(py: pyo3::Python<'_>, value: &serde_json::Value) -> PyResult<Py<PyAny>> {
    match value {
        serde_json::Value::Null => Ok(py.None()),
        serde_json::Value::Bool(v) => Ok(v.into_pyobject(py)?.to_owned().into_any().unbind()),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_pyobject(py)?.into_any().unbind())
            } else if let Some(u) = n.as_u64() {
                Ok(u.into_pyobject(py)?.into_any().unbind())
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_pyobject(py)?.into_any().unbind())
            } else {
                Ok(py.None())
            }
        }
        serde_json::Value::String(s) => Ok(s.into_pyobject(py)?.into_any().unbind()),
        serde_json::Value::Array(arr) => {
            let list = PyList::empty(py);
            for item in arr {
                list.append(json_value_to_py(py, item)?)?;
            }
            Ok(list.into_any().unbind())
        }
        serde_json::Value::Object(map) => {
            let dict = PyDict::new(py);
            for (k, v) in map {
                dict.set_item(k, json_value_to_py(py, v)?)?;
            }
            Ok(dict.into_any().unbind())
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct ConfidenceProfile {
    high_threshold: f32,
    medium_threshold: f32,
    high_base: f32,
    high_scale: f32,
    high_cap: f32,
    medium_base: f32,
    medium_scale: f32,
    medium_cap: f32,
    low_floor: f32,
}

impl Default for ConfidenceProfile {
    fn default() -> Self {
        Self {
            high_threshold: 0.75,
            medium_threshold: 0.5,
            high_base: 0.90,
            high_scale: 0.05,
            high_cap: 0.99,
            medium_base: 0.60,
            medium_scale: 0.30,
            medium_cap: 0.89,
            low_floor: 0.10,
        }
    }
}

impl ConfidenceProfile {
    fn sanitize(mut self) -> Self {
        if self.high_threshold < self.medium_threshold {
            std::mem::swap(&mut self.high_threshold, &mut self.medium_threshold);
        }
        if self.high_cap < self.high_base {
            self.high_cap = self.high_base;
        }
        if self.medium_cap < self.medium_base {
            self.medium_cap = self.medium_base;
        }
        self.low_floor = self.low_floor.clamp(0.0, 1.0);
        self
    }
}

fn calibrate_confidence(score: f32, profile: &ConfidenceProfile) -> (&'static str, f32) {
    if score >= profile.high_threshold {
        (
            "high",
            (profile.high_base + score * profile.high_scale).min(profile.high_cap),
        )
    } else if score >= profile.medium_threshold {
        (
            "medium",
            (profile.medium_base + score * profile.medium_scale).min(profile.medium_cap),
        )
    } else {
        ("low", score.max(profile.low_floor))
    }
}

const TOOL_SEARCH_COMMON_SCHEMA_JSON: &str =
    include_str!("../../../../crates/xiuxian-vector/resources/xiuxian.vector.tool_search.v1.schema.json");

#[derive(Debug, Clone)]
struct CommonSchemaRules {
    required: std::collections::HashSet<String>,
    allowed: std::collections::HashSet<String>,
}

fn tool_search_common_rules() -> &'static CommonSchemaRules {
    static RULES: OnceLock<CommonSchemaRules> = OnceLock::new();
    RULES.get_or_init(|| {
        let root: JsonValue = serde_json::from_str(TOOL_SEARCH_COMMON_SCHEMA_JSON)
            .unwrap_or_else(|_| serde_json::json!({}));
        let required = root
            .get("required")
            .and_then(JsonValue::as_array)
            .into_iter()
            .flatten()
            .filter_map(JsonValue::as_str)
            .map(ToString::to_string)
            .collect::<std::collections::HashSet<_>>();
        let allowed = root
            .get("properties")
            .and_then(JsonValue::as_object)
            .map(|obj| {
                obj.keys()
                    .cloned()
                    .collect::<std::collections::HashSet<_>>()
            })
            .unwrap_or_default();
        CommonSchemaRules { required, allowed }
    })
}

fn validate_type(type_name: &str, value: &JsonValue) -> bool {
    match type_name {
        "string" => value.is_string(),
        "number" => value.is_number(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "null" => value.is_null(),
        _ => true,
    }
}

fn validate_field_spec(key: &str, value: &JsonValue, spec: &JsonValue) -> Result<(), String> {
    if let Some(enum_vals) = spec.get("enum").and_then(JsonValue::as_array)
        && !enum_vals.iter().any(|v| v == value)
    {
        return Err(format!("field '{key}' value is outside enum"));
    }
    if let Some(min_len) = spec.get("minLength").and_then(JsonValue::as_u64)
        && let Some(s) = value.as_str()
        && s.len() < min_len as usize
    {
        return Err(format!("field '{key}' violates minLength={min_len}"));
    }

    if let Some(type_name) = spec.get("type").and_then(JsonValue::as_str)
        && !validate_type(type_name, value)
    {
        return Err(format!("field '{key}' type mismatch, expected {type_name}"));
    }

    if let Some(any_of) = spec.get("anyOf").and_then(JsonValue::as_array) {
        let any_match = any_of.iter().any(|branch| {
            branch
                .get("type")
                .and_then(JsonValue::as_str)
                .is_some_and(|t| validate_type(t, value))
        });
        if !any_match {
            return Err(format!("field '{key}' type mismatch for anyOf"));
        }
    }
    Ok(())
}

fn validate_tool_search_common_schema(payload: &JsonMap<String, JsonValue>) -> Result<(), String> {
    let rules = tool_search_common_rules();
    for field in &rules.required {
        if !payload.contains_key(field) {
            return Err(format!("missing required field '{field}'"));
        }
    }
    for key in payload.keys() {
        if !rules.allowed.contains(key) {
            return Err(format!("unknown field '{key}' (not in common schema)"));
        }
    }

    let schema_root: JsonValue = serde_json::from_str(TOOL_SEARCH_COMMON_SCHEMA_JSON)
        .unwrap_or_else(|_| serde_json::json!({}));
    let properties = schema_root
        .get("properties")
        .and_then(JsonValue::as_object)
        .ok_or_else(|| "invalid common schema: missing properties".to_string())?;

    for (key, value) in payload {
        if let Some(spec) = properties.get(key) {
            validate_field_spec(key, value, spec)?;
        }
    }
    Ok(())
}

fn build_tool_search_payload(
    r: &xiuxian_vector::skill::ToolSearchResult,
    confidence: &str,
    final_score: f32,
) -> JsonMap<String, JsonValue> {
    let mut payload = JsonMap::new();
    payload.insert(
        "schema".to_string(),
        JsonValue::String("xiuxian.vector.tool_search.v1".to_string()),
    );
    payload.insert("name".to_string(), JsonValue::String(r.name.clone()));
    payload.insert(
        "description".to_string(),
        JsonValue::String(r.description.clone()),
    );
    payload.insert("input_schema".to_string(), r.input_schema.clone());
    payload.insert("score".to_string(), JsonValue::from(r.score));
    if let Some(v) = r.vector_score {
        payload.insert("vector_score".to_string(), JsonValue::from(v));
    }
    if let Some(v) = r.keyword_score {
        payload.insert("keyword_score".to_string(), JsonValue::from(v));
    }
    payload.insert("final_score".to_string(), JsonValue::from(final_score));
    payload.insert(
        "confidence".to_string(),
        JsonValue::String(confidence.to_string()),
    );
    payload.insert(
        "skill_name".to_string(),
        JsonValue::String(r.skill_name.clone()),
    );
    payload.insert(
        "tool_name".to_string(),
        JsonValue::String(r.tool_name.clone()),
    );
    payload.insert(
        "file_path".to_string(),
        JsonValue::String(r.file_path.clone()),
    );
    payload.insert(
        "routing_keywords".to_string(),
        JsonValue::Array(r.keywords.iter().cloned().map(JsonValue::String).collect()),
    );
    payload.insert(
        "intents".to_string(),
        JsonValue::Array(r.intents.iter().cloned().map(JsonValue::String).collect()),
    );
    payload.insert(
        "category".to_string(),
        JsonValue::String(r.category.clone()),
    );
    payload
}

#[derive(Debug, Deserialize, Default)]
struct PySearchOptions {
    where_filter: Option<String>,
    batch_size: Option<usize>,
    fragment_readahead: Option<usize>,
    batch_readahead: Option<usize>,
    scan_limit: Option<usize>,
}

pub(crate) fn search_optimized_async(
    path: &str,
    dimension: usize,
    enable_kw: bool,
    table_name: &str,
    query: Vec<f32>,
    limit: usize,
    options_json: Option<String>,
) -> PyResult<Vec<String>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    rt.block_on(async {
        let store = VectorStore::new_with_keyword_index(path, Some(dimension), enable_kw)
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let py_options = options_json
            .as_deref()
            .map(serde_json::from_str::<PySearchOptions>)
            .transpose()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?
            .unwrap_or_default();

        let options = SearchOptions {
            where_filter: py_options.where_filter,
            batch_size: py_options.batch_size,
            fragment_readahead: py_options.fragment_readahead,
            batch_readahead: py_options.batch_readahead,
            scan_limit: py_options.scan_limit,
            ..SearchOptions::default()
        };

        let results = store
            .search_optimized(table_name, query, limit, options)
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(results
            .into_iter()
            .map(|r| {
                let score = 1.0f64 / (1.0f64 + r.distance.max(0.0));
                serde_json::json!({
                    "schema": "xiuxian.vector.search.v1",
                    "id": r.id,
                    "content": r.content,
                    "metadata": r.metadata,
                    "distance": r.distance,
                    "score": score,
                })
                .to_string()
            })
            .collect())
    })
}

pub(crate) fn search_hybrid_async(
    path: &str,
    dimension: usize,
    enable_kw: bool,
    table_name: &str,
    query: Vec<f32>,
    query_text: String,
    limit: usize,
) -> PyResult<Vec<String>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    rt.block_on(async {
        let store = VectorStore::new_with_keyword_index(path, Some(dimension), enable_kw)
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let vector_rows = store
            .search_optimized(
                table_name,
                query.clone(),
                limit.saturating_mul(2).max(limit),
                SearchOptions::default(),
            )
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        let mut by_id: HashMap<String, (String, serde_json::Value)> = HashMap::new();
        for row in vector_rows {
            by_id.insert(row.id, (row.content, row.metadata));
        }

        let results = store
            .hybrid_search(table_name, &query_text, query, limit)
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        Ok(results
            .into_iter()
            .map(|r| {
                let (content, metadata) = by_id
                    .get(&r.tool_name)
                    .cloned()
                    .unwrap_or_else(|| (String::new(), serde_json::json!({})));
                serde_json::json!({
                    "schema": "xiuxian.vector.hybrid.v1",
                    "id": r.tool_name,
                    "content": content,
                    "metadata": metadata,
                    "source": "hybrid",
                    "score": r.rrf_score,
                    "vector_score": r.vector_score,
                    "keyword_score": r.keyword_score,
                })
                .to_string()
            })
            .collect())
    })
}

pub(crate) fn create_index_async(
    path: &str,
    dimension: usize,
    enable_kw: bool,
    table_name: &str,
) -> PyResult<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    rt.block_on(async {
        let store = VectorStore::new_with_keyword_index(path, Some(dimension), enable_kw)
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;
        store
            .create_index(table_name)
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))
    })
}

pub(crate) fn search_tools_async(
    path: &str,
    dimension: usize,
    enable_kw: bool,
    table_name: &str,
    query_vector: Vec<f32>,
    query_text: Option<String>,
    limit: usize,
    threshold: f32,
    confidence_profile_json: Option<String>,
    rerank: bool,
) -> PyResult<Vec<Py<PyAny>>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    rt.block_on(async {
        let store = VectorStore::new_with_keyword_index(path, Some(dimension), enable_kw)
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let results = store
            .search_tools_with_options(
                table_name,
                &query_vector,
                query_text.as_deref(),
                limit,
                threshold,
                ToolSearchOptions { rerank },
            )
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let confidence_profile = confidence_profile_json
            .as_deref()
            .map(serde_json::from_str::<ConfidenceProfile>)
            .transpose()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?
            .unwrap_or_default()
            .sanitize();

        let py_results = pyo3::Python::attach(|py| -> PyResult<Vec<Py<PyAny>>> {
            let mut dicts = Vec::with_capacity(results.len());
            for r in results {
                let (confidence, final_score) = calibrate_confidence(r.score, &confidence_profile);
                let payload = build_tool_search_payload(&r, confidence, final_score);
                validate_tool_search_common_schema(&payload).map_err(|e| {
                    pyo3::exceptions::PyValueError::new_err(format!(
                        "tool_search common schema validation failed: {e}"
                    ))
                })?;
                let dict = pyo3::types::PyDict::new(py);
                for (k, v) in &payload {
                    dict.set_item(k, json_value_to_py(py, v)?)?;
                }
                dicts.push(dict.into_pyobject(py)?.into());
            }
            Ok(dicts)
        });
        py_results
    })
}

pub(crate) fn load_tool_registry_async(
    path: &str,
    dimension: usize,
    enable_kw: bool,
    table_name: &str,
    confidence_profile_json: Option<String>,
) -> PyResult<Vec<Py<PyAny>>> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    rt.block_on(async {
        let store = VectorStore::new_with_keyword_index(path, Some(dimension), enable_kw)
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let results = store
            .load_tool_registry(table_name)
            .await
            .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

        let confidence_profile = confidence_profile_json
            .as_deref()
            .map(serde_json::from_str::<ConfidenceProfile>)
            .transpose()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?
            .unwrap_or_default()
            .sanitize();

        let py_results = pyo3::Python::attach(|py| -> PyResult<Vec<Py<PyAny>>> {
            let mut dicts = Vec::with_capacity(results.len());
            for r in results {
                let (confidence, final_score) = calibrate_confidence(r.score, &confidence_profile);
                let payload = build_tool_search_payload(&r, confidence, final_score);
                validate_tool_search_common_schema(&payload).map_err(|e| {
                    pyo3::exceptions::PyValueError::new_err(format!(
                        "tool_search common schema validation failed: {e}"
                    ))
                })?;
                let dict = pyo3::types::PyDict::new(py);
                for (k, v) in &payload {
                    dict.set_item(k, json_value_to_py(py, v)?)?;
                }
                dicts.push(dict.into_pyobject(py)?.into());
            }
            Ok(dicts)
        });
        py_results
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_schema_accepts_canonical_payload() {
        let payload = serde_json::json!({
            "schema": "xiuxian.vector.tool_search.v1",
            "name": "git.commit",
            "tool_name": "git.commit",
            "description": "Commit changes",
            "input_schema": {"type": "object"},
            "score": 0.9,
            "final_score": 0.95,
            "confidence": "high",
            "skill_name": "git",
            "file_path": "assets/skills/git/scripts/commit.py",
            "routing_keywords": ["git", "commit"],
            "intents": [],
            "category": "write"
        });
        let map = payload.as_object().expect("payload object");
        assert!(validate_tool_search_common_schema(map).is_ok());
    }

    #[test]
    fn common_schema_rejects_legacy_keywords_field() {
        let payload = serde_json::json!({
            "schema": "xiuxian.vector.tool_search.v1",
            "name": "git.commit",
            "tool_name": "git.commit",
            "keywords": ["git", "commit"]
        });
        let map = payload.as_object().expect("payload object");
        let err = validate_tool_search_common_schema(map).expect_err("should fail");
        assert!(err.contains("unknown field 'keywords'"));
    }
}

pub(crate) fn scan_skill_tools_raw(base_path: &str) -> PyResult<Vec<String>> {
    use omni_scanner::{SkillScanner, ToolRecord, ToolsScanner};

    let skill_scanner = SkillScanner::new();
    let script_scanner = ToolsScanner::new();
    let skills_path = Path::new(base_path);

    if !skills_path.exists() {
        return Ok(vec![]);
    }

    let metadatas = skill_scanner
        .scan_all(skills_path, None)
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    let mut all_tools: Vec<ToolRecord> = Vec::new();
    let empty_intents: &[String] = &[];

    for metadata in &metadatas {
        let skill_scripts_path = skills_path.join(&metadata.skill_name).join("scripts");

        match script_scanner.scan_scripts(
            &skill_scripts_path,
            &metadata.skill_name,
            &metadata.routing_keywords,
            empty_intents,
        ) {
            Ok(tools) => all_tools.extend(tools),
            Err(e) => eprintln!(
                "Warning: Failed to scan for '{}': {}",
                metadata.skill_name, e
            ),
        }
    }

    let json_tools: Vec<String> = all_tools
        .into_iter()
        .map(|t| serde_json::to_string(&t).unwrap_or_default())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(json_tools)
}
