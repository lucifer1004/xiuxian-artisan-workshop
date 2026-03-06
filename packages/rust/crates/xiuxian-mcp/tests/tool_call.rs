//! Integration tests for MCP tool-call helpers.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Result, anyhow};
use rmcp::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, ErrorData, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio_util::sync::CancellationToken;
use xiuxian_mcp::{
    McpServerTransportConfig, McpToolCallExecution, OmniMcpClient, call_tool_with_timeout,
    init_params_omni_server,
};

#[derive(Clone, Default)]
struct MockToolServer;

impl MockToolServer {
    fn tool(name: &str, description: &str) -> Tool {
        let input_schema = serde_json::json!({
            "type": "object",
            "properties": { "message": { "type": "string" } }
        });
        let map = input_schema.as_object().cloned().unwrap_or_default();
        Tool {
            name: name.to_string().into(),
            title: None,
            description: Some(description.to_string().into()),
            input_schema: Arc::new(map),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        }
    }
}

impl ServerHandler for MockToolServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, ErrorData>> + Send + '_ {
        std::future::ready(Ok(ListToolsResult::with_all_items(vec![
            Self::tool("fast_echo", "Fast echo"),
            Self::tool("slow_echo", "Slow echo"),
        ])))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let name = request.name.to_string();
        let message = request
            .arguments
            .as_ref()
            .and_then(|map| map.get("message"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("ok")
            .to_string();

        if name == "slow_echo" {
            tokio::time::sleep(Duration::from_millis(1_500)).await;
        }

        Ok(CallToolResult::success(vec![Content::text(format!(
            "{name}:{message}"
        ))]))
    }
}

async fn spawn_mock_server() -> Result<(String, tokio::task::JoinHandle<()>, CancellationToken)> {
    let cancellation = CancellationToken::new();
    let service: StreamableHttpService<MockToolServer, LocalSessionManager> =
        StreamableHttpService::new(
            || Ok(MockToolServer),
            Arc::new(LocalSessionManager::default()),
            StreamableHttpServerConfig {
                stateful_mode: true,
                sse_keep_alive: None,
                cancellation_token: cancellation.child_token(),
                ..Default::default()
            },
        );
    let router = axum::Router::new().nest_service("/mcp", service);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server_task = tokio::spawn({
        let cancellation = cancellation.clone();
        async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    cancellation.cancelled_owned().await;
                })
                .await;
        }
    });
    Ok((format!("http://{address}/mcp"), server_task, cancellation))
}

#[tokio::test]
async fn call_tool_with_timeout_returns_completed_for_fast_tool() -> Result<()> {
    let (url, server_task, cancellation) = spawn_mock_server().await?;
    let client = OmniMcpClient::connect_streamable_http(
        &url,
        init_params_omni_server(),
        Some(Duration::from_secs(10)),
    )
    .await?;

    let outcome = call_tool_with_timeout(
        &client,
        "fast_echo",
        Some(serde_json::json!({ "message": "ok" })),
        1,
    )
    .await;

    let payload = match outcome {
        McpToolCallExecution::Completed(payload) => payload,
        McpToolCallExecution::Timeout { detail } => {
            return Err(anyhow!("expected completed, got timeout: {detail:?}"));
        }
        McpToolCallExecution::TransportError(error) => {
            return Err(anyhow!("expected completed, got transport error: {error}"));
        }
    };

    assert_eq!(payload.text, "fast_echo:ok");
    assert!(!payload.is_error);

    cancellation.cancel();
    server_task
        .await
        .map_err(|error| anyhow!("mock server join failed: {error}"))?;
    Ok(())
}

#[tokio::test]
async fn call_tool_with_timeout_returns_timeout_for_slow_tool() -> Result<()> {
    let (url, server_task, cancellation) = spawn_mock_server().await?;
    let client = OmniMcpClient::connect_streamable_http(
        &url,
        init_params_omni_server(),
        Some(Duration::from_secs(10)),
    )
    .await?;

    let outcome = call_tool_with_timeout(
        &client,
        "slow_echo",
        Some(serde_json::json!({ "message": "late" })),
        1,
    )
    .await;

    assert!(
        matches!(outcome, McpToolCallExecution::Timeout { .. }),
        "expected timeout outcome"
    );

    cancellation.cancel();
    server_task
        .await
        .map_err(|error| anyhow!("mock server join failed: {error}"))?;
    Ok(())
}

#[tokio::test]
async fn call_tool_with_timeout_returns_transport_error_when_uninitialized() -> Result<()> {
    let config = McpServerTransportConfig::StreamableHttp {
        url: "http://127.0.0.1:65535".to_string(),
        bearer_token_env_var: None,
    };
    let client = OmniMcpClient::from_config(&config);
    let outcome = call_tool_with_timeout(
        &client,
        "demo.echo",
        Some(serde_json::json!({ "message": "x" })),
        1,
    )
    .await;
    match outcome {
        McpToolCallExecution::TransportError(error) => {
            let message = error.to_string();
            assert!(
                message.contains("not initialized"),
                "expected not-initialized error, got: {message}"
            );
        }
        McpToolCallExecution::Completed(payload) => {
            return Err(anyhow!(
                "expected transport error, got completed: {}",
                payload.text
            ));
        }
        McpToolCallExecution::Timeout { detail } => {
            return Err(anyhow!("expected transport error, got timeout: {detail:?}"));
        }
    }
    Ok(())
}
