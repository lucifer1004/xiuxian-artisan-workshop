//! Gateway command implementation - starts the Axum HTTP server.
//!
//! This module starts the Wendao API gateway server with:
//! - REST API endpoints for knowledge graph operations
//! - VFS access endpoints
//! - Health check endpoints
//! - Webhook notification integration
//! - Signal propagation to `NotificationService`

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use axum::{
    Json,
    extract::State,
    http::{HeaderValue, StatusCode, header::HeaderName},
    response::{IntoResponse, Response},
    routing::{Router, get},
};
use log::info;
use serde_json::json;
use tokio::sync::mpsc;

use crate::types::{Cli, GatewayArgs, GatewayCommand, GatewayStartArgs};
use xiuxian_wendao::LinkGraphIndex;
use xiuxian_wendao::analyzers::bootstrap_builtin_registry;
use xiuxian_wendao::gateway::{
    openapi::paths as openapi_paths,
    studio::{GatewayState, studio_routes},
};
use xiuxian_zhenfa::{NotificationService, WebhookConfig, ZhenfaSignal, notification_worker};

/// Default port for the gateway server.
const DEFAULT_PORT: u16 = 9517;
/// Environment variable that points at the pidfile owned by the managed gateway process.
const GATEWAY_PIDFILE_ENV: &str = "WENDAO_GATEWAY_PIDFILE";
/// Response header that exposes the current gateway process id to readiness probes.
const GATEWAY_PROCESS_ID_HEADER: &str = "x-wendao-process-id";

/// Shared state for the gateway server.
type AppState = GatewayState;

/// Handle the gateway command.
pub(crate) async fn handle(
    cli: &Cli,
    args: &GatewayArgs,
    index: Option<&LinkGraphIndex>,
) -> Result<()> {
    // Match the subcommand
    match &args.command {
        GatewayCommand::Start(start_args) => handle_start(cli, start_args, index).await,
    }
}

/// Handle the `gateway start` subcommand.
async fn handle_start(
    cli: &Cli,
    args: &GatewayStartArgs,
    index: Option<&LinkGraphIndex>,
) -> Result<()> {
    let config_path = resolve_config_path(cli.config_file.as_deref());

    // Resolve port: CLI arg > config file > default
    let port = resolve_port(args.port, config_path.as_deref());

    // 1. Start Webhook notification sidecar
    let (signal_tx, signal_rx) = mpsc::unbounded_channel::<ZhenfaSignal>();

    // Configure webhook: TOML > env var > defaults
    let webhook_config = resolve_webhook_config(config_path.as_deref());

    let notification_service = Arc::new(NotificationService::new(webhook_config));

    // Spawn the notification worker as a background task
    tokio::spawn(notification_worker(
        signal_rx,
        Arc::clone(&notification_service),
    ));
    info!(
        "Gateway: Notification worker started (id={})",
        notification_service.id()
    );

    // 2. Create app state with index and signal channel
    // Note: Julia/Modelica plugins should be registered here if this crate
    // depended on them. Since it doesn't (to avoid circular dependency),
    // they are currently empty. A separate aggregator crate would be needed
    // to provide a pre-populated registry.
    let app_state = Arc::new(AppState::new(
        index.map(|i| Arc::new(i.clone())),
        Some(signal_tx),
        build_plugin_registry()?,
    ));

    // 3. Build the Axum router
    let app = Router::new()
        .route(openapi_paths::API_HEALTH_AXUM_PATH, get(health))
        .route(openapi_paths::API_STATS_AXUM_PATH, get(stats))
        .route(openapi_paths::API_NOTIFY_AXUM_PATH, get(notify_status))
        .merge(studio_routes())
        .with_state(app_state);

    // 4. Start the server
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    info!("Starting Wendao Gateway on port {port}");
    info!("Endpoints:");
    info!(
        "  - GET {}  - Health check",
        openapi_paths::API_HEALTH_AXUM_PATH
    );
    info!(
        "  - GET {}   - Graph statistics",
        openapi_paths::API_STATS_AXUM_PATH
    );
    info!(
        "  - GET {}  - Notification service status",
        openapi_paths::API_NOTIFY_AXUM_PATH
    );

    let listener = tokio::net::TcpListener::bind(addr).await?;
    Ok(axum::serve(listener, app).await?)
}

