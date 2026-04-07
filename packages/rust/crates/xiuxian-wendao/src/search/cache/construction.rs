use redis::AsyncConnectionConfig;

use crate::search::SearchManifestKeyspace;

use super::config::SearchPlaneCacheConfig;
use super::runtime::resolve_search_plane_cache_runtime;
#[cfg(test)]
use super::tests::TestCacheShadow;
use super::types::SearchPlaneCache;
#[cfg(test)]
use std::sync::{Arc, RwLock};

impl SearchPlaneCache {
    pub(crate) fn from_runtime(keyspace: SearchManifestKeyspace) -> Self {
        let runtime = resolve_search_plane_cache_runtime();
        Self::new(runtime.client, runtime.config, keyspace)
    }

    pub(crate) fn disabled(keyspace: SearchManifestKeyspace) -> Self {
        Self::new(None, SearchPlaneCacheConfig::default(), keyspace)
    }

    #[cfg(test)]
    pub(crate) fn for_tests(keyspace: SearchManifestKeyspace) -> Self {
        Self::for_tests_with_config(keyspace, SearchPlaneCacheConfig::default())
    }

    #[cfg(test)]
    pub(crate) fn for_tests_with_config(
        keyspace: SearchManifestKeyspace,
        config: SearchPlaneCacheConfig,
    ) -> Self {
        Self::new(
            Some(
                redis::Client::open("redis://127.0.0.1/")
                    .unwrap_or_else(|error| panic!("client: {error}")),
            ),
            config,
            keyspace,
        )
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

    pub(crate) fn async_connection_config(&self) -> AsyncConnectionConfig {
        AsyncConnectionConfig::new()
            .set_connection_timeout(Some(self.config.connection_timeout))
            .set_response_timeout(Some(self.config.response_timeout))
    }
}
