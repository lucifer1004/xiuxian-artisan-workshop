#![allow(missing_docs, unused_imports, dead_code)]

//! External tool health readiness gate behavior.

use std::time::{Duration, Instant};

use axum::routing::get;
use axum::{Json, Router};
use omni_agent::{Agent, AgentConfig, ToolServerEntry};
use serde_json::json;

fn config_for(base_url: &str, retries: u32) -> AgentConfig {
    AgentConfig {
        tool_servers: vec![ToolServerEntry {
            name: "mock-tool".to_string(),
            url: Some(format!("{base_url}/sse")),
            command: None,
            args: None,
        }],
        tool_pool_size: 1,
        tool_handshake_timeout_secs: 1,
        tool_connect_retries: retries,
        tool_strict_startup: true,
        tool_connect_retry_backoff_ms: 10,
        ..Default::default()
    }
}

async fn spawn_server(app: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = match tokio::net::TcpListener::bind("127.0.0.1:0").await {
        Ok(listener) => listener,
        Err(error) => panic!("bind test listener: {error}"),
    };
    let addr = match listener.local_addr() {
        Ok(addr) => addr,
        Err(error) => panic!("read test listener addr: {error}"),
    };
    let handle = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (format!("http://{addr}"), handle)
}

#[tokio::test]
async fn startup_fails_when_structured_health_never_becomes_ready() {
    let app = Router::new().route(
        "/health",
        get(|| async {
            Json(json!({
                "status": "ok",
                "ready": false,
                "initializing": true,
                "active_sessions": 0,
            }))
        }),
    );
    let (base_url, server_task) = spawn_server(app).await;

    let started = Instant::now();
    let Err(error) = Agent::from_config(config_for(&base_url, 2)).await else {
        panic!("startup should fail when tool health is never ready");
    };
    server_task.abort();

    let message = format!("{error:#}");
    assert!(
        message.contains("health ready wait timed out"),
        "unexpected error message: {message}"
    );
    assert!(
        started.elapsed() >= Duration::from_millis(900),
        "health readiness gate should wait before failing"
    );
}

#[tokio::test]
async fn startup_keeps_legacy_handshake_path_when_health_is_not_structured() {
    let app = Router::new().route("/health", get(|| async { "ok" }));
    let (base_url, server_task) = spawn_server(app).await;

    let Err(error) = Agent::from_config(config_for(&base_url, 1)).await else {
        panic!("startup should fail because /sse is not a tool endpoint");
    };
    server_task.abort();

    let message = format!("{error:#}");
    assert!(
        !message.contains("health ready wait timed out"),
        "health gate should be skipped for non-structured health endpoints: {message}"
    );
    assert!(
        message.contains("connect failed after 1 attempts"),
        "unexpected error message: {message}"
    );
}
