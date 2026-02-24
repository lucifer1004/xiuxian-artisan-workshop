//! Schema Registry - Dynamic JSON Schema Generation (Schema Singularity)
//!
//! This module exposes Rust-driven schema generation to Python.
//! It uses `schemars` to auto-generate JSON Schemas from Rust structs,
//! establishing Rust as the Single Source of Truth (SSOT) for type definitions.

use omni_types::SchemaError;
use pyo3::prelude::*;

/// Get JSON Schema for a registered type.
///
/// This enables Python to dynamically retrieve authoritative schemas from Rust.
/// The schema is generated at runtime from the actual Rust struct definition,
/// ensuring Python and LLM consumers always see the latest type contract.
///
/// # Arguments
/// * `type_name` - Name of the type to get schema for (e.g., "SkillDefinition", "OmniTool")
///
/// # Returns
/// JSON string representing the JSON Schema for the type.
///
/// # Errors
/// Raises `ValueError` if the type name is unknown.
#[pyfunction]
#[pyo3(signature = (type_name))]
pub fn py_get_schema_json(type_name: &str) -> PyResult<String> {
    match omni_types::get_schema_json(type_name) {
        Ok(schema) => Ok(schema),
        Err(SchemaError::UnknownType(name)) => {
            Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Unknown type: {}. Available types: {:?}",
                name,
                omni_types::get_registered_types()
            )))
        }
    }
}

/// Get canonical JSON Schema by schema identifier.
///
/// This API is for runtime schema validation where callers use canonical IDs
/// such as `omni.agent.server_info.v1` or `omni.vector.tool_search.v1`.
///
/// # Errors
/// Raises `ValueError` if the schema identifier is unknown.
#[pyfunction]
#[pyo3(signature = (name))]
pub fn py_get_named_schema_json(name: &str) -> PyResult<String> {
    xiuxian_wendao::schemas::get_schema(name)
        .map(std::string::ToString::to_string)
        .ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!("Unknown schema identifier: {}", name))
        })
}

/// Get list of all registered type names.
///
/// # Returns
/// List of type names that can be passed to `py_get_schema_json`.
#[pyfunction]
pub fn py_get_registered_types() -> Vec<&'static str> {
    omni_types::get_registered_types()
}
