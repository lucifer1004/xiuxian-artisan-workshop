use anyhow::Result;

use super::Agent;

const MEMORY_SEARCH_TOOL_NAME: &str = "memory.search_memory";
const MEMORY_SAVE_TOOL_NAME: &str = "memory.save_memory";

pub(super) struct ToolCallOutput {
    pub(super) text: String,
    pub(super) is_error: bool,
}

impl Agent {
    #[allow(clippy::unused_self)]
    pub(super) fn soft_fail_mcp_tool_error_output(
        &self,
        name: &str,
        error: &anyhow::Error,
    ) -> Option<ToolCallOutput> {
        soft_fail_mcp_tool_error_output(name, error)
    }

    pub(super) async fn mcp_tools_for_llm(&self) -> Result<Option<Vec<serde_json::Value>>> {
        let Some(ref mcp) = self.mcp else {
            return Ok(None);
        };
        let list = mcp.list_tools(None).await?;
        let tools: Vec<serde_json::Value> = list
            .tools
            .iter()
            .map(|t| {
                let mut obj = serde_json::Map::new();
                obj.insert(
                    "name".to_string(),
                    serde_json::Value::String(t.name.to_string()),
                );
                if let Some(ref d) = t.description {
                    obj.insert(
                        "description".to_string(),
                        serde_json::Value::String(d.to_string()),
                    );
                }
                let schema = serde_json::Value::Object(t.input_schema.as_ref().clone());
                obj.insert("parameters".to_string(), schema);
                serde_json::Value::Object(obj)
            })
            .collect();
        if tools.is_empty() {
            return Ok(None);
        }
        Ok(Some(tools))
    }

    pub(super) async fn call_mcp_tool_with_diagnostics(
        &self,
        name: &str,
        arguments: Option<serde_json::Value>,
    ) -> Result<ToolCallOutput> {
        let Some(ref mcp) = self.mcp else {
            return Err(anyhow::anyhow!("no MCP client"));
        };
        let result = mcp.call_tool(name.to_string(), arguments).await?;
        let text: String = result
            .content
            .iter()
            .filter_map(|c| {
                if let rmcp::model::RawContent::Text(t) = &c.raw {
                    Some(t.text.as_str())
                } else {
                    None
                }
            })
            .collect();
        Ok(ToolCallOutput {
            text,
            is_error: result.is_error.unwrap_or(false),
        })
    }
}

fn soft_fail_mcp_tool_error_output(name: &str, error: &anyhow::Error) -> Option<ToolCallOutput> {
    let message = format!("{error:#}");
    let lower = message.to_ascii_lowercase();
    if name == MEMORY_SAVE_TOOL_NAME {
        let error_kind = if is_timeout_error_message(&lower) {
            "timeout"
        } else {
            "save_failed"
        };
        tracing::warn!(
            event = "agent.mcp.tool.soft_fail",
            tool = name,
            error_kind,
            error = %message,
            "mcp tool failed while saving memory; degrading to soft tool error output"
        );
        return Some(ToolCallOutput {
            text: serde_json::json!({
                "ok": false,
                "degraded": true,
                "tool": name,
                "error_kind": error_kind,
                "message": "Memory save failed; continuing without blocking this turn.",
            })
            .to_string(),
            is_error: true,
        });
    }

    let is_embedding_timeout = lower.contains("embedding timed out")
        || (lower.contains("embedding")
            && lower.contains("timed out")
            && lower.contains("mcp error: -32603"));
    if name != MEMORY_SEARCH_TOOL_NAME || !is_embedding_timeout {
        return None;
    }
    tracing::warn!(
        event = "agent.mcp.tool.soft_fail",
        tool = name,
        error = %message,
        "mcp tool failed with embedding timeout; degrading to soft tool error output"
    );
    Some(ToolCallOutput {
        text: serde_json::json!({
            "ok": false,
            "degraded": true,
            "tool": name,
            "error_kind": "embedding_timeout",
            "message": "Embedding lookup timed out; continuing without tool result.",
        })
        .to_string(),
        is_error: true,
    })
}

fn is_timeout_error_message(lowercase_error: &str) -> bool {
    lowercase_error.contains("timed out")
        || lowercase_error.contains("timeout")
        || lowercase_error.contains("mcp.pool.call.waiting")
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use anyhow::anyhow;

    use super::{MEMORY_SAVE_TOOL_NAME, MEMORY_SEARCH_TOOL_NAME, soft_fail_mcp_tool_error_output};

    #[test]
    fn soft_fail_output_is_enabled_for_memory_search_embedding_timeout() {
        let error = anyhow!(
            "MCP tools/call failed after reconnect retry: Mcp error: -32603: Embedding timed out after 5s"
        );
        let output = soft_fail_mcp_tool_error_output(MEMORY_SEARCH_TOOL_NAME, &error)
            .expect("embedding timeout should degrade for memory.search_memory");
        assert!(output.is_error);
        assert!(output.text.contains("\"degraded\":true"));
        assert!(output.text.contains(MEMORY_SEARCH_TOOL_NAME));
    }

    #[test]
    fn soft_fail_output_is_not_enabled_for_other_tools() {
        let error = anyhow!("Mcp error: -32603: Embedding timed out after 5s");
        assert!(
            soft_fail_mcp_tool_error_output("skill.discover", &error).is_none(),
            "non-memory-search tools should keep hard-fail semantics"
        );
    }

    #[test]
    fn soft_fail_output_is_enabled_for_memory_save_timeout() {
        let error = anyhow!(
            "MCP tools/call timed out after 5s (client_index=1, tool=tools/call:memory.save_memory)"
        );
        let output = soft_fail_mcp_tool_error_output(MEMORY_SAVE_TOOL_NAME, &error)
            .expect("memory.save_memory timeout should degrade");
        assert!(output.is_error);
        assert!(output.text.contains("\"degraded\":true"));
        assert!(output.text.contains(MEMORY_SAVE_TOOL_NAME));
        assert!(output.text.contains("\"error_kind\":\"timeout\""));
    }

    #[test]
    fn soft_fail_output_is_enabled_for_memory_save_non_timeout_error() {
        let error = anyhow!("Mcp error: -32603: write failed");
        let output = soft_fail_mcp_tool_error_output(MEMORY_SAVE_TOOL_NAME, &error)
            .expect("memory.save_memory failure should degrade");
        assert!(output.is_error);
        assert!(output.text.contains("\"error_kind\":\"save_failed\""));
    }
}
