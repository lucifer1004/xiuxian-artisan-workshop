//! Gateway command execution.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::routing::{Router, get};
use log::info;
use tokio::sync::mpsc;

use crate::execute::gateway::{
    config::{resolve_config_path, resolve_port, resolve_webhook_config},
    health::health,
    registry::build_plugin_registry,
    shared::AppState,
    status::{notify_status, stats},
};
use crate::types::{Cli, GatewayArgs, GatewayCommand, GatewayStartArgs};
use xiuxian_wendao::LinkGraphIndex;
use xiuxian_wendao::gateway::{openapi::paths as openapi_paths, studio::studio_routes};
use xiuxian_zhenfa::{NotificationService, ZhenfaSignal, notification_worker};

/// Handle the gateway command.
pub(crate) async fn handle(
    cli: &Cli,
    args: &GatewayArgs,
    index: Option<&LinkGraphIndex>,
) -> Result<()> {
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
