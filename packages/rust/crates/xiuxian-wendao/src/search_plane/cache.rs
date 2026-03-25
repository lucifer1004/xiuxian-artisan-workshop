use std::collections::BTreeMap;
#[cfg(test)]
use std::sync::{Arc, RwLock};
use std::time::Duration;

use redis::{AsyncCommands, AsyncConnectionConfig};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::{
    SearchCorpusKind, SearchFileFingerprint, SearchManifestKeyspace, SearchRepoCorpusRecord,
    SearchRepoCorpusSnapshotRecord,
};
use crate::valkey_common::resolve_optional_client_from_env;

const SEARCH_PLANE_VALKEY_URL_ENV: &str = "XIUXIAN_WENDAO_SEARCH_PLANE_VALKEY_URL";
const KNOWLEDGE_VALKEY_URL_ENV: &str = "XIUXIAN_WENDAO_KNOWLEDGE_VALKEY_URL";
const VALKEY_URL_ENV: &str = "VALKEY_URL";
const REDIS_URL_ENV: &str = "REDIS_URL";
const QUERY_CACHE_TTL_ENV: &str = "XIUXIAN_WENDAO_SEARCH_PLANE_QUERY_CACHE_TTL_SEC";
const AUTOCOMPLETE_CACHE_TTL_ENV: &str = "XIUXIAN_WENDAO_SEARCH_PLANE_AUTOCOMPLETE_CACHE_TTL_SEC";
const CACHE_CONNECTION_TIMEOUT_MS_ENV: &str =
    "XIUXIAN_WENDAO_SEARCH_PLANE_CACHE_CONNECTION_TIMEOUT_MS";
const CACHE_RESPONSE_TIMEOUT_MS_ENV: &str = "XIUXIAN_WENDAO_SEARCH_PLANE_CACHE_RESPONSE_TIMEOUT_MS";

const DEFAULT_QUERY_CACHE_TTL_SEC: u64 = 90;
const DEFAULT_AUTOCOMPLETE_CACHE_TTL_SEC: u64 = 300;
const DEFAULT_CACHE_CONNECTION_TIMEOUT_MS: u64 = 25;
const DEFAULT_CACHE_RESPONSE_TIMEOUT_MS: u64 = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SearchPlaneCacheTtl {
    HotQuery,
    Autocomplete,
}

