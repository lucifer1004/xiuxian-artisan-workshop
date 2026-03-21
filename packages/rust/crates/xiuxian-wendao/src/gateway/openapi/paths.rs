//! Route inventory shared by the Wendao gateway runtime and `OpenAPI` contract tests.

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
/// Axum runtime path for the VFS root endpoint.
pub const API_VFS_ROOT_AXUM_PATH: &str = "/api/vfs";
/// `OpenAPI` path for the VFS root endpoint.
pub const API_VFS_ROOT_OPENAPI_PATH: &str = "/api/vfs";
/// Axum runtime path for the VFS scan endpoint.
pub const API_VFS_SCAN_AXUM_PATH: &str = "/api/vfs/scan";
/// `OpenAPI` path for the VFS scan endpoint.
pub const API_VFS_SCAN_OPENAPI_PATH: &str = "/api/vfs/scan";
/// Axum runtime path for the VFS cat endpoint.
pub const API_VFS_CAT_AXUM_PATH: &str = "/api/vfs/cat";
/// `OpenAPI` path for the VFS cat endpoint.
pub const API_VFS_CAT_OPENAPI_PATH: &str = "/api/vfs/cat";
/// Axum runtime path for the VFS wildcard entry endpoint.
pub const API_VFS_ENTRY_AXUM_PATH: &str = "/api/vfs/{*path}";
/// `OpenAPI` path for the VFS entry endpoint.
pub const API_VFS_ENTRY_OPENAPI_PATH: &str = "/api/vfs/{path}";
/// Axum runtime path for the legacy neighbors endpoint.
pub const API_NEIGHBORS_AXUM_PATH: &str = "/api/neighbors/{*id}";
/// `OpenAPI` path for the legacy neighbors endpoint.
pub const API_NEIGHBORS_OPENAPI_PATH: &str = "/api/neighbors/{id}";
/// Axum runtime path for the graph neighbors endpoint.
pub const API_GRAPH_NEIGHBORS_AXUM_PATH: &str = "/api/graph/neighbors/{*id}";
/// `OpenAPI` path for the graph neighbors endpoint.
pub const API_GRAPH_NEIGHBORS_OPENAPI_PATH: &str = "/api/graph/neighbors/{id}";
/// Axum runtime path for the 3D topology endpoint.
pub const API_TOPOLOGY_3D_AXUM_PATH: &str = "/api/topology/3d";
/// `OpenAPI` path for the 3D topology endpoint.
pub const API_TOPOLOGY_3D_OPENAPI_PATH: &str = "/api/topology/3d";
/// Axum runtime path for the search endpoint.
pub const API_SEARCH_AXUM_PATH: &str = "/api/search";
/// `OpenAPI` path for the search endpoint.
pub const API_SEARCH_OPENAPI_PATH: &str = "/api/search";
/// Axum runtime path for the intent-aware search endpoint.
pub const API_SEARCH_INTENT_AXUM_PATH: &str = "/api/search/intent";
/// `OpenAPI` path for the intent-aware search endpoint.
pub const API_SEARCH_INTENT_OPENAPI_PATH: &str = "/api/search/intent";
/// Axum runtime path for the attachment search endpoint.
pub const API_SEARCH_ATTACHMENTS_AXUM_PATH: &str = "/api/search/attachments";
/// `OpenAPI` path for the attachment search endpoint.
pub const API_SEARCH_ATTACHMENTS_OPENAPI_PATH: &str = "/api/search/attachments";
/// Axum runtime path for the AST search endpoint.
pub const API_SEARCH_AST_AXUM_PATH: &str = "/api/search/ast";
/// `OpenAPI` path for the AST search endpoint.
pub const API_SEARCH_AST_OPENAPI_PATH: &str = "/api/search/ast";
/// Axum runtime path for the definition search endpoint.
pub const API_SEARCH_DEFINITION_AXUM_PATH: &str = "/api/search/definition";
/// `OpenAPI` path for the definition search endpoint.
pub const API_SEARCH_DEFINITION_OPENAPI_PATH: &str = "/api/search/definition";
/// Axum runtime path for the references search endpoint.
pub const API_SEARCH_REFERENCES_AXUM_PATH: &str = "/api/search/references";
/// `OpenAPI` path for the references search endpoint.
pub const API_SEARCH_REFERENCES_OPENAPI_PATH: &str = "/api/search/references";
/// Axum runtime path for the symbols search endpoint.
pub const API_SEARCH_SYMBOLS_AXUM_PATH: &str = "/api/search/symbols";
/// `OpenAPI` path for the symbols search endpoint.
pub const API_SEARCH_SYMBOLS_OPENAPI_PATH: &str = "/api/search/symbols";
/// Axum runtime path for the autocomplete search endpoint.
pub const API_SEARCH_AUTOCOMPLETE_AXUM_PATH: &str = "/api/search/autocomplete";
/// `OpenAPI` path for the autocomplete search endpoint.
pub const API_SEARCH_AUTOCOMPLETE_OPENAPI_PATH: &str = "/api/search/autocomplete";
/// Axum runtime path for the markdown analysis endpoint.
pub const API_ANALYSIS_MARKDOWN_AXUM_PATH: &str = "/api/analysis/markdown";
/// `OpenAPI` path for the markdown analysis endpoint.
pub const API_ANALYSIS_MARKDOWN_OPENAPI_PATH: &str = "/api/analysis/markdown";
/// Axum runtime path for the UI config endpoint.
pub const API_UI_CONFIG_AXUM_PATH: &str = "/api/ui/config";
/// `OpenAPI` path for the UI config endpoint.
pub const API_UI_CONFIG_OPENAPI_PATH: &str = "/api/ui/config";
/// Axum runtime path for the repo overview endpoint.
pub const API_REPO_OVERVIEW_AXUM_PATH: &str = "/api/repo/overview";
/// `OpenAPI` path for the repo overview endpoint.
pub const API_REPO_OVERVIEW_OPENAPI_PATH: &str = "/api/repo/overview";
/// Axum runtime path for the repo module-search endpoint.
pub const API_REPO_MODULE_SEARCH_AXUM_PATH: &str = "/api/repo/module-search";
/// `OpenAPI` path for the repo module-search endpoint.
pub const API_REPO_MODULE_SEARCH_OPENAPI_PATH: &str = "/api/repo/module-search";
/// Axum runtime path for the repo symbol-search endpoint.
pub const API_REPO_SYMBOL_SEARCH_AXUM_PATH: &str = "/api/repo/symbol-search";
/// `OpenAPI` path for the repo symbol-search endpoint.
pub const API_REPO_SYMBOL_SEARCH_OPENAPI_PATH: &str = "/api/repo/symbol-search";
/// Axum runtime path for the repo example-search endpoint.
pub const API_REPO_EXAMPLE_SEARCH_AXUM_PATH: &str = "/api/repo/example-search";
/// `OpenAPI` path for the repo example-search endpoint.
pub const API_REPO_EXAMPLE_SEARCH_OPENAPI_PATH: &str = "/api/repo/example-search";
/// Axum runtime path for the repo doc-coverage endpoint.
pub const API_REPO_DOC_COVERAGE_AXUM_PATH: &str = "/api/repo/doc-coverage";
/// `OpenAPI` path for the repo doc-coverage endpoint.
pub const API_REPO_DOC_COVERAGE_OPENAPI_PATH: &str = "/api/repo/doc-coverage";
/// Axum runtime path for the repo sync endpoint.
pub const API_REPO_SYNC_AXUM_PATH: &str = "/api/repo/sync";
/// `OpenAPI` path for the repo sync endpoint.
pub const API_REPO_SYNC_OPENAPI_PATH: &str = "/api/repo/sync";
/// Axum runtime path for the repo projected-pages endpoint.
pub const API_REPO_PROJECTED_PAGES_AXUM_PATH: &str = "/api/repo/projected-pages";
/// `OpenAPI` path for the repo projected-pages endpoint.
pub const API_REPO_PROJECTED_PAGES_OPENAPI_PATH: &str = "/api/repo/projected-pages";
/// Axum runtime path for the repo projected-page endpoint.
pub const API_REPO_PROJECTED_PAGE_AXUM_PATH: &str = "/api/repo/projected-page";
/// `OpenAPI` path for the repo projected-page endpoint.
pub const API_REPO_PROJECTED_PAGE_OPENAPI_PATH: &str = "/api/repo/projected-page";
/// Axum runtime path for the repo projected-page-index-node endpoint.
pub const API_REPO_PROJECTED_PAGE_INDEX_NODE_AXUM_PATH: &str =
    "/api/repo/projected-page-index-node";
