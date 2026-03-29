mod constants;
mod models;
mod resolve;
mod retrieval;

pub use constants::{
    DEFAULT_LINK_GRAPH_CANDIDATE_MULTIPLIER, DEFAULT_LINK_GRAPH_COACTIVATION_ALPHA_SCALE,
    DEFAULT_LINK_GRAPH_COACTIVATION_ENABLED, DEFAULT_LINK_GRAPH_COACTIVATION_HOP_DECAY_SCALE,
    DEFAULT_LINK_GRAPH_COACTIVATION_MAX_HOPS,
    DEFAULT_LINK_GRAPH_COACTIVATION_MAX_NEIGHBORS_PER_DIRECTION,
    DEFAULT_LINK_GRAPH_COACTIVATION_TOUCH_QUEUE_DEPTH, DEFAULT_LINK_GRAPH_HYBRID_MIN_HITS,
    DEFAULT_LINK_GRAPH_HYBRID_MIN_TOP_SCORE, DEFAULT_LINK_GRAPH_MAX_SOURCES,
    DEFAULT_LINK_GRAPH_RELATED_MAX_CANDIDATES, DEFAULT_LINK_GRAPH_RELATED_MAX_PARTITIONS,
    DEFAULT_LINK_GRAPH_RELATED_TIME_BUDGET_MS, DEFAULT_LINK_GRAPH_ROWS_PER_SOURCE,
    DEFAULT_LINK_GRAPH_VALKEY_KEY_PREFIX, LINK_GRAPH_CANDIDATE_MULTIPLIER_ENV,
    LINK_GRAPH_HYBRID_MIN_HITS_ENV, LINK_GRAPH_HYBRID_MIN_TOP_SCORE_ENV,
    LINK_GRAPH_MAX_SOURCES_ENV, LINK_GRAPH_ROWS_PER_SOURCE_ENV,
};
pub use models::{
    LinkGraphAgenticRuntimeConfig, LinkGraphCacheRuntimeConfig, LinkGraphCoactivationRuntimeConfig,
    LinkGraphIndexRuntimeConfig, LinkGraphRelatedRuntimeConfig,
};
pub use resolve::{
    resolve_link_graph_agentic_runtime_with_settings,
    resolve_link_graph_cache_runtime_with_settings,
    resolve_link_graph_coactivation_runtime_with_settings,
    resolve_link_graph_index_runtime_with_settings,
    resolve_link_graph_related_runtime_with_settings,
};
pub use retrieval::{
    LinkGraphRetrievalBaseRuntimeConfig, LinkGraphSemanticIgnitionBackend,
    LinkGraphSemanticIgnitionRuntimeConfig, apply_semantic_ignition_runtime_config,
    resolve_link_graph_retrieval_base_runtime_with_settings,
};
