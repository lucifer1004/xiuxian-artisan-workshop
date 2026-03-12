//! Graph bridge module for connecting agent to graph operations.

use anyhow::Result;

/// Request for graph bridge operations.
pub struct GraphBridgeRequest {
    pub query: String,
    pub context: Option<String>,
}

/// Result from graph bridge operations.
pub struct GraphBridgeResult {
    pub response: String,
    pub metadata: Option<serde_json::Value>,
}

/// Validate a graph bridge request.
pub fn validate_graph_bridge_request(request: &GraphBridgeRequest) -> Result<()> {
    if request.query.trim().is_empty() {
        anyhow::bail!("Query cannot be empty");
    }
    Ok(())
}