/// `OpenAPI` path for the repo projected-page-index-node endpoint.
pub const API_REPO_PROJECTED_PAGE_INDEX_NODE_OPENAPI_PATH: &str =
    "/api/repo/projected-page-index-node";
/// Axum runtime path for the repo projected-retrieval-hit endpoint.
pub const API_REPO_PROJECTED_RETRIEVAL_HIT_AXUM_PATH: &str = "/api/repo/projected-retrieval-hit";
/// `OpenAPI` path for the repo projected-retrieval-hit endpoint.
pub const API_REPO_PROJECTED_RETRIEVAL_HIT_OPENAPI_PATH: &str = "/api/repo/projected-retrieval-hit";
/// Axum runtime path for the repo projected-retrieval-context endpoint.
pub const API_REPO_PROJECTED_RETRIEVAL_CONTEXT_AXUM_PATH: &str =
    "/api/repo/projected-retrieval-context";
/// `OpenAPI` path for the repo projected-retrieval-context endpoint.
pub const API_REPO_PROJECTED_RETRIEVAL_CONTEXT_OPENAPI_PATH: &str =
    "/api/repo/projected-retrieval-context";
/// Axum runtime path for the repo projected-page-family-context endpoint.
pub const API_REPO_PROJECTED_PAGE_FAMILY_CONTEXT_AXUM_PATH: &str =
    "/api/repo/projected-page-family-context";