impl SearchPlaneCacheTtl {
    fn as_seconds(self, config: &SearchPlaneCacheConfig) -> u64 {
        match self {
            Self::HotQuery => config.query_ttl_seconds,
            Self::Autocomplete => config.autocomplete_ttl_seconds,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchPlaneCacheConfig {
    query_ttl_seconds: u64,
    autocomplete_ttl_seconds: u64,
    connection_timeout: Duration,
    response_timeout: Duration,
}

impl Default for SearchPlaneCacheConfig {
    fn default() -> Self {
        Self {
            query_ttl_seconds: DEFAULT_QUERY_CACHE_TTL_SEC,
            autocomplete_ttl_seconds: DEFAULT_AUTOCOMPLETE_CACHE_TTL_SEC,
            connection_timeout: Duration::from_millis(DEFAULT_CACHE_CONNECTION_TIMEOUT_MS),
            response_timeout: Duration::from_millis(DEFAULT_CACHE_RESPONSE_TIMEOUT_MS),
        }
    }
}

impl SearchPlaneCacheConfig {
    fn from_env() -> Self {
        Self {
            query_ttl_seconds: parse_env_u64(QUERY_CACHE_TTL_ENV)
                .unwrap_or(DEFAULT_QUERY_CACHE_TTL_SEC),
            autocomplete_ttl_seconds: parse_env_u64(AUTOCOMPLETE_CACHE_TTL_ENV)
                .unwrap_or(DEFAULT_AUTOCOMPLETE_CACHE_TTL_SEC),
            connection_timeout: Duration::from_millis(
                parse_env_u64(CACHE_CONNECTION_TIMEOUT_MS_ENV)
                    .unwrap_or(DEFAULT_CACHE_CONNECTION_TIMEOUT_MS),
            ),
            response_timeout: Duration::from_millis(
                parse_env_u64(CACHE_RESPONSE_TIMEOUT_MS_ENV)
                    .unwrap_or(DEFAULT_CACHE_RESPONSE_TIMEOUT_MS),
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SearchPlaneCache {
    client: Option<redis::Client>,
    config: SearchPlaneCacheConfig,
    keyspace: SearchManifestKeyspace,
    #[cfg(test)]
    shadow: Arc<RwLock<TestCacheShadow>>,
}

#[cfg(test)]
#[derive(Debug, Default)]
struct TestCacheShadow {
    repo_corpus_records: BTreeMap<(SearchCorpusKind, String), SearchRepoCorpusRecord>,
    repo_corpus_snapshot: Option<SearchRepoCorpusSnapshotRecord>,
    corpus_file_fingerprints: BTreeMap<SearchCorpusKind, BTreeMap<String, SearchFileFingerprint>>,
    repo_corpus_file_fingerprints:
        BTreeMap<(SearchCorpusKind, String), BTreeMap<String, SearchFileFingerprint>>,
}

impl SearchPlaneCache {
    pub(crate) fn from_env(keyspace: SearchManifestKeyspace) -> Self {
        Self::new(
            resolve_valkey_client(),
            SearchPlaneCacheConfig::from_env(),
            keyspace,
        )
    }

    pub(crate) fn disabled(keyspace: SearchManifestKeyspace) -> Self {
        Self::new(None, SearchPlaneCacheConfig::default(), keyspace)
    }

    #[cfg(test)]
    pub(crate) fn for_tests(keyspace: SearchManifestKeyspace) -> Self {
        Self::new(
            Some(
                redis::Client::open("redis://127.0.0.1/")
                    .unwrap_or_else(|error| panic!("client: {error}")),
            ),
            SearchPlaneCacheConfig::default(),
            keyspace,
        )
    }

    #[cfg(test)]
    pub(crate) fn clear_repo_shadow_for_tests(&self, repo_id: &str) {
        let mut shadow = self
            .shadow
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        shadow
            .repo_corpus_records
            .retain(|(_, candidate_repo_id), _| candidate_repo_id != repo_id);
        if let Some(snapshot) = shadow.repo_corpus_snapshot.as_mut() {
            snapshot.records.retain(|record| record.repo_id != repo_id);
            if snapshot.records.is_empty() {
                shadow.repo_corpus_snapshot = None;
            }
        }
        shadow
            .repo_corpus_file_fingerprints
            .retain(|(_, candidate_repo_id), _| candidate_repo_id != repo_id);
    }

    fn new(
        client: Option<redis::Client>,
        config: SearchPlaneCacheConfig,
        keyspace: SearchManifestKeyspace,
    ) -> Self {
        Self {
            client,
            config,
            keyspace,
            #[cfg(test)]
            shadow: Arc::new(RwLock::new(TestCacheShadow::default())),
        }
    }

    pub(crate) fn autocomplete_cache_key(
        &self,
        prefix: &str,
        limit: usize,
        active_epoch: u64,
    ) -> Option<String> {
        self.client.as_ref()?;
        let token = hashed_cache_token(
            "autocomplete",
            [
                format!("epoch:{active_epoch}"),
                format!("limit:{limit}"),
                format!("prefix:{}", normalize_cache_text(prefix)),
            ],
        );
        Some(self.keyspace.autocomplete_cache_key(token.as_str()))
    }

    pub(crate) fn search_query_cache_key(
        &self,
        scope: &str,
        epochs: &[(SearchCorpusKind, u64)],
        query: &str,
        limit: usize,
        intent: Option<&str>,
        repo_hint: Option<&str>,
    ) -> Option<String> {
        let versions = epochs
            .iter()
            .map(|(corpus, epoch)| format!("{corpus}:{epoch}"))
            .collect::<Vec<_>>();
        self.search_query_cache_key_from_versions(
            scope,
            versions.as_slice(),
            query,
            limit,
            intent,
            repo_hint,
        )
    }

    pub(crate) fn search_query_cache_key_from_versions(
        &self,
        scope: &str,
        versions: &[String],
        query: &str,
        limit: usize,
        intent: Option<&str>,
        repo_hint: Option<&str>,
    ) -> Option<String> {
        self.client.as_ref()?;
        let mut components = Vec::with_capacity(4 + versions.len());
        let mut normalized_versions = versions.to_vec();
        normalized_versions.sort_unstable();
        normalized_versions.dedup();
        components.extend(normalized_versions);
        components.push(format!("limit:{limit}"));
        components.push(format!("query:{}", normalize_cache_text(query)));
        components.push(format!(
            "intent:{}",
            normalize_cache_text(intent.unwrap_or_default())
        ));
        components.push(format!(
            "repo:{}",
            normalize_cache_text(repo_hint.unwrap_or_default())
        ));
        let token = hashed_cache_token(scope, components);
        Some(self.keyspace.search_query_cache_key(scope, token.as_str()))
    }

    pub(crate) async fn get_json<T>(&self, key: &str) -> Option<T>
    where
        T: DeserializeOwned,
    {
        let client = self.client.as_ref()?;
        let mut connection = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
            .ok()?;
        let payload: Option<String> = connection.get(key).await.ok()?;
        serde_json::from_str(payload?.as_str()).ok()
    }

    pub(crate) async fn get_repo_corpus_record(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> Option<SearchRepoCorpusRecord> {
        #[cfg(test)]
        if let Some(record) = self
            .shadow
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_corpus_records
            .get(&(corpus, repo_id.to_string()))
            .cloned()
        {
            return Some(record);
        }
        let key = self.keyspace.repo_corpus_record_key(corpus, repo_id);
        self.get_json(key.as_str()).await
    }

    pub(crate) async fn get_repo_corpus_snapshot(&self) -> Option<SearchRepoCorpusSnapshotRecord> {
        #[cfg(test)]
        if let Some(record) = self
            .shadow
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_corpus_snapshot
            .clone()
        {
            return Some(record);
        }
        let key = self.keyspace.repo_corpus_snapshot_key();
        self.get_json(key.as_str()).await
    }

    pub(crate) async fn get_corpus_file_fingerprints(
        &self,
        corpus: SearchCorpusKind,
    ) -> Option<BTreeMap<String, SearchFileFingerprint>> {
        #[cfg(test)]
        if let Some(fingerprints) = self
            .shadow
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .corpus_file_fingerprints
            .get(&corpus)
            .cloned()
        {
            return Some(fingerprints);
        }
        let key = self.keyspace.corpus_file_fingerprints_key(corpus);
        self.get_json(key.as_str()).await
    }

    pub(crate) async fn get_repo_corpus_file_fingerprints(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) -> Option<BTreeMap<String, SearchFileFingerprint>> {
        #[cfg(test)]
        if let Some(fingerprints) = self
            .shadow
            .read()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_corpus_file_fingerprints
            .get(&(corpus, repo_id.to_string()))
            .cloned()
        {
            return Some(fingerprints);
        }
        let key = self
            .keyspace
            .repo_corpus_file_fingerprints_key(corpus, repo_id);
        self.get_json(key.as_str()).await
    }

    pub(crate) async fn set_json<T>(&self, key: &str, ttl: SearchPlaneCacheTtl, value: &T)
    where
        T: Serialize,
    {
        let Some(client) = self.client.as_ref() else {
            return;
        };
        let ttl_seconds = ttl.as_seconds(&self.config);
        if ttl_seconds == 0 {
            return;
        }
        let Ok(payload) = serde_json::to_string(value) else {
            return;
        };
        let Ok(mut connection) = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
        else {
            return;
        };
        let _: redis::RedisResult<()> = connection.set_ex(key, payload, ttl_seconds).await;
    }

    pub(crate) async fn set_repo_corpus_record(&self, record: &SearchRepoCorpusRecord) {
        #[cfg(test)]
        self.shadow
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_corpus_records
            .insert((record.corpus, record.repo_id.clone()), record.clone());
        let Some(client) = self.client.as_ref() else {
            return;
        };
        let Ok(payload) = serde_json::to_string(record) else {
            return;
        };
        let key = self
            .keyspace
            .repo_corpus_record_key(record.corpus, record.repo_id.as_str());
        let Ok(mut connection) = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
        else {
            return;
        };
        let _: redis::RedisResult<()> = connection.set(key, payload).await;
    }

    pub(crate) async fn set_repo_corpus_snapshot(&self, record: &SearchRepoCorpusSnapshotRecord) {
        #[cfg(test)]
        {
            self.shadow
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .repo_corpus_snapshot = Some(record.clone());
        }
        let Some(client) = self.client.as_ref() else {
            return;
        };
        let Ok(payload) = serde_json::to_string(record) else {
            return;
        };
        let key = self.keyspace.repo_corpus_snapshot_key();
        let Ok(mut connection) = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
        else {
            return;
        };
        let _: redis::RedisResult<()> = connection.set(key, payload).await;
    }

    pub(crate) async fn set_corpus_file_fingerprints(
        &self,
        corpus: SearchCorpusKind,
        fingerprints: &BTreeMap<String, SearchFileFingerprint>,
    ) {
        #[cfg(test)]
        {
            self.shadow
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .corpus_file_fingerprints
                .insert(corpus, fingerprints.clone());
        }
        let Some(client) = self.client.as_ref() else {
            return;
        };
        let Ok(payload) = serde_json::to_string(fingerprints) else {
            return;
        };
        let key = self.keyspace.corpus_file_fingerprints_key(corpus);
        let Ok(mut connection) = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
        else {
            return;
        };
        let _: redis::RedisResult<()> = connection.set(key, payload).await;
    }

    pub(crate) async fn set_repo_corpus_file_fingerprints(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
        fingerprints: &BTreeMap<String, SearchFileFingerprint>,
    ) {
        #[cfg(test)]
        {
            self.shadow
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .repo_corpus_file_fingerprints
                .insert((corpus, repo_id.to_string()), fingerprints.clone());
        }
        let Some(client) = self.client.as_ref() else {
            return;
        };
        let Ok(payload) = serde_json::to_string(fingerprints) else {
            return;
        };
        let key = self
            .keyspace
            .repo_corpus_file_fingerprints_key(corpus, repo_id);
        let Ok(mut connection) = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
        else {
            return;
        };
        let _: redis::RedisResult<()> = connection.set(key, payload).await;
    }

    pub(crate) async fn delete_repo_corpus_record(&self, corpus: SearchCorpusKind, repo_id: &str) {
        #[cfg(test)]
        self.shadow
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_corpus_records
            .remove(&(corpus, repo_id.to_string()));
        let Some(client) = self.client.as_ref() else {
            return;
        };
        let key = self.keyspace.repo_corpus_record_key(corpus, repo_id);
        let Ok(mut connection) = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
        else {
            return;
        };
        let _: redis::RedisResult<()> = connection.del(key).await;
    }

    pub(crate) async fn delete_repo_corpus_file_fingerprints(
        &self,
        corpus: SearchCorpusKind,
        repo_id: &str,
    ) {
        #[cfg(test)]
        self.shadow
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .repo_corpus_file_fingerprints
            .remove(&(corpus, repo_id.to_string()));
        let Some(client) = self.client.as_ref() else {
            return;
        };
        let key = self
            .keyspace
            .repo_corpus_file_fingerprints_key(corpus, repo_id);
        let Ok(mut connection) = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
        else {
            return;
        };
        let _: redis::RedisResult<()> = connection.del(key).await;
    }

    pub(crate) async fn delete_repo_corpus_snapshot(&self) {
        #[cfg(test)]
        {
            self.shadow
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .repo_corpus_snapshot = None;
        }
        let Some(client) = self.client.as_ref() else {
            return;
        };
        let key = self.keyspace.repo_corpus_snapshot_key();
        let Ok(mut connection) = client
            .get_multiplexed_async_connection_with_config(&self.async_connection_config())
            .await
        else {
            return;
        };
        let _: redis::RedisResult<()> = connection.del(key).await;
    }

    fn async_connection_config(&self) -> AsyncConnectionConfig {
        AsyncConnectionConfig::new()
            .set_connection_timeout(Some(self.config.connection_timeout))
            .set_response_timeout(Some(self.config.response_timeout))
    }
}

fn normalize_cache_text(input: &str) -> String {
    input
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn hashed_cache_token<I>(scope: &str, components: I) -> String
where
    I: IntoIterator<Item = String>,
{
    let mut payload = String::from(scope);
    for component in components {
        payload.push('|');
        payload.push_str(component.as_str());
    }
    blake3::hash(payload.as_bytes()).to_hex().to_string()
}

fn resolve_valkey_client() -> Option<redis::Client> {
    resolve_optional_client_from_env(&[
        SEARCH_PLANE_VALKEY_URL_ENV,
        KNOWLEDGE_VALKEY_URL_ENV,
        VALKEY_URL_ENV,
        REDIS_URL_ENV,
    ])
}

fn parse_env_u64(name: &str) -> Option<u64> {
    std::env::var(name)
        .ok()
        .and_then(|value| value.trim().parse::<u64>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn required_cache_key(key: Option<String>, context: &str) -> String {
        key.unwrap_or_else(|| panic!("{context}"))
    }

    fn cache_for_tests() -> SearchPlaneCache {
        SearchPlaneCache::for_tests(SearchManifestKeyspace::new("xiuxian:test:search_plane"))
    }

    #[test]
    fn autocomplete_key_is_stable_for_epoch_prefix_and_limit() {
        let cache = cache_for_tests();
        let key = required_cache_key(
            cache.autocomplete_cache_key(" Alpha Handler ", 8, 7),
            "autocomplete key",
        );
        assert_eq!(
            key,
            required_cache_key(
                cache.autocomplete_cache_key("alpha    handler", 8, 7),
                "stable autocomplete key",
            )
        );
        assert_ne!(
            key,
            required_cache_key(
                cache.autocomplete_cache_key("alpha handler", 8, 8),
                "epoch-specific autocomplete key",
            )
        );
    }

    #[test]
    fn search_query_key_tracks_scope_epochs_and_query_shape() {
        let cache = cache_for_tests();
        let key = required_cache_key(
            cache.search_query_cache_key(
                "intent",
                &[
                    (SearchCorpusKind::KnowledgeSection, 3),
                    (SearchCorpusKind::LocalSymbol, 11),
                ],
                "  alpha_handler  ",
                10,
                Some("semantic_lookup"),
                None,
            ),
            "search query key",
        );
        assert_eq!(
            key,
            required_cache_key(
                cache.search_query_cache_key(
                    "intent",
                    &[
                        (SearchCorpusKind::KnowledgeSection, 3),
                        (SearchCorpusKind::LocalSymbol, 11),
                    ],
                    "alpha_handler",
                    10,
                    Some("semantic_lookup"),
                    None,
                ),
                "stable search query key",
            )
        );
        assert_ne!(
            key,
            required_cache_key(
                cache.search_query_cache_key(
                    "intent",
                    &[
                        (SearchCorpusKind::KnowledgeSection, 3),
                        (SearchCorpusKind::LocalSymbol, 12),
                    ],
                    "alpha_handler",
                    10,
                    Some("semantic_lookup"),
                    None,
                ),
                "epoch-specific search query key",
            )
        );
    }

    #[test]
    fn search_query_key_tracks_repo_versions_and_sorts_components() {
        let cache = cache_for_tests();
        let key = required_cache_key(
            cache.search_query_cache_key_from_versions(
                "intent_code",
                &[
                    "repo_entity:schema:1:repo:alpha:phase:ready:revision:abc:updated:2026-03-23t08:00:00z"
                        .to_string(),
                    "knowledge_section:schema:1:epoch:3".to_string(),
                    "repo_content_chunk:schema:1:repo:alpha:phase:ready:revision:abc:updated:2026-03-23t08:00:00z"
                        .to_string(),
                ],
                " lang:julia reexport ",
                10,
                Some("debug_lookup"),
                Some("alpha"),
            ),
            "repo search query key",
        );
        assert_eq!(
            key,
            required_cache_key(
                cache.search_query_cache_key_from_versions(
                    "intent_code",
                    &[
                        "repo_content_chunk:schema:1:repo:alpha:phase:ready:revision:abc:updated:2026-03-23t08:00:00z"
                            .to_string(),
                        "knowledge_section:schema:1:epoch:3".to_string(),
                        "repo_entity:schema:1:repo:alpha:phase:ready:revision:abc:updated:2026-03-23t08:00:00z"
                            .to_string(),
                    ],
                    "lang:julia   reexport",
                    10,
                    Some("debug_lookup"),
                    Some("alpha"),
                ),
                "stable repo search query key",
            )
        );
        assert_ne!(
            key,
            required_cache_key(
                cache.search_query_cache_key_from_versions(
                    "intent_code",
                    &[
                        "repo_entity:schema:1:repo:alpha:phase:ready:revision:def:updated:2026-03-23t09:00:00z"
                            .to_string(),
                        "knowledge_section:schema:1:epoch:3".to_string(),
                        "repo_content_chunk:schema:1:repo:alpha:phase:ready:revision:def:updated:2026-03-23t09:00:00z"
                            .to_string(),
                    ],
                    "lang:julia reexport",
                    10,
                    Some("debug_lookup"),
                    Some("alpha"),
                ),
                "repo-specific search query key",
            )
        );
    }

    #[test]
    fn disabled_cache_skips_key_generation() {
        let cache = SearchPlaneCache::disabled(SearchManifestKeyspace::new("xiuxian:test"));
        assert!(cache.autocomplete_cache_key("alpha", 8, 1).is_none());
        assert!(
            cache
                .search_query_cache_key(
                    "knowledge",
                    &[(SearchCorpusKind::KnowledgeSection, 1)],
                    "alpha",
                    10,
                    None,
                    None,
                )
                .is_none()
        );
    }
}
