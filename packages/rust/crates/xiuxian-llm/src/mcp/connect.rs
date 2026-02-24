//! MCP connect/retry orchestration.

use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use tokio::sync::oneshot;

use crate::mcp::config::McpPoolConnectConfig;
use crate::mcp::health::{probe_health_status, probe_health_summary};
use crate::mcp::transport_error::classify_transport_error;
use xiuxian_mcp::{OmniMcpClient, init_params_omni_server};

const DEFAULT_INFLIGHT_LOG_INTERVAL_SECS: u64 = 5;
const DEFAULT_HEALTH_READY_POLL_MS: u64 = 200;
const MAX_CONNECT_RETRY_BACKOFF_MS: u64 = 30_000;
const MAX_HANDSHAKE_TIMEOUT_SECS: u64 = 120;
const MAX_HEALTH_READY_WAIT_SECS: u64 = 180;

enum ConnectAttemptOutcome {
    Connected(OmniMcpClient),
    Failed(anyhow::Error),
}

struct ConnectAttemptMeta<'a> {
    url: &'a str,
    client_index: usize,
    attempt: u32,
    retries: u32,
    handshake_timeout_secs: u64,
    started: Instant,
}

fn log_connect_attempt_started(
    url: &str,
    client_index: usize,
    attempt: u32,
    retries: u32,
    handshake_timeout_secs: u64,
    pre_health_probe: &str,
) {
    tracing::debug!(
        event = "mcp.pool.connect.attempt",
        url,
        client_index,
        attempt,
        retries,
        handshake_timeout_secs,
        pre_health_probe = %pre_health_probe,
        "mcp pool client connect attempt started"
    );
}

fn log_connect_attempt_succeeded(meta: &ConnectAttemptMeta<'_>) {
    tracing::info!(
        event = "mcp.pool.connect.succeeded",
        url = meta.url,
        client_index = meta.client_index,
        attempt = meta.attempt,
        retries = meta.retries,
        handshake_timeout_secs = meta.handshake_timeout_secs,
        duration_ms = meta.started.elapsed().as_millis(),
        "mcp pool client connected"
    );
}

async fn log_connect_attempt_failed(
    meta: &ConnectAttemptMeta<'_>,
    error: &anyhow::Error,
    error_class: &str,
    message: &'static str,
) {
    let health_probe = probe_health_summary(meta.url).await;
    tracing::warn!(
        event = "mcp.pool.connect.failed",
        url = meta.url,
        client_index = meta.client_index,
        attempt = meta.attempt,
        retries = meta.retries,
        handshake_timeout_secs = meta.handshake_timeout_secs,
        duration_ms = meta.started.elapsed().as_millis(),
        health_probe = %health_probe,
        error_class,
        error = %error,
        "{message}"
    );
}