/// `OpenAPI` path for the repo projected-page-family-context endpoint.
pub const API_REPO_PROJECTED_PAGE_FAMILY_CONTEXT_OPENAPI_PATH: &str =
    "/api/repo/projected-page-family-context";
/// Axum runtime path for the repo projected-page-family-search endpoint.
pub const API_REPO_PROJECTED_PAGE_FAMILY_SEARCH_AXUM_PATH: &str =
    "/api/repo/projected-page-family-search";
/// `OpenAPI` path for the repo projected-page-family-search endpoint.
pub const API_REPO_PROJECTED_PAGE_FAMILY_SEARCH_OPENAPI_PATH: &str =
    "/api/repo/projected-page-family-search";
/// Axum runtime path for the repo projected-page-family-cluster endpoint.
pub const API_REPO_PROJECTED_PAGE_FAMILY_CLUSTER_AXUM_PATH: &str =
    "/api/repo/projected-page-family-cluster";
/// `OpenAPI` path for the repo projected-page-family-cluster endpoint.
pub const API_REPO_PROJECTED_PAGE_FAMILY_CLUSTER_OPENAPI_PATH: &str =
    "/api/repo/projected-page-family-cluster";
/// Axum runtime path for the repo projected-page-navigation endpoint.
pub const API_REPO_PROJECTED_PAGE_NAVIGATION_AXUM_PATH: &str =
    "/api/repo/projected-page-navigation";
/// `OpenAPI` path for the repo projected-page-navigation endpoint.
pub const API_REPO_PROJECTED_PAGE_NAVIGATION_OPENAPI_PATH: &str =
    "/api/repo/projected-page-navigation";
/// Axum runtime path for the repo projected-page-navigation-search endpoint.
pub const API_REPO_PROJECTED_PAGE_NAVIGATION_SEARCH_AXUM_PATH: &str =
    "/api/repo/projected-page-navigation-search";
/// `OpenAPI` path for the repo projected-page-navigation-search endpoint.
pub const API_REPO_PROJECTED_PAGE_NAVIGATION_SEARCH_OPENAPI_PATH: &str =
    "/api/repo/projected-page-navigation-search";
/// Axum runtime path for the repo projected-page-index-tree endpoint.
pub const API_REPO_PROJECTED_PAGE_INDEX_TREE_AXUM_PATH: &str =
    "/api/repo/projected-page-index-tree";
/// `OpenAPI` path for the repo projected-page-index-tree endpoint.
pub const API_REPO_PROJECTED_PAGE_INDEX_TREE_OPENAPI_PATH: &str =
    "/api/repo/projected-page-index-tree";
/// Axum runtime path for the repo projected-page-index-tree-search endpoint.
pub const API_REPO_PROJECTED_PAGE_INDEX_TREE_SEARCH_AXUM_PATH: &str =
    "/api/repo/projected-page-index-tree-search";
/// `OpenAPI` path for the repo projected-page-index-tree-search endpoint.
pub const API_REPO_PROJECTED_PAGE_INDEX_TREE_SEARCH_OPENAPI_PATH: &str =
    "/api/repo/projected-page-index-tree-search";
/// Axum runtime path for the repo projected-page-search endpoint.
pub const API_REPO_PROJECTED_PAGE_SEARCH_AXUM_PATH: &str = "/api/repo/projected-page-search";
/// `OpenAPI` path for the repo projected-page-search endpoint.
pub const API_REPO_PROJECTED_PAGE_SEARCH_OPENAPI_PATH: &str = "/api/repo/projected-page-search";
/// Axum runtime path for the repo projected-retrieval endpoint.
pub const API_REPO_PROJECTED_RETRIEVAL_AXUM_PATH: &str = "/api/repo/projected-retrieval";
/// `OpenAPI` path for the repo projected-retrieval endpoint.
pub const API_REPO_PROJECTED_RETRIEVAL_OPENAPI_PATH: &str = "/api/repo/projected-retrieval";
/// Axum runtime path for the repo projected-page-index-trees endpoint.
pub const API_REPO_PROJECTED_PAGE_INDEX_TREES_AXUM_PATH: &str =
    "/api/repo/projected-page-index-trees";
/// `OpenAPI` path for the repo projected-page-index-trees endpoint.
pub const API_REPO_PROJECTED_PAGE_INDEX_TREES_OPENAPI_PATH: &str =
    "/api/repo/projected-page-index-trees";

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
        axum_path: API_ANALYSIS_MARKDOWN_AXUM_PATH,
        openapi_path: API_ANALYSIS_MARKDOWN_OPENAPI_PATH,
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
