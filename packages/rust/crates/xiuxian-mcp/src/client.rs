//! MCP client: full protocol handshake and tool calls.
//!
//! **Protocol (MCP spec, same as codex-rs):**
//! 1. Build transport (Streamable HTTP or stdio via `rmcp`).
//! 2. Run the handshake with a client handler:
//!    - Client sends `initialize` request (JSON-RPC) with protocolVersion, capabilities, clientInfo.
//!    - Server responds with `InitializeResult`.
//!    - Client sends `notifications/initialized`.
//! 3. After handshake, use `list_tools` and `call_tool` on the running service.
//!
//! The client handler also receives server-side notifications such as
//! `notifications/message` and `notifications/tools/list_changed` so upper layers can react.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use rmcp::ClientHandler;
use rmcp::model::{
    CallToolRequestParams, CancelledNotificationParam, ClientCapabilities, ClientInfo,
    InitializeRequestParams, LoggingLevel, LoggingMessageNotificationParam, PaginatedRequestParams,
    ProgressNotificationParam, ProtocolVersion, ServerInfo, SetLevelRequestParams,
};
use rmcp::service::{NotificationContext, RoleClient, RunningService, ServiceExt};
use rmcp::transport::IntoTransport;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::child_process::TokioChildProcess;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde::Serialize;
use tokio::process::Command;
use tokio::sync::Mutex;

use crate::config::McpServerTransportConfig;

static NEXT_TOOL_LIST_CHANGED_EPOCH: AtomicU64 = AtomicU64::new(1);

type RunningClientService = RunningService<RoleClient, ObservedClientHandler>;

/// Notification counters observed on one live MCP client connection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct McpClientNotificationStats {
    /// Number of server `notifications/tools/list_changed` messages received.
    pub tool_list_changed_count: u64,
    /// Monotonic process-wide epoch of the most recent tool-list-change signal.
    pub tool_list_changed_epoch: u64,
    /// Number of server `notifications/message` messages received.
    pub logging_message_count: u64,
    /// Number of server `notifications/resources/list_changed` messages received.
    pub resource_list_changed_count: u64,
    /// Number of server `notifications/prompts/list_changed` messages received.
    pub prompt_list_changed_count: u64,
    /// Number of server `notifications/progress` messages received.
    pub progress_notification_count: u64,
    /// Number of server `notifications/cancelled` messages received.
    pub cancelled_notification_count: u64,
}

/// Build init params for the omni Python MCP server (protocol 2025-06-18).
/// Use this when connecting to `omni mcp --transport sse` so protocol version matches server support.
#[must_use]
pub fn init_params_omni_server() -> InitializeRequestParams {
    InitializeRequestParams {
        meta: None,
        protocol_version: ProtocolVersion::V_2025_06_18,
        capabilities: ClientCapabilities::default(),
        client_info: rmcp::model::Implementation::from_build_env(),
    }
}

#[derive(Debug, Clone)]
struct ClientConnectionContext {
    transport: &'static str,
    target: String,
}

impl ClientConnectionContext {
    fn streamable_http(url: &str) -> Self {
        Self {
            transport: "streamable_http",
            target: url.to_string(),
        }
    }

    fn stdio(command: &str, args: &[String]) -> Self {
        let rendered_args = if args.is_empty() {
            command.to_string()
        } else {
            format!("{} {}", command, args.join(" "))
        };
        Self {
            transport: "stdio",
            target: rendered_args,
        }
    }
}

#[derive(Debug, Default)]
struct McpClientDiagnostics {
    tool_list_changed_count: AtomicU64,
    tool_list_changed_epoch: AtomicU64,
    logging_message_count: AtomicU64,
    resource_list_changed_count: AtomicU64,
    prompt_list_changed_count: AtomicU64,
    progress_notification_count: AtomicU64,
    cancelled_notification_count: AtomicU64,
}

impl McpClientDiagnostics {
    fn snapshot(&self) -> McpClientNotificationStats {
        McpClientNotificationStats {
            tool_list_changed_count: self.tool_list_changed_count.load(Ordering::Relaxed),
            tool_list_changed_epoch: self.tool_list_changed_epoch.load(Ordering::Relaxed),
            logging_message_count: self.logging_message_count.load(Ordering::Relaxed),
            resource_list_changed_count: self.resource_list_changed_count.load(Ordering::Relaxed),
            prompt_list_changed_count: self.prompt_list_changed_count.load(Ordering::Relaxed),
            progress_notification_count: self.progress_notification_count.load(Ordering::Relaxed),
            cancelled_notification_count: self.cancelled_notification_count.load(Ordering::Relaxed),
        }
    }
}

