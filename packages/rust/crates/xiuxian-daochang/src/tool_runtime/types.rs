use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolRuntimeListResult {
    pub tools: Vec<ToolRuntimeToolDefinition>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolRuntimeToolDefinition {
    pub name: String,
    pub description: Option<String>,
    pub input_schema: serde_json::Map<String, serde_json::Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolRuntimeCallResult {
    pub text_segments: Vec<String>,
    pub is_error: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ToolRuntimeListRequestParams {
    pub cursor: Option<String>,
}
