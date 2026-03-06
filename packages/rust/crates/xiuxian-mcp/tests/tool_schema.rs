//! Integration tests for MCP-to-LLM tool schema conversion.

use std::sync::Arc;

use rmcp::model::{ListToolsResult, Tool};
use xiuxian_mcp::{llm_tool_definition, llm_tool_definitions};

fn mock_tool(name: &str, description: Option<&str>) -> Tool {
    let input_schema = serde_json::json!({
        "type": "object",
        "properties": {
            "query": { "type": "string" }
        },
        "required": ["query"]
    });
    let map = input_schema.as_object().cloned().unwrap_or_default();
    Tool {
        name: name.to_string().into(),
        title: None,
        description: description.map(|value| value.to_string().into()),
        input_schema: Arc::new(map),
        output_schema: None,
        annotations: None,
        execution: None,
        icons: None,
        meta: None,
    }
}

#[test]
fn llm_tool_definition_maps_required_fields() {
    let tool = mock_tool("skill.search", Some("Search skills"));
    let value = llm_tool_definition(&tool);
    assert_eq!(
        value["name"],
        serde_json::Value::String("skill.search".into())
    );
    assert_eq!(
        value["description"],
        serde_json::Value::String("Search skills".into())
    );
    assert_eq!(
        value["parameters"]["type"],
        serde_json::Value::String("object".into())
    );
}

#[test]
fn llm_tool_definitions_maps_full_list() {
    let list = ListToolsResult::with_all_items(vec![
        mock_tool("skill.search", Some("Search")),
        mock_tool("skill.run", None),
    ]);
    let values = llm_tool_definitions(&list);
    assert_eq!(values.len(), 2);
    assert_eq!(
        values[0]["name"],
        serde_json::Value::String("skill.search".into())
    );
    assert_eq!(
        values[1]["name"],
        serde_json::Value::String("skill.run".into())
    );
    assert!(values[1].get("description").is_none());
}