async fn connect_attempt(
    url: &str,
    client_index: usize,
    attempt: u32,
    retries: u32,
    handshake_timeout_secs: u64,
) -> ConnectAttemptOutcome {
    let pre_health_probe = probe_health_summary(url).await;
    log_connect_attempt_started(
        url,
        client_index,
        attempt,
        retries,
        handshake_timeout_secs,
        &pre_health_probe,
    );

    let started = Instant::now();
    let (connect_wait_logger, connect_wait_logger_stop) = spawn_connect_wait_logger(
        url.to_string(),
        client_index,
        attempt,
        retries,
        handshake_timeout_secs,
    );
    let url_owned = url.to_string();
    let mut connect_task = tokio::spawn(async move {
        OmniMcpClient::connect_streamable_http(
            &url_owned,
            init_params_omni_server(),
            Some(Duration::from_secs(handshake_timeout_secs)),
        )
        .await
    });
    let connect_result = tokio::time::timeout(
        Duration::from_secs(handshake_timeout_secs),
        &mut connect_task,
    )
    .await;
    stop_wait_logger(connect_wait_logger, connect_wait_logger_stop).await;
    let meta = ConnectAttemptMeta {
        url,
        client_index,
        attempt,
        retries,
        handshake_timeout_secs,
        started,
    };

    match connect_result {
        Ok(Ok(Ok(client))) => {
            log_connect_attempt_succeeded(&meta);
            ConnectAttemptOutcome::Connected(client)
        }
        Ok(Ok(Err(error))) => {
            let error_class = classify_transport_error(&error);
            log_connect_attempt_failed(
                &meta,
                &error,
                error_class.kind,
                "mcp pool client connect failed",
            )
            .await;
            ConnectAttemptOutcome::Failed(error)
        }
        Ok(Err(join_error)) => {
            let error = anyhow!(
                "MCP connect worker task join failed (url={url}, client_index={client_index}, attempt={attempt}, error={join_error})"
            );
            let error_class = classify_transport_error(&error);
            log_connect_attempt_failed(
                &meta,
                &error,
                error_class.kind,
                "mcp pool client connect failed",
            )
            .await;
            ConnectAttemptOutcome::Failed(error)
        }
        Err(_) => {
            connect_task.abort();
            let error = anyhow!("MCP handshake timeout");
            log_connect_attempt_failed(
                &meta,
                &error,
                "timeout",
                "mcp pool client connect hard timeout reached; worker task aborted",
            )
            .await;
            ConnectAttemptOutcome::Failed(error)
        }
    }
}

async fn maybe_sleep_before_retry(base_backoff_ms: u64, attempt: u32, retries: u32) {
    if attempt >= retries {
        return;
    }
    let delay_ms = compute_retry_backoff_ms(base_backoff_ms, attempt, retries);
    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
}

/// Connect one MCP client with bounded retries and readiness gating.
///
/// # Errors
/// Returns an error when readiness/handshake fails after retries.
pub async fn connect_one_client_with_retry(
    url: &str,
    config: McpPoolConnectConfig,
    retries: u32,
    client_index: usize,
) -> Result<OmniMcpClient> {
    let handshake_timeout_secs = config.handshake_timeout_secs.max(1);
    let retry_backoff_ms = config.connect_retry_backoff_ms.max(1);
    let health_wait_secs = compute_health_ready_wait_secs(handshake_timeout_secs, retries);
    wait_for_mcp_ready(url, client_index, health_wait_secs).await?;
    let mut last_error = None;
    for attempt in 1..=retries {
        let attempt_timeout_secs = compute_handshake_timeout_secs(handshake_timeout_secs, attempt);
        match connect_attempt(url, client_index, attempt, retries, attempt_timeout_secs).await {
            ConnectAttemptOutcome::Connected(client) => return Ok(client),
            ConnectAttemptOutcome::Failed(error) => {
                last_error = Some(error);
                maybe_sleep_before_retry(retry_backoff_ms, attempt, retries).await;
            }
        }
    }

    let last_error = last_error.unwrap_or_else(|| anyhow!("unknown mcp connect error"));
    Err(anyhow!(
        "MCP connect failed after {retries} attempts (url={url}, client_index={client_index}, handshake_timeout_secs_base={handshake_timeout_secs}, last_error={last_error})"
    ))
}

async fn stop_wait_logger(handle: tokio::task::JoinHandle<()>, stop_tx: oneshot::Sender<()>) {
    let _ = stop_tx.send(());
    let _ = handle.await;
}