/// Resolve the config file from CLI override, local project file, or `PRJ_ROOT`.
fn resolve_config_path(cli_config: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = cli_config {
        return Some(path.to_path_buf());
    }

    let local_config = Path::new("wendao.toml");
    if local_config.exists() {
        return Some(local_config.to_path_buf());
    }

    let config_path = std::env::var("PRJ_ROOT")
        .ok()
        .map(|root| Path::new(&root).join("wendao.toml"))?;
    config_path.exists().then_some(config_path)
}

/// Resolve the port from CLI arg, config file, or default.
fn resolve_port(cli_port: Option<u16>, config_path: Option<&Path>) -> u16 {
    // CLI arg takes highest priority
    if let Some(port) = cli_port {
        return port;
    }

    // Try config file
    if let Some(config_port) = get_port_from_config(config_path) {
        return config_port;
    }

    // Default
    DEFAULT_PORT
}

/// Get port from wendao.toml config file.
fn get_port_from_config(config_path: Option<&Path>) -> Option<u16> {
    parse_port_from_toml(config_path?)
}

/// Parse port from a TOML config file.
fn parse_port_from_toml(path: &std::path::Path) -> Option<u16> {
    use std::fs;
    use std::io::Read;

    let mut file = fs::File::open(path).ok()?;
    let mut content = String::new();
    file.read_to_string(&mut content).ok()?;

    // Parse [gateway] section for port
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("port") {
            // Parse: port = 9517 or port = "9517"
            if let Some(eq_pos) = line.find('=') {
                let value = line[eq_pos + 1..].trim().trim_matches('"');
                if let Ok(port) = value.parse::<u16>() {
                    return Some(port);
                }
            }
        }
    }

    None
}

/// Resolve webhook config with priority: TOML > env var > defaults.
fn resolve_webhook_config(config_path: Option<&Path>) -> WebhookConfig {
    // Try TOML config first (highest priority)
    if let Some(config) = get_webhook_from_config(config_path) {
        info!("Gateway: Using webhook config from wendao.toml");
        return config;
    }

    // Fall back to environment variables
    let url = std::env::var("WENDAO_WEBHOOK_URL").unwrap_or_default();
    if !url.is_empty() {
        info!("Gateway: Using webhook config from WENDAO_WEBHOOK_URL env var");
    }

    WebhookConfig {
        url,
        secret: std::env::var("WENDAO_WEBHOOK_SECRET").ok(),
        timeout_secs: 10,
        retry_on_failure: true,
    }
}

/// Get webhook config from wendao.toml config file.
fn get_webhook_from_config(config_path: Option<&Path>) -> Option<WebhookConfig> {
    parse_webhook_from_toml(config_path?)
}

