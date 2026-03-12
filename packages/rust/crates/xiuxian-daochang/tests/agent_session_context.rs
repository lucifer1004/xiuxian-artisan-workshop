//! Integration tests for server-originated MCP notifications.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Result, anyhow};
use rmcp::ErrorData as McpError;
use rmcp::ServerHandler;
use rmcp::model::{
    ListToolsResult, LoggingLevel, LoggingMessageNotificationParam, PaginatedRequestParams,
    ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use tokio_util::sync::CancellationToken;
use xiuxian_mcp::{
    McpHttpServerConfig, McpServerBackend, McpServerHandle, OmniMcpClient,
    build_streamable_http_router, init_params_omni_server,
};

#[derive(Clone, Default)]
struct BroadcastBackend {
    server_handle: Arc<Mutex<Option<McpServerHandle>>>,
}

impl BroadcastBackend {
    fn handle(&self) -> Result<McpServerHandle> {
        self.server_handle
            .lock()
            .expect("server handle mutex poisoned")
            .clone()
            .ok_or_else(|| anyhow!("expected MCP server handle to be bound"))
    }
}

impl McpServerBackend for BroadcastBackend {
    fn bind_server_handle(&self, server_handle: McpServerHandle) {
        let mut slot = self
            .server_handle
            .lock()
            .expect("server handle mutex poisoned");
        *slot = Some(server_handle);
    }
}

impl ServerHandler for BroadcastBackend {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_logging()
                .enable_tools()
                .enable_tool_list_changed()
                .enable_prompts()
                .enable_prompts_list_changed()
                .enable_resources()
                .enable_resources_list_changed()
                .build(),
        )
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(vec![Tool::new(
            "mock.echo",
            "Echo tool for server-notification tests",
            Arc::new(serde_json::Map::new()),
        )]))
    }
}

async fn wait_for_notification<F>(mut predicate: F) -> Result<()>
where
    F: FnMut() -> bool,
{
    let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
    while tokio::time::Instant::now() < deadline {
        if predicate() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Err(anyhow!("timed out waiting for MCP notification"))
}

#[tokio::test]
async fn streamable_http_server_broadcasts_list_changed_and_logging_notifications() -> Result<()> {
    let backend = BroadcastBackend::default();
    let cancellation_token = CancellationToken::new();
    let app = build_streamable_http_router(
        backend.clone(),
        McpHttpServerConfig::default(),
        &cancellation_token,
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, app)
            .with_graceful_shutdown(async move { cancellation_token.cancelled().await })
            .await;
    });

    let client = OmniMcpClient::connect_streamable_http(
        &format!("http://{addr}/mcp"),
        init_params_omni_server(),
        Some(Duration::from_secs(5)),
    )
    .await?;
    let handle = backend.handle()?;

    let _ = handle
        .broadcast_logging_message(LoggingMessageNotificationParam {
            level: LoggingLevel::Info,
            logger: Some("test.server".to_string()),
            data: serde_json::json!({ "message": "hello" }),
        })
        .await?;
    let _ = handle.broadcast_prompt_list_changed().await?;
    let _ = handle.broadcast_resource_list_changed().await?;
    handle.debounce_tool_list_changed().await;

    wait_for_notification(|| {
        let stats = client.notification_stats_snapshot();
        stats.logging_message_count >= 1
            && stats.prompt_list_changed_count >= 1
            && stats.resource_list_changed_count >= 1
            && stats.tool_list_changed_count >= 1
    })
    .await?;

    let first_stats = client.notification_stats_snapshot();
    assert_eq!(first_stats.logging_message_count, 1);
    assert_eq!(first_stats.prompt_list_changed_count, 1);
    assert_eq!(first_stats.resource_list_changed_count, 1);
    assert_eq!(first_stats.tool_list_changed_count, 1);

    handle.debounce_tool_list_changed().await;
    handle.debounce_tool_list_changed().await;
    tokio::time::sleep(handle.debounce_window() + Duration::from_millis(200)).await;

    let final_stats = client.notification_stats_snapshot();
    assert_eq!(final_stats.tool_list_changed_count, 2);
    assert_eq!(final_stats.prompt_list_changed_count, 1);
    assert_eq!(final_stats.resource_list_changed_count, 1);

    server.abort();
    let _ = server.await;
    Ok(())
}
