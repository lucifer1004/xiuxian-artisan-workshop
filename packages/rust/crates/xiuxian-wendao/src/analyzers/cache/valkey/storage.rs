use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::analyzers::cache::{RepositoryAnalysisCacheKey, RepositorySearchQueryCacheKey};
use crate::analyzers::plugin::RepositoryAnalysisOutput;

const ANALYZER_CACHE_SCHEMA_VERSION: &str = "xiuxian_wendao.repo_analysis_cache.v1";
const SEARCH_QUERY_CACHE_SCHEMA_VERSION: &str = "xiuxian_wendao.repo_search_query_cache.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ValkeyAnalysisCachePayload {
    schema: String,
    repo_id: String,
    revision: String,
    cached_at_rfc3339: String,
    analysis: RepositoryAnalysisOutput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ValkeySearchQueryCachePayload {
    schema: String,
    repo_id: String,
    revision: String,
    endpoint: String,
    query: String,
    filter: Option<String>,
    max_distance: u8,
    prefix_length: usize,
    transposition: bool,
    limit: usize,
    cached_at_rfc3339: String,
    value: serde_json::Value,
}

pub(super) fn valkey_analysis_key(
    cache_key: &RepositoryAnalysisCacheKey,
    key_prefix: &str,
) -> Option<String> {
    let revision = stable_revision(cache_key)?;
    let payload = format!(
        "repo:{}|root:{}|revision:{}|mirror:{}|tracking:{}|plugins:{}",
        cache_key.repo_id.trim(),
        cache_key.checkout_root.trim(),
        revision,
        cache_key
            .mirror_revision
            .as_deref()
            .unwrap_or_default()
            .trim(),
        cache_key
            .tracking_revision
            .as_deref()
            .unwrap_or_default()
            .trim(),
        cache_key.plugin_ids.join(","),
    );
    let token = blake3::hash(payload.as_bytes()).to_hex().to_string();
    Some(format!("{key_prefix}:analysis:{token}"))
}

pub(super) fn encode_analysis_payload(
    cache_key: &RepositoryAnalysisCacheKey,
    analysis: &RepositoryAnalysisOutput,
) -> Option<String> {
    let revision = stable_revision(cache_key)?;
    serde_json::to_string(&ValkeyAnalysisCachePayload {
        schema: ANALYZER_CACHE_SCHEMA_VERSION.to_string(),
        repo_id: cache_key.repo_id.clone(),
        revision: revision.to_string(),
        cached_at_rfc3339: Utc::now().to_rfc3339(),
        analysis: analysis.clone(),
    })
    .ok()
}

pub(super) fn valkey_search_query_key(
    cache_key: &RepositorySearchQueryCacheKey,
    key_prefix: &str,
) -> Option<String> {
    let revision = stable_revision(&cache_key.analysis_key)?;
    let payload = format!(
        "repo:{}|root:{}|revision:{}|mirror:{}|tracking:{}|plugins:{}|endpoint:{}|query:{}|filter:{}|distance:{}|prefix:{}|transpose:{}|limit:{}",
        cache_key.analysis_key.repo_id.trim(),
        cache_key.analysis_key.checkout_root.trim(),
        revision,
        cache_key
            .analysis_key
            .mirror_revision
            .as_deref()
            .unwrap_or_default()
            .trim(),
        cache_key
            .analysis_key
            .tracking_revision
            .as_deref()
            .unwrap_or_default()
            .trim(),
        cache_key.analysis_key.plugin_ids.join(","),
        cache_key.endpoint.trim(),
        cache_key.query.trim(),
        cache_key.filter.as_deref().unwrap_or_default().trim(),
        cache_key.max_distance,
        cache_key.prefix_length,
        cache_key.transposition,
        cache_key.limit,
    );
    let token = blake3::hash(payload.as_bytes()).to_hex().to_string();
    Some(format!("{key_prefix}:search-query:{token}"))
}

pub(super) fn encode_search_query_payload<T>(
    cache_key: &RepositorySearchQueryCacheKey,
    value: &T,
) -> Option<String>
where
    T: serde::Serialize,
{
    let revision = stable_revision(&cache_key.analysis_key)?;
    let encoded_value = serde_json::to_value(value).ok()?;
    serde_json::to_string(&ValkeySearchQueryCachePayload {
        schema: SEARCH_QUERY_CACHE_SCHEMA_VERSION.to_string(),
        repo_id: cache_key.analysis_key.repo_id.clone(),
        revision: revision.to_string(),
        endpoint: cache_key.endpoint.clone(),
        query: cache_key.query.clone(),
        filter: cache_key.filter.clone(),
        max_distance: cache_key.max_distance,
        prefix_length: cache_key.prefix_length,
        transposition: cache_key.transposition,
        limit: cache_key.limit,
        cached_at_rfc3339: Utc::now().to_rfc3339(),
        value: encoded_value,
    })
    .ok()
}

pub(super) fn decode_analysis_payload(
    cache_key: &RepositoryAnalysisCacheKey,
    payload: &str,
) -> Option<RepositoryAnalysisOutput> {
    let revision = stable_revision(cache_key)?;
    let decoded = serde_json::from_str::<ValkeyAnalysisCachePayload>(payload).ok()?;
    if decoded.schema != ANALYZER_CACHE_SCHEMA_VERSION {
        return None;
    }
    if decoded.repo_id != cache_key.repo_id || decoded.revision != revision {
        return None;
    }
    Some(decoded.analysis)
}

pub(super) fn decode_search_query_payload<T>(
    cache_key: &RepositorySearchQueryCacheKey,
    payload: &str,
) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    let revision = stable_revision(&cache_key.analysis_key)?;
    let decoded = serde_json::from_str::<ValkeySearchQueryCachePayload>(payload).ok()?;
    if decoded.schema != SEARCH_QUERY_CACHE_SCHEMA_VERSION {
        return None;
    }
    if decoded.repo_id != cache_key.analysis_key.repo_id
        || decoded.revision != revision
        || decoded.endpoint != cache_key.endpoint
        || decoded.query != cache_key.query
        || decoded.filter != cache_key.filter
        || decoded.max_distance != cache_key.max_distance
        || decoded.prefix_length != cache_key.prefix_length
        || decoded.transposition != cache_key.transposition
        || decoded.limit != cache_key.limit
    {
        return None;
    }
    serde_json::from_value(decoded.value).ok()
}

pub(super) fn stable_revision(cache_key: &RepositoryAnalysisCacheKey) -> Option<&str> {
    cache_key
        .checkout_revision
        .as_deref()
        .or(cache_key.mirror_revision.as_deref())
        .or(cache_key.tracking_revision.as_deref())
}
