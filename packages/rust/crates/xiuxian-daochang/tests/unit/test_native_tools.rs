//! Native-tool registry smoke tests.

use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use xiuxian_daochang::{NativeTool, NativeToolCallContext, NativeToolRegistry};

struct MockTool;

#[async_trait]
impl NativeTool for MockTool {
    fn name(&self) -> &str {
        "mock.test"
    }
    fn description(&self) -> &str {
        "Mock tool for testing"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({})
    }
    async fn call(
        &self,
        _args: Option<serde_json::Value>,
        _context: &NativeToolCallContext,
    ) -> anyhow::Result<String> {
        Ok("Mock success".to_string())
    }
}

#[tokio::test]
async fn test_native_tool_registration_and_dispatch() {
    let mut registry = NativeToolRegistry::new();
    registry.register(Arc::new(MockTool));

    let tool = registry
        .get("mock.test")
        .expect("Tool should be registered");
    assert_eq!(tool.name(), "mock.test");

    let result = tool
        .call(None, &NativeToolCallContext::default())
        .await
        .expect("Call should succeed");
    assert_eq!(result, "Mock success");
}

#[test]
fn test_registry_summary_injection() {
    let mut registry = NativeToolRegistry::new();
    registry.register(Arc::new(MockTool));

    let summary = registry.get_registry_summary();
    assert!(
        summary.contains("mock.test"),
        "Summary should contain tool name"
    );
    assert!(
        summary.contains("Native Core Tools"),
        "Summary should have standard prefix"
    );
}
