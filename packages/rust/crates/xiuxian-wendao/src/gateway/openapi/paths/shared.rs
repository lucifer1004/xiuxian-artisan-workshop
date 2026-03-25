use super::{
    analysis::{
        API_ANALYSIS_CODE_AST_AXUM_PATH, API_ANALYSIS_CODE_AST_OPENAPI_PATH,
        API_ANALYSIS_MARKDOWN_AXUM_PATH, API_ANALYSIS_MARKDOWN_OPENAPI_PATH,
    },
    docs::{
        API_DOCS_FAMILY_CLUSTER_AXUM_PATH, API_DOCS_FAMILY_CLUSTER_OPENAPI_PATH,
        API_DOCS_FAMILY_CONTEXT_AXUM_PATH, API_DOCS_FAMILY_CONTEXT_OPENAPI_PATH,
        API_DOCS_FAMILY_SEARCH_AXUM_PATH, API_DOCS_FAMILY_SEARCH_OPENAPI_PATH,
        API_DOCS_NAVIGATION_AXUM_PATH, API_DOCS_NAVIGATION_OPENAPI_PATH,
        API_DOCS_NAVIGATION_SEARCH_AXUM_PATH, API_DOCS_NAVIGATION_SEARCH_OPENAPI_PATH,
        API_DOCS_PAGE_AXUM_PATH, API_DOCS_PAGE_OPENAPI_PATH, API_DOCS_PLANNER_ITEM_AXUM_PATH,
        API_DOCS_PLANNER_ITEM_OPENAPI_PATH, API_DOCS_PLANNER_QUEUE_AXUM_PATH,
        API_DOCS_PLANNER_QUEUE_OPENAPI_PATH, API_DOCS_PLANNER_RANK_AXUM_PATH,
        API_DOCS_PLANNER_RANK_OPENAPI_PATH, API_DOCS_PLANNER_SEARCH_AXUM_PATH,
        API_DOCS_PLANNER_SEARCH_OPENAPI_PATH, API_DOCS_PLANNER_WORKSET_AXUM_PATH,
        API_DOCS_PLANNER_WORKSET_OPENAPI_PATH, API_DOCS_PROJECTED_GAP_REPORT_AXUM_PATH,
        API_DOCS_PROJECTED_GAP_REPORT_OPENAPI_PATH, API_DOCS_RETRIEVAL_AXUM_PATH,
        API_DOCS_RETRIEVAL_CONTEXT_AXUM_PATH, API_DOCS_RETRIEVAL_CONTEXT_OPENAPI_PATH,
        API_DOCS_RETRIEVAL_HIT_AXUM_PATH, API_DOCS_RETRIEVAL_HIT_OPENAPI_PATH,
        API_DOCS_RETRIEVAL_OPENAPI_PATH, API_DOCS_SEARCH_AXUM_PATH, API_DOCS_SEARCH_OPENAPI_PATH,
    },
    graph::{
        API_GRAPH_NEIGHBORS_AXUM_PATH, API_GRAPH_NEIGHBORS_OPENAPI_PATH, API_NEIGHBORS_AXUM_PATH,
        API_NEIGHBORS_OPENAPI_PATH, API_TOPOLOGY_3D_AXUM_PATH, API_TOPOLOGY_3D_OPENAPI_PATH,
    },
    repo::{
        API_REPO_DOC_COVERAGE_AXUM_PATH, API_REPO_DOC_COVERAGE_OPENAPI_PATH,
        API_REPO_EXAMPLE_SEARCH_AXUM_PATH, API_REPO_EXAMPLE_SEARCH_OPENAPI_PATH,
        API_REPO_INDEX_AXUM_PATH, API_REPO_INDEX_OPENAPI_PATH, API_REPO_INDEX_STATUS_AXUM_PATH,
        API_REPO_INDEX_STATUS_OPENAPI_PATH, API_REPO_MODULE_SEARCH_AXUM_PATH,
        API_REPO_MODULE_SEARCH_OPENAPI_PATH, API_REPO_OVERVIEW_AXUM_PATH,
        API_REPO_OVERVIEW_OPENAPI_PATH, API_REPO_PROJECTED_GAP_REPORT_AXUM_PATH,
        API_REPO_PROJECTED_GAP_REPORT_OPENAPI_PATH, API_REPO_PROJECTED_PAGE_AXUM_PATH,
        API_REPO_PROJECTED_PAGE_FAMILY_CLUSTER_AXUM_PATH,
        API_REPO_PROJECTED_PAGE_FAMILY_CLUSTER_OPENAPI_PATH,
        API_REPO_PROJECTED_PAGE_FAMILY_CONTEXT_AXUM_PATH,
        API_REPO_PROJECTED_PAGE_FAMILY_CONTEXT_OPENAPI_PATH,
        API_REPO_PROJECTED_PAGE_FAMILY_SEARCH_AXUM_PATH,
        API_REPO_PROJECTED_PAGE_FAMILY_SEARCH_OPENAPI_PATH,
        API_REPO_PROJECTED_PAGE_INDEX_NODE_AXUM_PATH,
        API_REPO_PROJECTED_PAGE_INDEX_NODE_OPENAPI_PATH,
        API_REPO_PROJECTED_PAGE_INDEX_TREE_AXUM_PATH,
        API_REPO_PROJECTED_PAGE_INDEX_TREE_OPENAPI_PATH,
        API_REPO_PROJECTED_PAGE_INDEX_TREE_SEARCH_AXUM_PATH,
        API_REPO_PROJECTED_PAGE_INDEX_TREE_SEARCH_OPENAPI_PATH,
        API_REPO_PROJECTED_PAGE_INDEX_TREES_AXUM_PATH,
        API_REPO_PROJECTED_PAGE_INDEX_TREES_OPENAPI_PATH,
        API_REPO_PROJECTED_PAGE_NAVIGATION_AXUM_PATH,
        API_REPO_PROJECTED_PAGE_NAVIGATION_OPENAPI_PATH,
        API_REPO_PROJECTED_PAGE_NAVIGATION_SEARCH_AXUM_PATH,
        API_REPO_PROJECTED_PAGE_NAVIGATION_SEARCH_OPENAPI_PATH,
        API_REPO_PROJECTED_PAGE_OPENAPI_PATH, API_REPO_PROJECTED_PAGE_SEARCH_AXUM_PATH,
        API_REPO_PROJECTED_PAGE_SEARCH_OPENAPI_PATH, API_REPO_PROJECTED_PAGES_AXUM_PATH,
        API_REPO_PROJECTED_PAGES_OPENAPI_PATH, API_REPO_PROJECTED_RETRIEVAL_AXUM_PATH,
        API_REPO_PROJECTED_RETRIEVAL_CONTEXT_AXUM_PATH,
        API_REPO_PROJECTED_RETRIEVAL_CONTEXT_OPENAPI_PATH,
        API_REPO_PROJECTED_RETRIEVAL_HIT_AXUM_PATH, API_REPO_PROJECTED_RETRIEVAL_HIT_OPENAPI_PATH,
        API_REPO_PROJECTED_RETRIEVAL_OPENAPI_PATH, API_REPO_SYMBOL_SEARCH_AXUM_PATH,
        API_REPO_SYMBOL_SEARCH_OPENAPI_PATH, API_REPO_SYNC_AXUM_PATH, API_REPO_SYNC_OPENAPI_PATH,
    },
    search::{
        API_SEARCH_AST_AXUM_PATH, API_SEARCH_AST_OPENAPI_PATH, API_SEARCH_ATTACHMENTS_AXUM_PATH,
        API_SEARCH_ATTACHMENTS_OPENAPI_PATH, API_SEARCH_AUTOCOMPLETE_AXUM_PATH,
        API_SEARCH_AUTOCOMPLETE_OPENAPI_PATH, API_SEARCH_AXUM_PATH,
        API_SEARCH_DEFINITION_AXUM_PATH, API_SEARCH_DEFINITION_OPENAPI_PATH,
        API_SEARCH_INDEX_STATUS_AXUM_PATH, API_SEARCH_INDEX_STATUS_OPENAPI_PATH,
        API_SEARCH_INTENT_AXUM_PATH, API_SEARCH_INTENT_OPENAPI_PATH, API_SEARCH_OPENAPI_PATH,
        API_SEARCH_REFERENCES_AXUM_PATH, API_SEARCH_REFERENCES_OPENAPI_PATH,
        API_SEARCH_SYMBOLS_AXUM_PATH, API_SEARCH_SYMBOLS_OPENAPI_PATH,
    },
    ui::{
        API_UI_CAPABILITIES_AXUM_PATH, API_UI_CAPABILITIES_OPENAPI_PATH, API_UI_CONFIG_AXUM_PATH,
        API_UI_CONFIG_OPENAPI_PATH,
    },
    vfs::{
        API_VFS_CAT_AXUM_PATH, API_VFS_CAT_OPENAPI_PATH, API_VFS_ENTRY_AXUM_PATH,
        API_VFS_ENTRY_OPENAPI_PATH, API_VFS_ROOT_AXUM_PATH, API_VFS_ROOT_OPENAPI_PATH,
        API_VFS_SCAN_AXUM_PATH, API_VFS_SCAN_OPENAPI_PATH,
    },
};

