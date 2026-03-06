//! Integration tests for MCP client notification handling.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use anyhow::{Result, anyhow};
use axum::Router;
use rmcp::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, ErrorData, ListToolsResult, LoggingLevel,
    LoggingMessageNotificationParam, PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::{RequestContext, RoleServer};
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};
use tokio_util::sync::CancellationToken;
use xiuxian_mcp::{OmniMcpClient, init_params_omni_server};

#[derive(Clone)]
struct NotifyingMockServer {
    list_tools_calls: Arc<AtomicUsize>,
}

impl NotifyingMockServer {
    fn new(list_tools_calls: Arc<AtomicUsize>) -> Self {
        Self { list_tools_calls }
    }

    fn mock_tool() -> Tool {
        Tool {
            name: "mock.echo".into(),
            title: Some("Mock Echo".into()),
            description: Some("Echo tool for notification regression tests".into()),
            input_schema: Arc::new(serde_json::Map::new()),
            output_schema: None,
            annotations: None,
            execution: None,
            icons: None,
            meta: None,
        }
    }
}

impl ServerHandler for NotifyingMockServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_tool_list_changed()
                .enable_logging()
                .build(),
            ..Default::default()
        }
    }

    fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<ListToolsResult, ErrorData>> + Send + '_ {
        let list_tools_calls = Arc::clone(&self.list_tools_calls);
        async move {
            list_tools_calls.fetch_add(1, Ordering::SeqCst);
            let _ = context
                .peer
                .notify_logging_message(LoggingMessageNotificationParam {
                    level: LoggingLevel::Info,
                    logger: Some("mock_server".to_string()),
                    data: serde_json::json!({
                        "message": "tool list refreshed",
                        "calls": list_tools_calls.load(Ordering::SeqCst),
                    }),
                })
                .await;
            Ok(ListToolsResult::with_all_items(vec![Self::mock_tool()]))
        }
    }

    fn call_tool(
        &self,
        _request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> impl std::future::Future<Output = Result<CallToolResult, ErrorData>> + Send + '_ {
        async move {
            let _ = context.peer.notify_tool_list_changed().await;
            Ok(CallToolResult::success(vec![Content::text("ok")]))
        }
    }
}

async fn spawn_mock_server() -> Result<(String, CancellationToken, tokio::task::JoinHandle<()>)> {
    let cancellation = CancellationToken::new();
    let list_tools_calls = Arc::new(AtomicUsize::new(0));
    let service: StreamableHttpService<NotifyingMockServer, LocalSessionManager> =
        StreamableHttpService::new(
            {
                let list_tools_calls = Arc::clone(&list_tools_calls);
                move || Ok(NotifyingMockServer::new(Arc::clone(&list_tools_calls)))
            },
            Arc::new(LocalSessionManager::default()),
            StreamableHttpServerConfig {
                stateful_mode: true,
                sse_keep_alive: None,
                cancellation_token: cancellation.child_token(),
                ..Default::default()
            },
        );
    let router = Router::new().nest_service("/mcp", service);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let handle = tokio::spawn({
        let cancellation = cancellation.clone();
        async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async move { cancellation.cancelled_owned().await })
                .await;
        }
    });
    Ok((format!("http://{addr}/mcp"), cancellation, handle))
}

async fn wait_for_notification<F>(mut predicate: F) -> Result<()>
where
    F: FnMut() -> bool,
{
    let deadline = tokio::time::Instant::now() + Duration::from_secs(2);
    while tokio::time::Instant::now() < deadline {
        if predicate() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Err(anyhow!("timed out waiting for MCP notification"))
}

#[tokio::test]
async fn client_observes_logging_and_tool_list_change_notifications() -> Result<()> {
    let (url, cancellation, handle) = spawn_mock_server().await?;
    let client = OmniMcpClient::connect_streamable_http(
        &url,
        init_params_omni_server(),
        Some(Duration::from_secs(5)),
    )
    .await?;

    let listed = client.list_tools(None).await?;
    assert_eq!(listed.tools.len(), 1);
    wait_for_notification(|| client.notification_stats_snapshot().logging_message_count >= 1)
        .await?;

    let called = client
        .call_tool(
            "mock.echo".to_string(),
            Some(serde_json::json!({"message": "hi"})),
        )
        .await?;
    assert_eq!(called.content.len(), 1);
    wait_for_notification(|| client.tool_list_changed_epoch() > 0).await?;

    let stats = client.notification_stats_snapshot();
    assert_eq!(stats.logging_message_count, 1);
    assert_eq!(stats.tool_list_changed_count, 1);
    assert!(stats.tool_list_changed_epoch > 0);

    cancellation.cancel();
    handle
        .await
        .map_err(|error| anyhow!("mock server join failed: {error}"))?;
    Ok(())
}