fn spawn_connect_wait_logger(
    url: String,
    client_index: usize,
    attempt: u32,
    retries: u32,
    handshake_timeout_secs: u64,
) -> (tokio::task::JoinHandle<()>, oneshot::Sender<()>) {
    let (stop_tx, mut stop_rx) = oneshot::channel::<()>();
    let timeout_secs = handshake_timeout_secs.max(1);
    let overdue_limit_secs = timeout_secs.saturating_add(DEFAULT_INFLIGHT_LOG_INTERVAL_SECS);
    let handle = tokio::spawn(async move {
        let mut waited_secs = DEFAULT_INFLIGHT_LOG_INTERVAL_SECS;
        loop {
            tokio::select! {
                () = tokio::time::sleep(Duration::from_secs(DEFAULT_INFLIGHT_LOG_INTERVAL_SECS)) => {}
                _ = &mut stop_rx => break,
            }
            tracing::warn!(
                event = "mcp.pool.connect.waiting",
                url = %url,
                client_index,
                attempt,
                retries,
                waited_secs,
                handshake_timeout_secs = timeout_secs,
                "mcp connect attempt still waiting"
            );
            if waited_secs >= overdue_limit_secs {
                tracing::warn!(
                    event = "mcp.pool.connect.waiting.guard_stop",
                    url = %url,
                    client_index,
                    attempt,
                    retries,
                    waited_secs,
                    handshake_timeout_secs = timeout_secs,
                    "mcp connect wait logger stopped after exceeding timeout guard"
                );
                break;
            }
            waited_secs += DEFAULT_INFLIGHT_LOG_INTERVAL_SECS;
        }
    });
    (handle, stop_tx)
}

fn compute_retry_backoff_ms(base_ms: u64, attempt: u32, retries: u32) -> u64 {
    if retries <= 1 {
        return 0;
    }
    let shift = attempt.saturating_sub(1).min(8);
    let multiplier = 1_u64 << shift;
    base_ms
        .saturating_mul(multiplier)
        .min(MAX_CONNECT_RETRY_BACKOFF_MS)
}

fn compute_handshake_timeout_secs(base_secs: u64, attempt: u32) -> u64 {
    let shift = attempt.saturating_sub(1).min(2);
    let multiplier = 1_u64 << shift;
    base_secs
        .saturating_mul(multiplier)
        .min(MAX_HANDSHAKE_TIMEOUT_SECS)
}

fn compute_health_ready_wait_secs(base_secs: u64, retries: u32) -> u64 {
    let effective_retries = u64::from(retries.max(1));
    base_secs
        .max(1)
        .saturating_mul(effective_retries)
        .min(MAX_HEALTH_READY_WAIT_SECS)
}

async fn wait_for_mcp_ready(url: &str, client_index: usize, wait_secs: u64) -> Result<()> {
    let wait_secs = wait_secs.clamp(1, MAX_HEALTH_READY_WAIT_SECS);
    let deadline = Instant::now() + Duration::from_secs(wait_secs);
    let mut probe = probe_health_status(url).await;

    if !probe.has_structured_ready_state {
        tracing::debug!(
            event = "mcp.pool.health.wait.skipped",
            url,
            client_index,
            health_probe = %probe.summary,
            "mcp health readiness gate skipped (structured fields unavailable)"
        );
        return Ok(());
    }

    tracing::debug!(
        event = "mcp.pool.health.wait.start",
        url,
        client_index,
        wait_secs,
        health_probe = %probe.summary,
        "mcp health readiness gate started"
    );
    loop {
        if probe.ready == Some(true) && probe.initializing != Some(true) {
            tracing::debug!(
                event = "mcp.pool.health.wait.ready",
                url,
                client_index,
                wait_secs,
                health_probe = %probe.summary,
                "mcp health readiness gate passed"
            );
            return Ok(());
        }

        if Instant::now() >= deadline {
            tracing::warn!(
                event = "mcp.pool.health.wait.timeout",
                url,
                client_index,
                wait_secs,
                health_probe = %probe.summary,
                "mcp health readiness gate timed out"
            );
            return Err(anyhow!(
                "MCP health ready wait timed out after {}s (url={}, client_index={}, last_probe={})",
                wait_secs,
                url,
                client_index,
                probe.summary
            ));
        }

        tokio::time::sleep(Duration::from_millis(DEFAULT_HEALTH_READY_POLL_MS)).await;
        probe = probe_health_status(url).await;
    }
}
