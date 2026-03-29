use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, anyhow};
use redis::Client as ValkeyClient;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ClientCapabilities, InitializeRequestParams,
    ListToolsResult, PaginatedRequestParams, ProtocolVersion,
};
use rmcp::service::{RoleClient, RunningService, serve_client};
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use serde::Serialize;
use sha2::{Digest, Sha256};
use tokio::sync::{Mutex, RwLock};

const DISCOVER_TOOL_NAME: &str = "skill.discover";
const DEFAULT_PROTOCOL_VERSION: ProtocolVersion = ProtocolVersion::V_2025_06_18;

#[derive(Clone, Debug, Serialize)]
pub struct ToolPoolConnectConfig {
    pub pool_size: usize,
    pub handshake_timeout_secs: u64,
    pub connect_retries: u32,
    pub connect_retry_backoff_ms: u64,
    pub tool_timeout_secs: u64,
    pub list_tools_cache_ttl_ms: u64,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ToolListCacheStatsSnapshot {
    pub requests_total: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_refreshes: u64,
    pub refresh_errors: u64,
    pub last_refresh_unix_ms: Option<u64>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct ToolDiscoverCacheStatsSnapshot {
    pub requests_total: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub cache_writes: u64,
    pub hit_rate_pct: f64,
}

#[derive(Clone, Debug)]
pub struct ToolDiscoverCacheConfig {
    pub valkey_url: String,
    pub key_prefix: String,
    pub ttl_secs: u64,
}

#[derive(Clone, Debug)]
pub struct ToolDiscoverCacheRuntimeInfo {
    pub backend: &'static str,
    pub ttl_secs: u64,
}

#[derive(Debug)]
struct ToolListCacheStats {
    requests_total: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    cache_refreshes: AtomicU64,
    refresh_errors: AtomicU64,
    last_refresh_unix_ms: AtomicU64,
}

impl Default for ToolListCacheStats {
    fn default() -> Self {
        Self {
            requests_total: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            cache_refreshes: AtomicU64::new(0),
            refresh_errors: AtomicU64::new(0),
            last_refresh_unix_ms: AtomicU64::new(0),
        }
    }
}

#[derive(Debug)]
struct ToolDiscoverCacheStats {
    requests_total: AtomicU64,
    cache_hits: AtomicU64,
    cache_misses: AtomicU64,
    cache_writes: AtomicU64,
}

impl Default for ToolDiscoverCacheStats {
    fn default() -> Self {
        Self {
            requests_total: AtomicU64::new(0),
            cache_hits: AtomicU64::new(0),
            cache_misses: AtomicU64::new(0),
            cache_writes: AtomicU64::new(0),
        }
    }
}

#[derive(Debug)]
pub struct ToolDiscoverReadThroughCache {
    client: ValkeyClient,
    key_prefix: String,
    ttl_secs: u64,
    stats: Arc<ToolDiscoverCacheStats>,
}

impl ToolDiscoverReadThroughCache {
    pub fn from_config(config: ToolDiscoverCacheConfig) -> Result<Self> {
        let client = ValkeyClient::open(config.valkey_url.clone())
            .with_context(|| format!("open discover cache valkey client: {}", config.valkey_url))?;
        Ok(Self {
            client,
            key_prefix: config.key_prefix,
            ttl_secs: config.ttl_secs,
            stats: Arc::new(ToolDiscoverCacheStats::default()),
        })
    }

    #[must_use]
    pub fn runtime_info(&self) -> ToolDiscoverCacheRuntimeInfo {
        ToolDiscoverCacheRuntimeInfo {
            backend: "valkey",
            ttl_secs: self.ttl_secs,
        }
    }

    #[must_use]
    pub fn stats_snapshot(&self) -> ToolDiscoverCacheStatsSnapshot {
        let requests_total = self.stats.requests_total.load(Ordering::Relaxed);
        let cache_hits = self.stats.cache_hits.load(Ordering::Relaxed);
        let cache_misses = self.stats.cache_misses.load(Ordering::Relaxed);
        let cache_writes = self.stats.cache_writes.load(Ordering::Relaxed);
        let hit_rate_pct = if requests_total == 0 {
            0.0
        } else {
            (cache_hits as f64 / requests_total as f64) * 100.0
        };
        ToolDiscoverCacheStatsSnapshot {
            requests_total,
            cache_hits,
            cache_misses,
            cache_writes,
            hit_rate_pct,
        }
    }

    pub async fn lookup(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> Result<Option<CallToolResult>> {
        self.stats.requests_total.fetch_add(1, Ordering::Relaxed);
        let key = self.cache_key(tool_name, arguments)?;
        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .await
            .context("connect discover cache valkey")?;
        let payload: Option<String> = redis::cmd("GET")
            .arg(&key)
            .query_async(&mut connection)
            .await
            .with_context(|| format!("discover cache GET {key}"))?;
        match payload {
            Some(payload) => {
                self.stats.cache_hits.fetch_add(1, Ordering::Relaxed);
                let result = serde_json::from_str(&payload)
                    .with_context(|| format!("decode discover cache payload for {key}"))?;
                Ok(Some(result))
            }
            None => {
                self.stats.cache_misses.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            }
        }
    }

    pub async fn store(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
        result: &CallToolResult,
    ) -> Result<()> {
        let key = self.cache_key(tool_name, arguments)?;
        let payload = serde_json::to_string(result)
            .with_context(|| format!("encode discover cache payload for {key}"))?;
        let mut connection = self
            .client
            .get_multiplexed_async_connection()
            .await
            .context("connect discover cache valkey")?;
        let _: () = redis::cmd("SETEX")
            .arg(&key)
            .arg(self.ttl_secs)
            .arg(payload)
            .query_async(&mut connection)
            .await
            .with_context(|| format!("discover cache SETEX {key}"))?;
        self.stats.cache_writes.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn cache_key(&self, tool_name: &str, arguments: &serde_json::Value) -> Result<String> {
        let normalized = normalize_json(arguments);
        let canonical =
            serde_json::to_string(&normalized).context("serialize discover cache key")?;
        let mut digest = Sha256::new();
        digest.update(tool_name.as_bytes());
        digest.update([0]);
        digest.update(canonical.as_bytes());
        let digest_hex = hex::encode(digest.finalize());
        Ok(format!("{}:{tool_name}:{digest_hex}", self.key_prefix))
    }
}

type ToolClientService = RunningService<RoleClient, InitializeRequestParams>;

#[derive(Debug)]
struct ToolRuntimeState {
    service: Option<Arc<ToolClientService>>,
}

#[derive(Debug)]
struct CachedToolList {
    value: ListToolsResult,
    expires_at: Instant,
}

#[derive(Debug)]
pub struct ToolClientPool {
    url: String,
    config: ToolPoolConnectConfig,
    state: Mutex<ToolRuntimeState>,
    list_cache: RwLock<Option<CachedToolList>>,
    list_stats: Arc<ToolListCacheStats>,
    discover_cache: Option<Arc<ToolDiscoverReadThroughCache>>,
}

impl ToolClientPool {
    pub async fn list_tools(
        &self,
        params: Option<PaginatedRequestParams>,
    ) -> Result<ListToolsResult> {
        self.list_stats
            .requests_total
            .fetch_add(1, Ordering::Relaxed);
        if params.is_none()
            && let Some(cached) = self.read_cached_tools().await
        {
            self.list_stats.cache_hits.fetch_add(1, Ordering::Relaxed);
            return Ok(cached);
        }
        self.list_stats.cache_misses.fetch_add(1, Ordering::Relaxed);

        let timeout_secs = self.config.tool_timeout_secs.max(1);
        let list = self
            .call_with_retry("tools/list", timeout_secs, |service| async move {
                service
                    .list_tools(params.clone())
                    .await
                    .map_err(|error| anyhow!("tools/list failed: {error}"))
            })
            .await?;
        if params.is_none() {
            self.store_cached_tools(list.clone()).await;
        }
        Ok(list)
    }

    pub async fn call_tool(
        &self,
        name: String,
        arguments: Option<serde_json::Value>,
    ) -> Result<CallToolResult> {
        if let Some(discover_cache) = self.discover_cache.as_ref()
            && name == DISCOVER_TOOL_NAME
            && let Some(arguments) = arguments.as_ref()
        {
            if let Some(result) = discover_cache.lookup(&name, arguments).await? {
                return Ok(result);
            }
        }

        let timeout_secs = self.config.tool_timeout_secs.max(1);
        let result = self
            .call_with_retry("tools/call", timeout_secs, |service| async move {
                let params = CallToolRequestParams {
                    meta: None,
                    name: name.clone().into(),
                    arguments: arguments
                        .clone()
                        .and_then(|value| value.as_object().cloned()),
                    task: None,
                };
                service
                    .call_tool(params)
                    .await
                    .map_err(|error| anyhow!("tools/call failed: {error}"))
            })
            .await?;

        if let Some(discover_cache) = self.discover_cache.as_ref()
            && name == DISCOVER_TOOL_NAME
            && let Some(arguments) = arguments.as_ref()
            && result.is_error != Some(true)
            && let Err(error) = discover_cache.store(&name, arguments, &result).await
        {
            tracing::warn!(
                event = "tool_runtime.discover_cache.store_failed",
                tool_name = %name,
                error = %error,
                "discover cache store failed; continuing without cached write"
            );
        }

        Ok(result)
    }

    #[must_use]
    pub fn tools_list_cache_stats_snapshot(&self) -> ToolListCacheStatsSnapshot {
        let last_refresh_unix_ms =
            match self.list_stats.last_refresh_unix_ms.load(Ordering::Relaxed) {
                0 => None,
                value => Some(value),
            };
        ToolListCacheStatsSnapshot {
            requests_total: self.list_stats.requests_total.load(Ordering::Relaxed),
            cache_hits: self.list_stats.cache_hits.load(Ordering::Relaxed),
            cache_misses: self.list_stats.cache_misses.load(Ordering::Relaxed),
            cache_refreshes: self.list_stats.cache_refreshes.load(Ordering::Relaxed),
            refresh_errors: self.list_stats.refresh_errors.load(Ordering::Relaxed),
            last_refresh_unix_ms,
        }
    }

    #[must_use]
    pub fn discover_cache_stats_snapshot(&self) -> Option<ToolDiscoverCacheStatsSnapshot> {
        self.discover_cache
            .as_ref()
            .map(|cache| cache.stats_snapshot())
    }

    async fn call_with_retry<T, F, Fut>(
        &self,
        op_name: &'static str,
        timeout_secs: u64,
        mut op: F,
    ) -> Result<T>
    where
        F: FnMut(Arc<ToolClientService>) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let max_attempts = self.config.connect_retries.max(1);
        let timeout = Duration::from_secs(timeout_secs);
        let backoff = Duration::from_millis(self.config.connect_retry_backoff_ms.max(1));

        let mut last_error = None;
        for attempt in 1..=max_attempts {
            let service = self.connected_service().await?;
            let outcome = tokio::time::timeout(timeout, op(service)).await;
            match outcome {
                Ok(Ok(value)) => return Ok(value),
                Ok(Err(error)) => {
                    last_error = Some(error);
                    self.invalidate_connection().await;
                }
                Err(_) => {
                    self.invalidate_connection().await;
                    last_error = Some(anyhow!("{op_name} timed out after {timeout_secs}s"));
                }
            }
            if attempt < max_attempts {
                tokio::time::sleep(backoff).await;
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow!("{op_name} failed without error")))
    }

    async fn connected_service(&self) -> Result<Arc<ToolClientService>> {
        let mut guard = self.state.lock().await;
        if let Some(service) = guard.service.as_ref()
            && !service.is_closed()
        {
            return Ok(Arc::clone(service));
        }
        let service =
            Arc::new(connect_service(&self.url, self.config.handshake_timeout_secs).await?);
        guard.service = Some(Arc::clone(&service));
        Ok(service)
    }

    async fn invalidate_connection(&self) {
        let mut guard = self.state.lock().await;
        guard.service = None;
    }

    async fn read_cached_tools(&self) -> Option<ListToolsResult> {
        let guard = self.list_cache.read().await;
        let cached = guard.as_ref()?;
        (Instant::now() < cached.expires_at).then(|| cached.value.clone())
    }

    async fn store_cached_tools(&self, list: ListToolsResult) {
        self.list_stats
            .cache_refreshes
            .fetch_add(1, Ordering::Relaxed);
        self.list_stats
            .last_refresh_unix_ms
            .store(now_unix_ms().unwrap_or_default(), Ordering::Relaxed);
        let ttl_ms = self.config.list_tools_cache_ttl_ms.max(1);
        let expires_at = Instant::now() + Duration::from_millis(ttl_ms);
        let mut guard = self.list_cache.write().await;
        *guard = Some(CachedToolList {
            value: list,
            expires_at,
        });
    }
}

pub(crate) async fn connect_tool_pool_backend(
    url: &str,
    config: ToolPoolConnectConfig,
    discover_cache: Option<Arc<ToolDiscoverReadThroughCache>>,
) -> Result<ToolClientPool> {
    let pool = ToolClientPool {
        url: url.to_string(),
        config,
        state: Mutex::new(ToolRuntimeState { service: None }),
        list_cache: RwLock::new(None),
        list_stats: Arc::new(ToolListCacheStats::default()),
        discover_cache,
    };
    let service = connect_service(url, pool.config.handshake_timeout_secs).await?;
    {
        let mut guard = pool.state.lock().await;
        guard.service = Some(Arc::new(service));
    }
    Ok(pool)
}

async fn connect_service(url: &str, handshake_timeout_secs: u64) -> Result<ToolClientService> {
    let init_params = InitializeRequestParams {
        meta: None,
        protocol_version: DEFAULT_PROTOCOL_VERSION,
        capabilities: ClientCapabilities::default(),
        client_info: rmcp::model::Implementation::from_build_env(),
    };
    let transport_config = StreamableHttpClientTransportConfig::with_uri(url.to_string());
    let http_client = reqwest::Client::builder()
        .build()
        .context("build tool runtime HTTP client")?;
    let transport = StreamableHttpClientTransport::with_client(http_client, transport_config);
    let timeout = Duration::from_secs(handshake_timeout_secs.max(1));
    tokio::time::timeout(timeout, serve_client(init_params, transport))
        .await
        .map_err(|_| {
            anyhow!(
                "tool runtime handshake timed out after {}s",
                timeout.as_secs()
            )
        })?
        .map_err(|error| anyhow!("tool runtime handshake failed: {error}"))
}

fn normalize_json(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let sorted: BTreeMap<String, serde_json::Value> = map
                .iter()
                .map(|(key, value)| (key.clone(), normalize_json(value)))
                .collect();
            let normalized = sorted.into_iter().collect();
            serde_json::Value::Object(normalized)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(normalize_json).collect())
        }
        _ => value.clone(),
    }
}

fn now_unix_ms() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}