/// Parse webhook config from a TOML config file.
fn parse_webhook_from_toml(path: &std::path::Path) -> Option<WebhookConfig> {
    use std::fs;
    use std::io::Read;

    let mut file = fs::File::open(path).ok()?;
    let mut content = String::new();
    file.read_to_string(&mut content).ok()?;

    let mut url = None;
    let mut secret = None;
    let mut enabled = true;

    // Parse [gateway] section for webhook settings
    let mut in_gateway_section = false;
    for line in content.lines() {
        let line = line.trim();

        // Track section
        if line == "[gateway]" {
            in_gateway_section = true;
            continue;
        } else if line.starts_with('[') && line.ends_with(']') {
            in_gateway_section = false;
            continue;
        }

        if !in_gateway_section {
            continue;
        }

        // Parse settings
        if line.starts_with("webhook_url") {
            if let Some(eq_pos) = line.find('=') {
                let value = line[eq_pos + 1..].trim().trim_matches('"');
                if !value.is_empty() && !value.starts_with('#') {
                    url = Some(value.to_string());
                }
            }
        } else if line.starts_with("webhook_secret") {
            if let Some(eq_pos) = line.find('=') {
                let value = line[eq_pos + 1..].trim().trim_matches('"');
                if !value.is_empty() {
                    secret = Some(value.to_string());
                }
            }
        } else if line.starts_with("webhook_enabled")
            && let Some(eq_pos) = line.find('=')
        {
            let value = line[eq_pos + 1..].trim();
            enabled = value.eq_ignore_ascii_case("true");
        }
    }

    if !enabled {
        return None;
    }

    // Only return config if URL was found
    url.map(|u| WebhookConfig {
        url: u,
        secret,
        timeout_secs: 10,
        retry_on_failure: true,
    })
}

/// Health check endpoint.
async fn health() -> Response {
    gateway_health_response(gateway_pidfile_from_env().as_deref())
}

fn gateway_pidfile_from_env() -> Option<PathBuf> {
    std::env::var_os(GATEWAY_PIDFILE_ENV)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
}

fn gateway_health_response(pidfile: Option<&Path>) -> Response {
    let process_id = std::process::id();
    let mut response = match pidfile {
        Some(pidfile) => match gateway_pidfile_process_id(pidfile) {
            Ok(expected_process_id) if expected_process_id == process_id => {
                Json("ok").into_response()
            }
            Ok(expected_process_id) => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error": "gateway is not ready",
                    "pidfile": pidfile.display().to_string(),
                    "expectedPid": expected_process_id,
                    "processId": process_id,
                })),
            )
                .into_response(),
            Err(details) => (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error": "gateway is not ready",
                    "pidfile": pidfile.display().to_string(),
                    "details": details,
                    "processId": process_id,
                })),
            )
                .into_response(),
        },
        None => Json("ok").into_response(),
    };

    response.headers_mut().insert(
        HeaderName::from_static(GATEWAY_PROCESS_ID_HEADER),
        HeaderValue::from_str(&process_id.to_string()).unwrap_or_else(|error| {
            panic!("gateway process id should be a valid header value: {error}")
        }),
    );

    response
}

fn gateway_pidfile_process_id(pidfile: &Path) -> Result<u32, String> {
    let contents = std::fs::read_to_string(pidfile)
        .map_err(|error| format!("failed to read pidfile {}: {error}", pidfile.display()))?;
    contents.trim().parse::<u32>().map_err(|error| {
        format!(
            "failed to parse pidfile {} as a process id: {error}",
            pidfile.display()
        )
    })
}

/// Stats endpoint.
async fn stats(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    match &state.index {
        Some(index) => {
            let payload = LinkGraphIndex::stats(index.as_ref());
            Json(
                serde_json::to_value(payload)
                    .unwrap_or_else(|_| json!({"error": "serialization failed"})),
            )
        }
        None => Json(json!({"error": "no index loaded"})),
    }
}

/// Notification service status endpoint.
async fn notify_status(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let has_signal_channel = state.signal_tx.is_some();
    let webhook_url =
        std::env::var("WENDAO_WEBHOOK_URL").unwrap_or_else(|_| "not configured".to_string());

    Json(json!({
        "notification_worker": if has_signal_channel { "active" } else { "inactive" },
        "webhook_configured": !webhook_url.is_empty(),
        "webhook_url": if webhook_url.is_empty() { serde_json::Value::Null } else { json!(webhook_url) }
    }))
}

fn build_plugin_registry() -> Result<Arc<xiuxian_wendao::analyzers::PluginRegistry>> {
    Ok(Arc::new(bootstrap_builtin_registry()?))
}

#[cfg(test)]
#[path = "../../../../tests/unit/bin/wendao/execute/gateway.rs"]
mod tests;
