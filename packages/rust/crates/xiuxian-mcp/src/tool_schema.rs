//! MCP-to-LLM tool schema conversion helpers.

use rmcp::model::{ListToolsResult, Tool};

/// Convert one MCP `Tool` to the LLM tool schema object used by callers.
#[must_use]
pub fn llm_tool_definition(tool: &Tool) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    object.insert(
        "name".to_string(),
        serde_json::Value::String(tool.name.to_string()),
    );
    if let Some(description) = &tool.description {
        object.insert(
            "description".to_string(),
            serde_json::Value::String(description.to_string()),
        );
    }
    object.insert(
        "parameters".to_string(),
        serde_json::Value::Object(tool.input_schema.as_ref().clone()),
    );
    serde_json::Value::Object(object)
}

/// Convert MCP `tools/list` response to LLM tool schema list.
#[must_use]
pub fn llm_tool_definitions(list: &ListToolsResult) -> Vec<serde_json::Value> {
    list.tools.iter().map(llm_tool_definition).collect()
}