#[derive(Clone)]
struct ObservedClientHandler {
    info: ClientInfo,
    connection: ClientConnectionContext,
    diagnostics: Arc<McpClientDiagnostics>,
}

impl ObservedClientHandler {
    fn new(
        info: InitializeRequestParams,
        connection: ClientConnectionContext,
        diagnostics: Arc<McpClientDiagnostics>,
    ) -> Self {
        Self {
            info,
            connection,
            diagnostics,
        }
    }
}

impl ClientHandler for ObservedClientHandler {
    fn on_cancelled(
        &self,
        params: CancelledNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        let connection = self.connection.clone();
        let diagnostics = Arc::clone(&self.diagnostics);
        async move {
            let count = diagnostics
                .cancelled_notification_count
                .fetch_add(1, Ordering::Relaxed)
                .saturating_add(1);
            tracing::debug!(
                event = "mcp.client.server_notification.cancelled",
                transport = connection.transport,
                target = %connection.target,
                notification_count = count,
                request_id = %params.request_id,
                reason = params.reason.as_deref().unwrap_or(""),
                "received MCP cancellation notification"
            );
        }
    }

    fn on_progress(
        &self,
        params: ProgressNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        let connection = self.connection.clone();
        let diagnostics = Arc::clone(&self.diagnostics);
        async move {
            let count = diagnostics
                .progress_notification_count
                .fetch_add(1, Ordering::Relaxed)
                .saturating_add(1);
            tracing::debug!(
                event = "mcp.client.server_notification.progress",
                transport = connection.transport,
                target = %connection.target,
                notification_count = count,
                progress = params.progress,
                total = params.total,
                progress_token = ?params.progress_token,
                message = params.message.as_deref().unwrap_or(""),
                "received MCP progress notification"
            );
        }
    }

    fn on_logging_message(
        &self,
        params: LoggingMessageNotificationParam,
        _context: NotificationContext<RoleClient>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        let connection = self.connection.clone();
        let diagnostics = Arc::clone(&self.diagnostics);
        async move {
            let count = diagnostics
                .logging_message_count
                .fetch_add(1, Ordering::Relaxed)
                .saturating_add(1);
            match params.level {
                LoggingLevel::Debug => tracing::debug!(
                    event = "mcp.client.server_notification.log",
                    transport = connection.transport,
                    target = %connection.target,
                    notification_count = count,
                    level = logging_level_name(params.level),
                    logger = params.logger.as_deref().unwrap_or(""),
                    data = %params.data,
                    "received MCP server log message"
                ),
                LoggingLevel::Info | LoggingLevel::Notice => tracing::info!(
                    event = "mcp.client.server_notification.log",
                    transport = connection.transport,
                    target = %connection.target,
                    notification_count = count,
                    level = logging_level_name(params.level),
                    logger = params.logger.as_deref().unwrap_or(""),
                    data = %params.data,
                    "received MCP server log message"
                ),
                LoggingLevel::Warning => tracing::warn!(
                    event = "mcp.client.server_notification.log",
                    transport = connection.transport,
                    target = %connection.target,
                    notification_count = count,
                    level = logging_level_name(params.level),
                    logger = params.logger.as_deref().unwrap_or(""),
                    data = %params.data,
                    "received MCP server log message"
                ),
                LoggingLevel::Error
                | LoggingLevel::Critical
                | LoggingLevel::Alert
                | LoggingLevel::Emergency => tracing::error!(
                    event = "mcp.client.server_notification.log",
                    transport = connection.transport,
                    target = %connection.target,
                    notification_count = count,
                    level = logging_level_name(params.level),
                    logger = params.logger.as_deref().unwrap_or(""),
                    data = %params.data,
                    "received MCP server log message"
                ),
            }
        }
    }

    fn on_resource_list_changed(
        &self,
        _context: NotificationContext<RoleClient>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        let connection = self.connection.clone();
        let diagnostics = Arc::clone(&self.diagnostics);
        async move {
            let count = diagnostics
                .resource_list_changed_count
                .fetch_add(1, Ordering::Relaxed)
                .saturating_add(1);
            tracing::debug!(
                event = "mcp.client.server_notification.resource_list_changed",
                transport = connection.transport,
                target = %connection.target,
                notification_count = count,
                "received MCP resource-list-changed notification"
            );
        }
    }