/// One declared route contract in the Wendao gateway surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteContract {
    /// The Axum runtime path pattern.
    pub axum_path: &'static str,
    /// The normalized `OpenAPI` path pattern.
    pub openapi_path: &'static str,
    /// Supported lowercase HTTP methods.
    pub methods: &'static [&'static str],
    /// Required `OpenAPI` path parameter names for this route.
    pub path_params: &'static [&'static str],
}

/// Axum runtime path for the health endpoint.
pub const API_HEALTH_AXUM_PATH: &str = "/api/health";
/// `OpenAPI` path for the health endpoint.
pub const API_HEALTH_OPENAPI_PATH: &str = "/api/health";
/// Axum runtime path for the stats endpoint.
pub const API_STATS_AXUM_PATH: &str = "/api/stats";
/// `OpenAPI` path for the stats endpoint.
pub const API_STATS_OPENAPI_PATH: &str = "/api/stats";
/// Axum runtime path for the notify endpoint.
pub const API_NOTIFY_AXUM_PATH: &str = "/api/notify";
/// `OpenAPI` path for the notify endpoint.
pub const API_NOTIFY_OPENAPI_PATH: &str = "/api/notify";

/// Stable inventory for the current Wendao gateway route surface.
pub const WENDAO_GATEWAY_ROUTE_CONTRACTS: &[RouteContract] = &[
    RouteContract {
        axum_path: API_HEALTH_AXUM_PATH,
        openapi_path: API_HEALTH_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_STATS_AXUM_PATH,
        openapi_path: API_STATS_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_NOTIFY_AXUM_PATH,
        openapi_path: API_NOTIFY_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_VFS_ROOT_AXUM_PATH,
        openapi_path: API_VFS_ROOT_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_VFS_SCAN_AXUM_PATH,
        openapi_path: API_VFS_SCAN_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_VFS_CAT_AXUM_PATH,
        openapi_path: API_VFS_CAT_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_VFS_ENTRY_AXUM_PATH,
        openapi_path: API_VFS_ENTRY_OPENAPI_PATH,
        methods: &["get"],
        path_params: &["path"],
    },
    RouteContract {
        axum_path: API_NEIGHBORS_AXUM_PATH,
        openapi_path: API_NEIGHBORS_OPENAPI_PATH,
        methods: &["get"],
        path_params: &["id"],
    },
    RouteContract {
        axum_path: API_GRAPH_NEIGHBORS_AXUM_PATH,
        openapi_path: API_GRAPH_NEIGHBORS_OPENAPI_PATH,
        methods: &["get"],
        path_params: &["id"],
    },
    RouteContract {
        axum_path: API_TOPOLOGY_3D_AXUM_PATH,
        openapi_path: API_TOPOLOGY_3D_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_SEARCH_AXUM_PATH,
        openapi_path: API_SEARCH_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_SEARCH_INTENT_AXUM_PATH,
        openapi_path: API_SEARCH_INTENT_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_SEARCH_ATTACHMENTS_AXUM_PATH,
        openapi_path: API_SEARCH_ATTACHMENTS_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_SEARCH_AST_AXUM_PATH,
        openapi_path: API_SEARCH_AST_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_SEARCH_DEFINITION_AXUM_PATH,
        openapi_path: API_SEARCH_DEFINITION_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_SEARCH_REFERENCES_AXUM_PATH,
        openapi_path: API_SEARCH_REFERENCES_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_SEARCH_SYMBOLS_AXUM_PATH,
        openapi_path: API_SEARCH_SYMBOLS_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_SEARCH_AUTOCOMPLETE_AXUM_PATH,
        openapi_path: API_SEARCH_AUTOCOMPLETE_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_SEARCH_INDEX_STATUS_AXUM_PATH,
        openapi_path: API_SEARCH_INDEX_STATUS_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_ANALYSIS_MARKDOWN_AXUM_PATH,
        openapi_path: API_ANALYSIS_MARKDOWN_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_ANALYSIS_CODE_AST_AXUM_PATH,
        openapi_path: API_ANALYSIS_CODE_AST_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_DOCS_PROJECTED_GAP_REPORT_AXUM_PATH,
        openapi_path: API_DOCS_PROJECTED_GAP_REPORT_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_DOCS_PLANNER_ITEM_AXUM_PATH,
        openapi_path: API_DOCS_PLANNER_ITEM_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_DOCS_PLANNER_SEARCH_AXUM_PATH,
        openapi_path: API_DOCS_PLANNER_SEARCH_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_DOCS_PLANNER_QUEUE_AXUM_PATH,
        openapi_path: API_DOCS_PLANNER_QUEUE_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_DOCS_PLANNER_RANK_AXUM_PATH,
        openapi_path: API_DOCS_PLANNER_RANK_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_DOCS_PLANNER_WORKSET_AXUM_PATH,
        openapi_path: API_DOCS_PLANNER_WORKSET_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_DOCS_SEARCH_AXUM_PATH,
        openapi_path: API_DOCS_SEARCH_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_DOCS_RETRIEVAL_AXUM_PATH,
        openapi_path: API_DOCS_RETRIEVAL_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_DOCS_RETRIEVAL_CONTEXT_AXUM_PATH,
        openapi_path: API_DOCS_RETRIEVAL_CONTEXT_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_DOCS_RETRIEVAL_HIT_AXUM_PATH,
        openapi_path: API_DOCS_RETRIEVAL_HIT_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_DOCS_PAGE_AXUM_PATH,
        openapi_path: API_DOCS_PAGE_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_DOCS_FAMILY_CONTEXT_AXUM_PATH,
        openapi_path: API_DOCS_FAMILY_CONTEXT_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_DOCS_FAMILY_SEARCH_AXUM_PATH,
        openapi_path: API_DOCS_FAMILY_SEARCH_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_DOCS_FAMILY_CLUSTER_AXUM_PATH,
        openapi_path: API_DOCS_FAMILY_CLUSTER_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_DOCS_NAVIGATION_AXUM_PATH,
        openapi_path: API_DOCS_NAVIGATION_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_DOCS_NAVIGATION_SEARCH_AXUM_PATH,
        openapi_path: API_DOCS_NAVIGATION_SEARCH_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_UI_CONFIG_AXUM_PATH,
        openapi_path: API_UI_CONFIG_OPENAPI_PATH,
        methods: &["get", "post"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_UI_CAPABILITIES_AXUM_PATH,
        openapi_path: API_UI_CAPABILITIES_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_OVERVIEW_AXUM_PATH,
        openapi_path: API_REPO_OVERVIEW_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_MODULE_SEARCH_AXUM_PATH,
        openapi_path: API_REPO_MODULE_SEARCH_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_SYMBOL_SEARCH_AXUM_PATH,
        openapi_path: API_REPO_SYMBOL_SEARCH_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_EXAMPLE_SEARCH_AXUM_PATH,
        openapi_path: API_REPO_EXAMPLE_SEARCH_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_DOC_COVERAGE_AXUM_PATH,
        openapi_path: API_REPO_DOC_COVERAGE_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_INDEX_STATUS_AXUM_PATH,
        openapi_path: API_REPO_INDEX_STATUS_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_INDEX_AXUM_PATH,
        openapi_path: API_REPO_INDEX_OPENAPI_PATH,
        methods: &["post"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_SYNC_AXUM_PATH,
        openapi_path: API_REPO_SYNC_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_PROJECTED_PAGES_AXUM_PATH,
        openapi_path: API_REPO_PROJECTED_PAGES_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_PROJECTED_GAP_REPORT_AXUM_PATH,
        openapi_path: API_REPO_PROJECTED_GAP_REPORT_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_PROJECTED_PAGE_AXUM_PATH,
        openapi_path: API_REPO_PROJECTED_PAGE_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_PROJECTED_PAGE_INDEX_NODE_AXUM_PATH,
        openapi_path: API_REPO_PROJECTED_PAGE_INDEX_NODE_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_PROJECTED_RETRIEVAL_HIT_AXUM_PATH,
        openapi_path: API_REPO_PROJECTED_RETRIEVAL_HIT_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_PROJECTED_RETRIEVAL_CONTEXT_AXUM_PATH,
        openapi_path: API_REPO_PROJECTED_RETRIEVAL_CONTEXT_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_PROJECTED_PAGE_FAMILY_CONTEXT_AXUM_PATH,
        openapi_path: API_REPO_PROJECTED_PAGE_FAMILY_CONTEXT_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_PROJECTED_PAGE_FAMILY_SEARCH_AXUM_PATH,
        openapi_path: API_REPO_PROJECTED_PAGE_FAMILY_SEARCH_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_PROJECTED_PAGE_FAMILY_CLUSTER_AXUM_PATH,
        openapi_path: API_REPO_PROJECTED_PAGE_FAMILY_CLUSTER_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_PROJECTED_PAGE_NAVIGATION_AXUM_PATH,
        openapi_path: API_REPO_PROJECTED_PAGE_NAVIGATION_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_PROJECTED_PAGE_NAVIGATION_SEARCH_AXUM_PATH,
        openapi_path: API_REPO_PROJECTED_PAGE_NAVIGATION_SEARCH_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_PROJECTED_PAGE_INDEX_TREE_AXUM_PATH,
        openapi_path: API_REPO_PROJECTED_PAGE_INDEX_TREE_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_PROJECTED_PAGE_INDEX_TREE_SEARCH_AXUM_PATH,
        openapi_path: API_REPO_PROJECTED_PAGE_INDEX_TREE_SEARCH_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_PROJECTED_PAGE_SEARCH_AXUM_PATH,
        openapi_path: API_REPO_PROJECTED_PAGE_SEARCH_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_PROJECTED_RETRIEVAL_AXUM_PATH,
        openapi_path: API_REPO_PROJECTED_RETRIEVAL_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
    RouteContract {
        axum_path: API_REPO_PROJECTED_PAGE_INDEX_TREES_AXUM_PATH,
        openapi_path: API_REPO_PROJECTED_PAGE_INDEX_TREES_OPENAPI_PATH,
        methods: &["get"],
        path_params: &[],
    },
];
