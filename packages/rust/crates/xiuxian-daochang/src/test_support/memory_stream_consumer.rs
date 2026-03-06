//! Memory stream-consumer helpers exposed for integration tests.

use std::collections::HashMap;
use std::time::Duration;

use crate::agent::{logging, memory_stream_consumer};

/// Test-facing stream event parsed from valkey stream payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryStreamEvent {
    pub id: String,
    pub fields: HashMap<String, String>,
}

/// Test-facing runtime config for stream-consumer helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryStreamConsumerRuntimeConfig {
    pub redis_url: String,
    pub stream_name: String,
    pub stream_key: String,
    pub promotion_stream_key: String,
    pub promotion_ledger_key: String,
    pub stream_consumer_group: String,
    pub stream_consumer_name: String,
    pub stream_consumer_batch_size: usize,
    pub stream_consumer_block_ms: u64,
    pub metrics_global_key: String,
    pub metrics_session_prefix: String,
    pub ttl_secs: Option<u64>,
}

/// Stream read-error classification bucket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamReadErrorKind {
    MissingConsumerGroup,
    Transport,
    Other,
}

/// Parse one `XREADGROUP` reply payload.
///
/// # Errors
///
/// Returns an error when the reply payload has unsupported structure.
pub fn parse_xreadgroup_reply(reply: redis::Value) -> anyhow::Result<Vec<MemoryStreamEvent>> {
    memory_stream_consumer::test_parse_xreadgroup_reply(reply).map(|events| {
        events
            .into_iter()
            .map(from_internal_event)
            .collect::<Vec<_>>()
    })
}

#[must_use]
pub fn build_consumer_name(prefix: &str) -> String {
    memory_stream_consumer::test_build_consumer_name(prefix)
}

#[must_use]
pub fn compute_retry_backoff_ms(base_ms: u64, failure_streak: u32) -> u64 {
    memory_stream_consumer::test_compute_retry_backoff_ms(base_ms, failure_streak)
}

#[must_use]
pub fn classify_stream_read_error(error: &anyhow::Error) -> StreamReadErrorKind {
    from_internal_stream_read_error_kind(memory_stream_consumer::test_classify_stream_read_error(
        error,
    ))
}

#[must_use]
pub fn stream_consumer_response_timeout(block_ms: u64) -> Duration {
    memory_stream_consumer::test_stream_consumer_response_timeout(block_ms)
}

#[must_use]
pub fn stream_consumer_connection_config(block_ms: u64) -> redis::AsyncConnectionConfig {
    memory_stream_consumer::test_stream_consumer_connection_config(block_ms)
}

#[must_use]
pub fn summarize_redis_error(error: &redis::RedisError) -> String {
    memory_stream_consumer::test_summarize_redis_error(error)
}

#[must_use]
pub fn is_idle_poll_timeout_error(error: &redis::RedisError) -> bool {
    memory_stream_consumer::test_is_idle_poll_timeout_error(error)
}

#[must_use]
pub fn should_surface_repeated_failure(failure_streak: u32) -> bool {
    logging::should_surface_repeated_failure(failure_streak)
}

/// Ensure stream consumer-group exists for current stream key.
///
/// # Errors
///
/// Returns an error when valkey group creation/check fails.
pub async fn ensure_consumer_group(
    connection: &mut redis::aio::MultiplexedConnection,
    config: &MemoryStreamConsumerRuntimeConfig,
) -> anyhow::Result<()> {
    memory_stream_consumer::test_ensure_consumer_group(connection, &to_internal_config(config))
        .await
}

/// Read stream events using `XREADGROUP`.
///
/// # Errors
///
/// Returns an error when valkey read fails.
pub async fn read_stream_events(
    connection: &mut redis::aio::MultiplexedConnection,
    config: &MemoryStreamConsumerRuntimeConfig,
    stream_id: &str,
) -> anyhow::Result<Vec<MemoryStreamEvent>> {
    memory_stream_consumer::test_read_stream_events(
        connection,
        &to_internal_config(config),
        stream_id,
    )
    .await
    .map(|events| {
        events
            .into_iter()
            .map(from_internal_event)
            .collect::<Vec<_>>()
    })
}

/// ACK one stream event and update metrics counters.
///
/// # Errors
///
/// Returns an error when valkey write operations fail.
pub async fn ack_and_record_metrics(
    connection: &mut redis::aio::MultiplexedConnection,
    config: &MemoryStreamConsumerRuntimeConfig,
    event_id: &str,
    kind: &str,
    session_id: Option<&str>,
) -> anyhow::Result<u64> {
    memory_stream_consumer::test_ack_and_record_metrics(
        connection,
        &to_internal_config(config),
        event_id,
        kind,
        session_id,
    )
    .await
}

/// Queue one promoted candidate into ingest stream + ledger.
///
/// # Errors
///
/// Returns an error when valkey write operations fail.
pub async fn queue_promoted_candidate(
    connection: &mut redis::aio::MultiplexedConnection,
    config: &MemoryStreamConsumerRuntimeConfig,
    event: &MemoryStreamEvent,
) -> anyhow::Result<bool> {
    memory_stream_consumer::test_queue_promoted_candidate(
        connection,
        &to_internal_config(config),
        &to_internal_event(event),
    )
    .await
}

fn from_internal_event(event: memory_stream_consumer::TestMemoryStreamEvent) -> MemoryStreamEvent {
    MemoryStreamEvent {
        id: event.id,
        fields: event.fields,
    }
}

fn to_internal_event(event: &MemoryStreamEvent) -> memory_stream_consumer::TestMemoryStreamEvent {
    memory_stream_consumer::TestMemoryStreamEvent {
        id: event.id.clone(),
        fields: event.fields.clone(),
    }
}

fn to_internal_config(
    config: &MemoryStreamConsumerRuntimeConfig,
) -> memory_stream_consumer::TestMemoryStreamConsumerRuntimeConfig {
    memory_stream_consumer::TestMemoryStreamConsumerRuntimeConfig {
        redis_url: config.redis_url.clone(),
        stream_name: config.stream_name.clone(),
        stream_key: config.stream_key.clone(),
        promotion_stream_key: config.promotion_stream_key.clone(),
        promotion_ledger_key: config.promotion_ledger_key.clone(),
        stream_consumer_group: config.stream_consumer_group.clone(),
        stream_consumer_name: config.stream_consumer_name.clone(),
        stream_consumer_batch_size: config.stream_consumer_batch_size,
        stream_consumer_block_ms: config.stream_consumer_block_ms,
        metrics_global_key: config.metrics_global_key.clone(),
        metrics_session_prefix: config.metrics_session_prefix.clone(),
        ttl_secs: config.ttl_secs,
    }
}

const fn from_internal_stream_read_error_kind(
    kind: memory_stream_consumer::TestStreamReadErrorKind,
) -> StreamReadErrorKind {
    match kind {
        memory_stream_consumer::TestStreamReadErrorKind::MissingConsumerGroup => {
            StreamReadErrorKind::MissingConsumerGroup
        }
        memory_stream_consumer::TestStreamReadErrorKind::Transport => {
            StreamReadErrorKind::Transport
        }
        memory_stream_consumer::TestStreamReadErrorKind::Other => StreamReadErrorKind::Other,
    }
}