    fn on_tool_list_changed(
        &self,
        _context: NotificationContext<RoleClient>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        let connection = self.connection.clone();
        let diagnostics = Arc::clone(&self.diagnostics);
        async move {
            let count = diagnostics
                .tool_list_changed_count
                .fetch_add(1, Ordering::Relaxed)
                .saturating_add(1);
            let epoch = NEXT_TOOL_LIST_CHANGED_EPOCH.fetch_add(1, Ordering::Relaxed);
            diagnostics
                .tool_list_changed_epoch
                .store(epoch, Ordering::Relaxed);
            tracing::info!(
                event = "mcp.client.server_notification.tool_list_changed",
                transport = connection.transport,
                target = %connection.target,
                notification_count = count,
                epoch,
                "received MCP tool-list-changed notification"
            );
        }
    }

    fn on_prompt_list_changed(
        &self,
        _context: NotificationContext<RoleClient>,
    ) -> impl std::future::Future<Output = ()> + Send + '_ {
        let connection = self.connection.clone();
        let diagnostics = Arc::clone(&self.diagnostics);
        async move {
            let count = diagnostics
                .prompt_list_changed_count
                .fetch_add(1, Ordering::Relaxed)
                .saturating_add(1);
            tracing::debug!(
                event = "mcp.client.server_notification.prompt_list_changed",
                transport = connection.transport,
                target = %connection.target,
                notification_count = count,
                "received MCP prompt-list-changed notification"
            );
        }
    }

    fn get_info(&self) -> ClientInfo {
        self.info.clone()
    }
}

/// State after connect: either still connecting or ready with running service.
enum ClientState {
    Connecting,
    Ready { service: Arc<RunningClientService> },
}

/// MCP client: one server. Initialize once, then `list_tools` / `call_tool`.
pub struct OmniMcpClient {
    state: Mutex<ClientState>,
    connection: ClientConnectionContext,
    diagnostics: Arc<McpClientDiagnostics>,
}

impl OmniMcpClient {
    /// Create client from transport config. Call `initialize` before `list_tools`/`call_tool`.
    #[must_use]
    pub fn from_config(transport: &McpServerTransportConfig) -> Self {
        let connection = match transport {
            McpServerTransportConfig::StreamableHttp { url, .. } => {
                ClientConnectionContext::streamable_http(url)
            }
            McpServerTransportConfig::Stdio { command, args } => {
                ClientConnectionContext::stdio(command, args)
            }
        };
        Self::connecting(connection)
    }

    fn connecting(connection: ClientConnectionContext) -> Self {
        Self {
            state: Mutex::new(ClientState::Connecting),
            connection,
            diagnostics: Arc::new(McpClientDiagnostics::default()),
        }
    }

    fn ready(
        service: RunningClientService,
        connection: ClientConnectionContext,
        diagnostics: Arc<McpClientDiagnostics>,
    ) -> Self {
        Self {
            state: Mutex::new(ClientState::Ready {
                service: Arc::new(service),
            }),
            connection,
            diagnostics,
        }
    }

    /// Connect via Streamable HTTP (e.g. `http://127.0.0.1:3000` for our Python MCP SSE).
    ///
    /// # Errors
    /// Returns an error if the HTTP client cannot be built, the MCP handshake times out,
    /// or the server rejects initialization.
    pub async fn connect_streamable_http(
        url: &str,
        init_params: InitializeRequestParams,
        timeout: Option<Duration>,
    ) -> Result<Self> {
        let connection = ClientConnectionContext::streamable_http(url);
        let http_config = StreamableHttpClientTransportConfig::with_uri(url.to_string());
        let http_client = reqwest::Client::builder()
            .build()
            .map_err(|error| anyhow!("reqwest client: {error}"))?;
        let transport = StreamableHttpClientTransport::with_client(http_client, http_config);
        let diagnostics = Arc::new(McpClientDiagnostics::default());
        let handler =
            ObservedClientHandler::new(init_params, connection.clone(), Arc::clone(&diagnostics));
        let service = connect_running_service(handler, transport, timeout, &connection).await?;
        maybe_enable_server_logging(&service, &connection).await;
        Ok(Self::ready(service, connection, diagnostics))
    }

    /// Connect via stdio: spawn command, stdin/stdout = MCP.
    ///
    /// # Errors
    /// Returns an error if spawning the MCP subprocess fails, the handshake times out,
    /// or the server rejects initialization.
    pub async fn connect_stdio(
        command: &str,
        args: &[String],
        init_params: InitializeRequestParams,
        timeout: Option<Duration>,
    ) -> Result<Self> {
        let connection = ClientConnectionContext::stdio(command, args);
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped());
        let (transport, _stderr) = TokioChildProcess::builder(cmd)
            .spawn()
            .map_err(|error| anyhow!("spawn MCP process: {error}"))?;
        let diagnostics = Arc::new(McpClientDiagnostics::default());
        let handler =
            ObservedClientHandler::new(init_params, connection.clone(), Arc::clone(&diagnostics));
        let service = connect_running_service(handler, transport, timeout, &connection).await?;
        maybe_enable_server_logging(&service, &connection).await;
        Ok(Self::ready(service, connection, diagnostics))
    }

    /// Return a cheap point-in-time snapshot of server notifications observed on this client.
    #[must_use]
    pub fn notification_stats_snapshot(&self) -> McpClientNotificationStats {
        self.diagnostics.snapshot()
    }

    /// Return the monotonic epoch of the most recent `tools/list_changed` notification.
    #[must_use]
    pub fn tool_list_changed_epoch(&self) -> u64 {
        self.diagnostics
            .tool_list_changed_epoch
            .load(Ordering::Relaxed)
    }

    /// List tools from the MCP server.
    ///
    /// # Errors
    /// Returns an error if the client has not connected yet or if the server fails `tools/list`.
    pub async fn list_tools(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> Result<rmcp::model::ListToolsResult> {
        let service = self.ready_service().await?;
        let started = Instant::now();
        tracing::debug!(
            event = "mcp.client.list_tools.start",
            transport = self.connection.transport,
            target = %self.connection.target,
            has_params = params.is_some(),
            "mcp client tools/list started"
        );
        let result = service
            .list_tools(params)
            .await
            .map_err(|error| anyhow!("tools/list: {error}"));
        match &result {
            Ok(output) => {
                tracing::debug!(
                    event = "mcp.client.list_tools.ready",
                    transport = self.connection.transport,
                    target = %self.connection.target,
                    duration_ms = started.elapsed().as_millis(),
                    tool_count = output.tools.len(),
                    "mcp client tools/list completed"
                );
            }
            Err(error) => {
                tracing::warn!(
                    event = "mcp.client.list_tools.failed",
                    transport = self.connection.transport,
                    target = %self.connection.target,
                    duration_ms = started.elapsed().as_millis(),
                    error = %error,
                    "mcp client tools/list failed"
                );
            }
        }
        result
    }

    /// Call a tool by name with optional arguments.
    ///
    /// # Errors
    /// Returns an error if the client has not connected yet or if the server fails `tools/call`.
    pub async fn call_tool(
        &self,
        name: String,
        arguments: Option<serde_json::Value>,
    ) -> Result<rmcp::model::CallToolResult> {
        let service = self.ready_service().await?;
        let started = Instant::now();
        let args = arguments.and_then(|value| value.as_object().cloned());
        let args_present = args.is_some();
        let params = CallToolRequestParams {
            meta: None,
            name: name.clone().into(),
            arguments: args,
            task: None,
        };
        tracing::debug!(
            event = "mcp.client.call_tool.start",
            transport = self.connection.transport,
            target = %self.connection.target,
            tool = %name,
            arguments_present = args_present,
            "mcp client tools/call started"
        );
        let result = service
            .call_tool(params)
            .await
            .map_err(|error| anyhow!("tools/call: {error}"));
        match &result {
            Ok(output) => {
                tracing::debug!(
                    event = "mcp.client.call_tool.ready",
                    transport = self.connection.transport,
                    target = %self.connection.target,
                    tool = %name,
                    duration_ms = started.elapsed().as_millis(),
                    content_items = output.content.len(),
                    is_error = output.is_error.unwrap_or(false),
                    "mcp client tools/call completed"
                );
            }
            Err(error) => {
                tracing::warn!(
                    event = "mcp.client.call_tool.failed",
                    transport = self.connection.transport,
                    target = %self.connection.target,
                    tool = %name,
                    duration_ms = started.elapsed().as_millis(),
                    error = %error,
                    "mcp client tools/call failed"
                );
            }
        }
        result
    }

    async fn ready_service(&self) -> Result<Arc<RunningClientService>> {
        let guard = self.state.lock().await;
        match &*guard {
            ClientState::Ready { service } => Ok(Arc::clone(service)),
            ClientState::Connecting => Err(anyhow!("MCP client not initialized")),
        }
    }
}

async fn connect_running_service<T, E, A>(
    handler: ObservedClientHandler,
    transport: T,
    timeout: Option<Duration>,
    connection: &ClientConnectionContext,
) -> Result<RunningClientService>
where
    T: IntoTransport<RoleClient, E, A>,
    E: std::error::Error + Send + Sync + 'static,
{
    let started = Instant::now();
    tracing::debug!(
        event = "mcp.client.connect.start",
        transport = connection.transport,
        target = %connection.target,
        handshake_timeout_ms = timeout.map(duration_millis_u64),
        "mcp client handshake started"
    );

    let handshake_result = match timeout {
        Some(duration) => {
            if let Ok(result) = tokio::time::timeout(duration, handler.serve(transport)).await {
                result
            } else {
                tracing::warn!(
                    event = "mcp.client.connect.timeout",
                    transport = connection.transport,
                    target = %connection.target,
                    duration_ms = started.elapsed().as_millis(),
                    handshake_timeout_ms = duration_millis_u64(duration),
                    "mcp client handshake timed out"
                );
                return Err(anyhow!("MCP handshake timeout"));
            }
        }
        None => handler.serve(transport).await,
    };

    match handshake_result {
        Ok(service) => {
            log_connect_ready(connection, started, service.peer().peer_info());
            Ok(service)
        }
        Err(error) => {
            tracing::warn!(
                event = "mcp.client.connect.failed",
                transport = connection.transport,
                target = %connection.target,
                duration_ms = started.elapsed().as_millis(),
                error = %error,
                "mcp client handshake failed"
            );
            Err(anyhow!("MCP handshake: {error}"))
        }
    }
}

fn log_connect_ready(
    connection: &ClientConnectionContext,
    started: Instant,
    peer_info: Option<&ServerInfo>,
) {
    let (server_name, server_version, supports_logging, supports_tool_list_changed) = peer_info
        .map_or(("", "", false, false), |info| {
            (
                info.server_info.name.as_str(),
                info.server_info.version.as_str(),
                info.capabilities.logging.is_some(),
                info.capabilities
                    .tools
                    .as_ref()
                    .and_then(|capability| capability.list_changed)
                    .unwrap_or(false),
            )
        });
    tracing::info!(
        event = "mcp.client.connect.ready",
        transport = connection.transport,
        target = %connection.target,
        duration_ms = started.elapsed().as_millis(),
        server_name,
        server_version,
        supports_logging,
        supports_tool_list_changed,
        "mcp client handshake completed"
    );
}

async fn maybe_enable_server_logging(
    service: &RunningClientService,
    connection: &ClientConnectionContext,
) {
    let Some(server_info) = service.peer().peer_info() else {
        return;
    };
    if server_info.capabilities.logging.is_none() {
        return;
    }

    match service
        .peer()
        .set_level(SetLevelRequestParams {
            meta: None,
            level: LoggingLevel::Info,
        })
        .await
    {
        Ok(()) => tracing::debug!(
            event = "mcp.client.logging.set_level",
            transport = connection.transport,
            target = %connection.target,
            level = "info",
            "requested MCP server logging at info level"
        ),
        Err(error) => tracing::debug!(
            event = "mcp.client.logging.set_level.failed",
            transport = connection.transport,
            target = %connection.target,
            level = "info",
            error = %error,
            "failed to request MCP server logging level; continuing without log-level negotiation"
        ),
    }
}

fn duration_millis_u64(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

const fn logging_level_name(level: LoggingLevel) -> &'static str {
    match level {
        LoggingLevel::Debug => "debug",
        LoggingLevel::Info => "info",
        LoggingLevel::Notice => "notice",
        LoggingLevel::Warning => "warning",
        LoggingLevel::Error => "error",
        LoggingLevel::Critical => "critical",
        LoggingLevel::Alert => "alert",
        LoggingLevel::Emergency => "emergency",
    }
}
