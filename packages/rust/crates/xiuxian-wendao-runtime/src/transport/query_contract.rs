use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};

/// Canonical schema-version metadata header for Wendao Flight requests.
pub const WENDAO_SCHEMA_VERSION_HEADER: &str = "x-wendao-schema-version";
/// Canonical rerank-embedding dimension metadata header for Wendao Flight exchange requests.
pub const WENDAO_RERANK_DIMENSION_HEADER: &str = "x-wendao-rerank-embedding-dimension";
/// Canonical rerank top-k metadata header for Wendao Flight exchange requests.
#[cfg(feature = "julia")]
pub const WENDAO_RERANK_TOP_K_HEADER: &str = "x-wendao-rerank-top-k";
/// Canonical rerank minimum-final-score metadata header for Wendao Flight exchange requests.
#[cfg(feature = "julia")]
pub const WENDAO_RERANK_MIN_FINAL_SCORE_HEADER: &str = "x-wendao-rerank-min-final-score";
/// Canonical repo-search query text metadata header for Wendao Flight requests.
pub const WENDAO_REPO_SEARCH_QUERY_HEADER: &str = "x-wendao-repo-search-query";
/// Canonical repo-search result-limit metadata header for Wendao Flight requests.
pub const WENDAO_REPO_SEARCH_LIMIT_HEADER: &str = "x-wendao-repo-search-limit";
/// Canonical repo-search repository metadata header for Wendao Flight requests.
pub const WENDAO_REPO_SEARCH_REPO_HEADER: &str = "x-wendao-repo-search-repo";
/// Canonical repo-doc-coverage repository metadata header for Wendao Flight requests.
pub const WENDAO_REPO_DOC_COVERAGE_REPO_HEADER: &str = "x-wendao-repo-doc-coverage-repo";
/// Canonical repo-overview repository metadata header for Wendao Flight requests.
pub const WENDAO_REPO_OVERVIEW_REPO_HEADER: &str = "x-wendao-repo-overview-repo";
/// Canonical repo-index repository metadata header for Wendao Flight requests.
pub const WENDAO_REPO_INDEX_REPO_HEADER: &str = "x-wendao-repo-index-repo";
/// Canonical repo-index refresh metadata header for Wendao Flight requests.
pub const WENDAO_REPO_INDEX_REFRESH_HEADER: &str = "x-wendao-repo-index-refresh";
/// Canonical repo-index request identifier metadata header for Wendao Flight
/// requests.
pub const WENDAO_REPO_INDEX_REQUEST_ID_HEADER: &str = "x-wendao-repo-index-request-id";
/// Canonical repo-index-status repository metadata header for Wendao Flight requests.
pub const WENDAO_REPO_INDEX_STATUS_REPO_HEADER: &str = "x-wendao-repo-index-status-repo";
/// Canonical repo-sync repository metadata header for Wendao Flight requests.
pub const WENDAO_REPO_SYNC_REPO_HEADER: &str = "x-wendao-repo-sync-repo";
/// Canonical repo-sync mode metadata header for Wendao Flight requests.
pub const WENDAO_REPO_SYNC_MODE_HEADER: &str = "x-wendao-repo-sync-mode";
/// Canonical repo-doc-coverage module metadata header for Wendao Flight requests.
pub const WENDAO_REPO_DOC_COVERAGE_MODULE_HEADER: &str = "x-wendao-repo-doc-coverage-module";
/// Canonical projected page-index tree repository metadata header for Wendao
/// Flight requests.
pub const WENDAO_REPO_PROJECTED_PAGE_INDEX_TREE_REPO_HEADER: &str =
    "x-wendao-repo-projected-page-index-tree-repo";
/// Canonical projected page-index tree page metadata header for Wendao Flight
/// requests.
pub const WENDAO_REPO_PROJECTED_PAGE_INDEX_TREE_PAGE_ID_HEADER: &str =
    "x-wendao-repo-projected-page-index-tree-page-id";
/// Canonical refine-doc repository metadata header for Wendao Flight requests.
pub const WENDAO_REFINE_DOC_REPO_HEADER: &str = "x-wendao-refine-doc-repo";
/// Canonical refine-doc entity identifier metadata header for Wendao Flight
/// requests.
pub const WENDAO_REFINE_DOC_ENTITY_ID_HEADER: &str = "x-wendao-refine-doc-entity-id";
/// Canonical refine-doc user hints metadata header for Wendao Flight requests.
pub const WENDAO_REFINE_DOC_USER_HINTS_HEADER: &str = "x-wendao-refine-doc-user-hints-b64";
/// Canonical generic search query text metadata header for Wendao Flight requests.
pub const WENDAO_SEARCH_QUERY_HEADER: &str = "x-wendao-search-query";
/// Canonical generic search result-limit metadata header for Wendao Flight requests.
pub const WENDAO_SEARCH_LIMIT_HEADER: &str = "x-wendao-search-limit";
/// Canonical generic search intent metadata header for Wendao Flight requests.
pub const WENDAO_SEARCH_INTENT_HEADER: &str = "x-wendao-search-intent";
/// Canonical generic search repository hint metadata header for Wendao Flight requests.
pub const WENDAO_SEARCH_REPO_HEADER: &str = "x-wendao-search-repo";
/// Canonical SQL query text metadata header for Wendao Flight requests.
pub const WENDAO_SQL_QUERY_HEADER: &str = "x-wendao-sql-query";
/// Canonical definition-resolution query metadata header for Wendao Flight requests.
pub const WENDAO_DEFINITION_QUERY_HEADER: &str = "x-wendao-definition-query";
/// Canonical definition-resolution source-path metadata header for Wendao Flight requests.
pub const WENDAO_DEFINITION_PATH_HEADER: &str = "x-wendao-definition-path";
/// Canonical definition-resolution source-line metadata header for Wendao Flight requests.
pub const WENDAO_DEFINITION_LINE_HEADER: &str = "x-wendao-definition-line";
/// Canonical autocomplete prefix metadata header for Wendao Flight requests.
pub const WENDAO_AUTOCOMPLETE_PREFIX_HEADER: &str = "x-wendao-autocomplete-prefix";
/// Canonical autocomplete result-limit metadata header for Wendao Flight requests.
pub const WENDAO_AUTOCOMPLETE_LIMIT_HEADER: &str = "x-wendao-autocomplete-limit";
/// Canonical VFS resolve path metadata header for Wendao Flight requests.
pub const WENDAO_VFS_PATH_HEADER: &str = "x-wendao-vfs-path";
/// Canonical graph-neighbors node identifier metadata header for Wendao Flight
/// requests.
pub const WENDAO_GRAPH_NODE_ID_HEADER: &str = "x-wendao-graph-node-id";
/// Canonical graph-neighbors direction metadata header for Wendao Flight
/// requests.
pub const WENDAO_GRAPH_DIRECTION_HEADER: &str = "x-wendao-graph-direction";
/// Canonical graph-neighbors hop-limit metadata header for Wendao Flight
/// requests.
pub const WENDAO_GRAPH_HOPS_HEADER: &str = "x-wendao-graph-hops";
/// Canonical graph-neighbors result-limit metadata header for Wendao Flight
/// requests.
pub const WENDAO_GRAPH_LIMIT_HEADER: &str = "x-wendao-graph-limit";
/// Canonical analysis path metadata header for Wendao Flight requests.
pub const WENDAO_ANALYSIS_PATH_HEADER: &str = "x-wendao-analysis-path";
/// Canonical analysis repository metadata header for Wendao Flight requests.
pub const WENDAO_ANALYSIS_REPO_HEADER: &str = "x-wendao-analysis-repo";
/// Canonical analysis line-hint metadata header for Wendao Flight requests.
pub const WENDAO_ANALYSIS_LINE_HEADER: &str = "x-wendao-analysis-line";
/// Canonical attachment-search extension-filter metadata header for Wendao Flight requests.
pub const WENDAO_ATTACHMENT_SEARCH_EXT_FILTERS_HEADER: &str =
    "x-wendao-attachment-search-ext-filters";
/// Canonical attachment-search kind-filter metadata header for Wendao Flight requests.
pub const WENDAO_ATTACHMENT_SEARCH_KIND_FILTERS_HEADER: &str =
    "x-wendao-attachment-search-kind-filters";
/// Canonical attachment-search case-sensitive metadata header for Wendao Flight requests.
pub const WENDAO_ATTACHMENT_SEARCH_CASE_SENSITIVE_HEADER: &str =
    "x-wendao-attachment-search-case-sensitive";
/// Canonical repo-search language-filter metadata header for Wendao Flight requests.
pub const WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER: &str =
    "x-wendao-repo-search-language-filters";
/// Canonical repo-search path-prefix metadata header for Wendao Flight requests.
pub const WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER: &str = "x-wendao-repo-search-path-prefixes";
/// Canonical repo-search title-filter metadata header for Wendao Flight requests.
pub const WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER: &str = "x-wendao-repo-search-title-filters";
/// Canonical repo-search tag-filter metadata header for Wendao Flight requests.
pub const WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER: &str = "x-wendao-repo-search-tag-filters";
/// Canonical repo-search filename-filter metadata header for Wendao Flight requests.
pub const WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER: &str =
    "x-wendao-repo-search-filename-filters";
/// Stable route for the repo-search query contract.
pub const REPO_SEARCH_ROUTE: &str = "/search/repos/main";
/// Stable route for the search-intent contract.
pub const SEARCH_INTENT_ROUTE: &str = "/search/intent";
/// Stable route for the general knowledge-search contract.
pub const SEARCH_KNOWLEDGE_ROUTE: &str = "/search/knowledge";
/// Stable route for the search-attachments contract.
pub const SEARCH_ATTACHMENTS_ROUTE: &str = "/search/attachments";
/// Stable route for the search-AST contract.
pub const SEARCH_AST_ROUTE: &str = "/search/ast";
/// Stable route for the search-references contract.
pub const SEARCH_REFERENCES_ROUTE: &str = "/search/references";
/// Stable route for the search-symbols contract.
pub const SEARCH_SYMBOLS_ROUTE: &str = "/search/symbols";
/// Stable route for the definition-resolution contract.
pub const SEARCH_DEFINITION_ROUTE: &str = "/search/definition";
/// Stable route for the autocomplete contract.
pub const SEARCH_AUTOCOMPLETE_ROUTE: &str = "/search/autocomplete";
/// Stable route for the read-only SQL query contract.
pub const QUERY_SQL_ROUTE: &str = "/query/sql";
/// Stable route for the VFS navigation-resolution contract.
pub const VFS_RESOLVE_ROUTE: &str = "/vfs/resolve";
/// Stable route for the VFS content-read contract.
pub const VFS_CONTENT_ROUTE: &str = "/vfs/content";
/// Stable route for the VFS scan contract.
pub const VFS_SCAN_ROUTE: &str = "/vfs/scan";
/// Stable route for the graph-neighbors contract.
pub const GRAPH_NEIGHBORS_ROUTE: &str = "/graph/neighbors";
/// Stable route for the 3D topology contract.
pub const TOPOLOGY_3D_ROUTE: &str = "/topology/3d";
/// Stable route for the markdown analysis contract.
pub const ANALYSIS_MARKDOWN_ROUTE: &str = "/analysis/markdown";
/// Stable route for the code-AST analysis contract.
pub const ANALYSIS_CODE_AST_ROUTE: &str = "/analysis/code-ast";
/// Stable route for the repo doc-coverage analysis contract.
pub const ANALYSIS_REPO_DOC_COVERAGE_ROUTE: &str = "/analysis/repo-doc-coverage";
/// Stable route for the repo overview analysis contract.
pub const ANALYSIS_REPO_OVERVIEW_ROUTE: &str = "/analysis/repo-overview";
/// Stable route for the repo index analysis contract.
pub const ANALYSIS_REPO_INDEX_ROUTE: &str = "/analysis/repo-index";
/// Stable route for the repo index-status analysis contract.
pub const ANALYSIS_REPO_INDEX_STATUS_ROUTE: &str = "/analysis/repo-index-status";
/// Stable route for the repo sync analysis contract.
pub const ANALYSIS_REPO_SYNC_ROUTE: &str = "/analysis/repo-sync";
/// Stable route for the repo projected page-index tree analysis contract.
pub const ANALYSIS_REPO_PROJECTED_PAGE_INDEX_TREE_ROUTE: &str =
    "/analysis/repo-projected-page-index-tree";
/// Stable route for the refine-doc analysis contract.
pub const ANALYSIS_REFINE_DOC_ROUTE: &str = "/analysis/refine-doc";
/// Stable route for the rerank contract.
pub const RERANK_ROUTE: &str = "/rerank";
/// Stable default result limit for repo-search requests.
pub const REPO_SEARCH_DEFAULT_LIMIT: usize = 10;
/// Stable default hop distance for graph-neighbors requests.
pub const GRAPH_NEIGHBORS_DEFAULT_HOPS: usize = 2;
/// Stable default result limit for graph-neighbors requests.
pub const GRAPH_NEIGHBORS_DEFAULT_LIMIT: usize = 50;
const GRAPH_NEIGHBORS_MAX_HOPS: usize = 8;
const GRAPH_NEIGHBORS_MAX_LIMIT: usize = 300;
/// Canonical rerank request `doc_id` column.
pub const RERANK_REQUEST_DOC_ID_COLUMN: &str = "doc_id";
/// Canonical rerank request `vector_score` column.
pub const RERANK_REQUEST_VECTOR_SCORE_COLUMN: &str = "vector_score";
/// Canonical rerank request `embedding` column.
pub const RERANK_REQUEST_EMBEDDING_COLUMN: &str = "embedding";
/// Canonical rerank request `query_embedding` column.
pub const RERANK_REQUEST_QUERY_EMBEDDING_COLUMN: &str = "query_embedding";
/// Canonical rerank response `doc_id` column.
pub const RERANK_RESPONSE_DOC_ID_COLUMN: &str = "doc_id";
/// Canonical rerank response raw vector-score column.
pub const RERANK_RESPONSE_VECTOR_SCORE_COLUMN: &str = "vector_score";
/// Canonical rerank response semantic-score column.
pub const RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN: &str = "semantic_score";
/// Canonical rerank response `final_score` column.
pub const RERANK_RESPONSE_FINAL_SCORE_COLUMN: &str = "final_score";
/// Canonical rerank response `rank` column.
pub const RERANK_RESPONSE_RANK_COLUMN: &str = "rank";
/// Canonical repo-search response `doc_id` column.
pub const REPO_SEARCH_DOC_ID_COLUMN: &str = "doc_id";
/// Canonical repo-search response `path` column.
pub const REPO_SEARCH_PATH_COLUMN: &str = "path";
/// Canonical repo-search response `title` column.
pub const REPO_SEARCH_TITLE_COLUMN: &str = "title";
/// Canonical repo-search response `best_section` column.
pub const REPO_SEARCH_BEST_SECTION_COLUMN: &str = "best_section";
/// Canonical repo-search response `match_reason` column.
pub const REPO_SEARCH_MATCH_REASON_COLUMN: &str = "match_reason";
/// Canonical repo-search response navigation-path column.
pub const REPO_SEARCH_NAVIGATION_PATH_COLUMN: &str = "navigation_path";
/// Canonical repo-search response navigation-category column.
pub const REPO_SEARCH_NAVIGATION_CATEGORY_COLUMN: &str = "navigation_category";
/// Canonical repo-search response navigation-line column.
pub const REPO_SEARCH_NAVIGATION_LINE_COLUMN: &str = "navigation_line";
/// Canonical repo-search response navigation-line-end column.
pub const REPO_SEARCH_NAVIGATION_LINE_END_COLUMN: &str = "navigation_line_end";
/// Canonical repo-search response hierarchy column.
pub const REPO_SEARCH_HIERARCHY_COLUMN: &str = "hierarchy";
/// Canonical repo-search response `tags` column.
pub const REPO_SEARCH_TAGS_COLUMN: &str = "tags";
/// Canonical repo-search response `score` column.
pub const REPO_SEARCH_SCORE_COLUMN: &str = "score";
/// Canonical repo-search response `language` column.
pub const REPO_SEARCH_LANGUAGE_COLUMN: &str = "language";

#[cfg(feature = "julia")]
use arrow_array::{
    FixedSizeListArray, Float32Array, Float64Array, Int32Array, RecordBatch, StringArray,
};
#[cfg(feature = "julia")]
use arrow_schema::{DataType, Schema};
#[cfg(feature = "julia")]
use datafusion::sql::parser::{DFParser, Statement as DataFusionStatement};
#[cfg(feature = "julia")]
use datafusion::sql::sqlparser::ast::Statement as SqlStatement;
#[cfg(feature = "julia")]
use std::collections::HashSet;

/// One scored rerank candidate produced by the shared Rust-owned scorer.
#[cfg(feature = "julia")]
#[derive(Debug, Clone, PartialEq)]
pub struct RerankScoredCandidate {
    /// Stable candidate identifier carried through the rerank request.
    pub doc_id: String,
    /// Raw vector score from the rerank request.
    pub vector_score: f64,
    /// Semantic score derived from cosine similarity and normalized into `[0, 1]`.
    pub semantic_score: f64,
    /// Final blended rerank score.
    pub final_score: f64,
}

/// Shared runtime-owned rerank score weights.
#[cfg(feature = "julia")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RerankScoreWeights {
    /// Weight applied to the inbound `vector_score`.
    pub vector_weight: f64,
    /// Weight applied to the derived `semantic_score`.
    pub semantic_weight: f64,
}

#[cfg(feature = "julia")]
impl Default for RerankScoreWeights {
    fn default() -> Self {
        Self {
            vector_weight: 0.4,
            semantic_weight: 0.6,
        }
    }
}

#[cfg(feature = "julia")]
impl RerankScoreWeights {
    /// Construct one validated rerank score-weight policy.
    ///
    /// # Errors
    ///
    /// Returns an error when either weight is non-finite, negative, or when
    /// both weights sum to zero.
    pub fn new(vector_weight: f64, semantic_weight: f64) -> Result<Self, String> {
        if !vector_weight.is_finite() {
            return Err("rerank vector_weight must be finite".to_string());
        }
        if !semantic_weight.is_finite() {
            return Err("rerank semantic_weight must be finite".to_string());
        }
        if vector_weight < 0.0 {
            return Err("rerank vector_weight must be greater than or equal to zero".to_string());
        }
        if semantic_weight < 0.0 {
            return Err("rerank semantic_weight must be greater than or equal to zero".to_string());
        }
        let total = vector_weight + semantic_weight;
        if total <= 0.0 {
            return Err("rerank score weights must sum to greater than zero".to_string());
        }
        Ok(Self {
            vector_weight,
            semantic_weight,
        })
    }

    /// Return the normalized score weights whose sum is exactly `1.0`.
    #[must_use]
    pub fn normalized(self) -> Self {
        let total = self.vector_weight + self.semantic_weight;
        Self {
            vector_weight: self.vector_weight / total,
            semantic_weight: self.semantic_weight / total,
        }
    }
}

/// Normalize one route into the canonical leading-slash Flight form.
///
/// # Errors
///
/// Returns an error when the route resolves to no descriptor segments.
pub fn normalize_flight_route(route: impl AsRef<str>) -> Result<String, String> {
    let route = route.as_ref();
    let normalized = if route.starts_with('/') {
        route.to_string()
    } else {
        format!("/{route}")
    };
    if normalized.trim_matches('/').is_empty() {
        return Err(
            "Arrow Flight route must resolve to at least one descriptor segment".to_string(),
        );
    }
    Ok(normalized)
}

/// Convert one canonical Flight route into descriptor-path segments.
///
/// # Errors
///
/// Returns an error when the route resolves to no descriptor segments.
pub fn flight_descriptor_path(route: impl AsRef<str>) -> Result<Vec<String>, String> {
    let normalized = normalize_flight_route(route)?;
    let path = normalized
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if path.is_empty() {
        return Err(
            "Arrow Flight route must resolve to at least one descriptor segment".to_string(),
        );
    }
    Ok(path)
}

/// Validate the stable repo-search request contract.
///
/// # Errors
///
/// Returns an error when the repo-search query text is blank or the requested
/// limit is zero.
pub fn validate_repo_search_request(
    query_text: &str,
    limit: usize,
    language_filters: &[String],
    path_prefixes: &[String],
    title_filters: &[String],
    tag_filters: &[String],
    filename_filters: &[String],
) -> Result<(), String> {
    if query_text.trim().is_empty() {
        return Err("repo search query text must not be blank".to_string());
    }
    if limit == 0 {
        return Err("repo search limit must be greater than zero".to_string());
    }
    for language_filter in language_filters {
        if language_filter.trim().is_empty() {
            return Err("repo search language filters must not contain blank values".to_string());
        }
    }
    for path_prefix in path_prefixes {
        if path_prefix.trim().is_empty() {
            return Err("repo search path prefixes must not contain blank values".to_string());
        }
    }
    for title_filter in title_filters {
        if title_filter.trim().is_empty() {
            return Err("repo search title filters must not contain blank values".to_string());
        }
    }
    for tag_filter in tag_filters {
        if tag_filter.trim().is_empty() {
            return Err("repo search tag filters must not contain blank values".to_string());
        }
    }
    for filename_filter in filename_filters {
        if filename_filter.trim().is_empty() {
            return Err("repo search filename filters must not contain blank values".to_string());
        }
    }
    Ok(())
}

/// Validate the stable attachment-search request contract.
///
/// # Errors
///
/// Returns an error when the attachment-search query text is blank, the
/// requested limit is zero, or any declared extension/kind filter is blank.
pub fn validate_attachment_search_request(
    query_text: &str,
    limit: usize,
    ext_filters: &[String],
    kind_filters: &[String],
) -> Result<(), String> {
    validate_repo_search_request(query_text, limit, &[], &[], &[], &[], &[])?;
    for ext_filter in ext_filters {
        if ext_filter.trim().is_empty() {
            return Err(
                "attachment search extension filters must not contain blank values".to_string(),
            );
        }
    }
    for kind_filter in kind_filters {
        if kind_filter.trim().is_empty() {
            return Err("attachment search kind filters must not contain blank values".to_string());
        }
    }
    Ok(())
}

/// Validate the stable definition-resolution request contract.
///
/// # Errors
///
/// Returns an error when the definition query text is blank, when the optional
/// source path is blank, or when the optional source line is zero.
pub fn validate_definition_request(
    query_text: &str,
    source_path: Option<&str>,
    source_line: Option<usize>,
) -> Result<(), String> {
    if query_text.trim().is_empty() {
        return Err("definition query text must not be blank".to_string());
    }
    if matches!(source_path, Some(path) if path.trim().is_empty()) {
        return Err("definition source path must not be blank".to_string());
    }
    if matches!(source_line, Some(0)) {
        return Err("definition source line must be greater than zero".to_string());
    }
    Ok(())
}

/// Validate the stable autocomplete request contract.
///
/// # Errors
///
/// Returns an error when the requested limit is zero or when the optional
/// prefix contains only whitespace.
pub fn validate_autocomplete_request(prefix: &str, limit: usize) -> Result<(), String> {
    if limit == 0 {
        return Err("autocomplete limit must be greater than zero".to_string());
    }
    if !prefix.is_empty() && prefix.trim().is_empty() {
        return Err("autocomplete prefix must not be blank".to_string());
    }
    Ok(())
}

/// Validate the stable read-only SQL request contract.
///
/// # Errors
///
/// Returns an error when the query text is blank, parses as multiple
/// statements, or resolves to anything other than one read-only `SELECT`-style
/// query statement.
#[cfg(feature = "julia")]
pub fn validate_sql_query_request(query_text: &str) -> Result<(), String> {
    let normalized_query = query_text.trim();
    if normalized_query.is_empty() {
        return Err("SQL query text must not be blank".to_string());
    }

    let mut statements = DFParser::parse_sql(normalized_query)
        .map_err(|error| format!("failed to parse SQL query text: {error}"))?;
    if statements.len() != 1 {
        return Err("SQL query text must contain exactly one statement".to_string());
    }

    let statement = statements
        .pop_front()
        .ok_or_else(|| "SQL query text must contain exactly one statement".to_string())?;
    match statement {
        DataFusionStatement::Statement(statement)
            if matches!(statement.as_ref(), SqlStatement::Query(_)) =>
        {
            Ok(())
        }
        _ => Err("SQL query text must be a read-only query statement".to_string()),
    }
}

/// Validate the stable VFS navigation-resolution request contract.
///
/// # Errors
///
/// Returns an error when the path is blank.
pub fn validate_vfs_resolve_request(path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("VFS resolve requires a non-empty path".to_string());
    }
    Ok(())
}

/// Validate the stable VFS content-read request contract.
///
/// # Errors
///
/// Returns an error when the path is blank.
pub fn validate_vfs_content_request(path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("VFS content requires a non-empty path".to_string());
    }
    Ok(())
}

/// Validate and normalize the stable graph-neighbors request contract.
///
/// # Errors
///
/// Returns an error when the requested node identifier is blank.
pub fn validate_graph_neighbors_request(
    node_id: &str,
    direction: Option<&str>,
    hops: Option<usize>,
    limit: Option<usize>,
) -> Result<(String, String, usize, usize), String> {
    let normalized_node_id = node_id.trim();
    if normalized_node_id.is_empty() {
        return Err("graph neighbors requires a non-empty node id".to_string());
    }

    let normalized_direction = match direction
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("incoming") => "incoming",
        Some("outgoing") => "outgoing",
        _ => "both",
    };
    let normalized_hops = hops
        .unwrap_or(GRAPH_NEIGHBORS_DEFAULT_HOPS)
        .clamp(1, GRAPH_NEIGHBORS_MAX_HOPS);
    let normalized_limit = limit
        .unwrap_or(GRAPH_NEIGHBORS_DEFAULT_LIMIT)
        .clamp(1, GRAPH_NEIGHBORS_MAX_LIMIT);

    Ok((
        normalized_node_id.to_string(),
        normalized_direction.to_string(),
        normalized_hops,
        normalized_limit,
    ))
}

/// Validate the stable markdown analysis request contract.
///
/// # Errors
///
/// Returns an error when the repository-relative path is blank.
pub fn validate_markdown_analysis_request(path: &str) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("markdown analysis path must not be blank".to_string());
    }
    Ok(())
}

/// Validate the stable code-AST analysis request contract.
///
/// # Errors
///
/// Returns an error when the repository-relative path is blank, when the repo
/// identifier is blank, or when the optional line hint is zero.
pub fn validate_code_ast_analysis_request(
    path: &str,
    repo_id: &str,
    line_hint: Option<usize>,
) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("code AST analysis path must not be blank".to_string());
    }
    if repo_id.trim().is_empty() {
        return Err("code AST analysis repo must not be blank".to_string());
    }
    if matches!(line_hint, Some(0)) {
        return Err("code AST analysis line hint must be greater than zero".to_string());
    }
    Ok(())
}

/// Validate the stable repo overview request contract.
///
/// # Errors
///
/// Returns an error when the repository identifier is blank.
pub fn validate_repo_overview_request(repo_id: &str) -> Result<String, String> {
    let normalized_repo_id = repo_id.trim();
    if normalized_repo_id.is_empty() {
        return Err("repo overview repo must not be blank".to_string());
    }
    Ok(normalized_repo_id.to_string())
}

/// Validate the stable repo index-status request contract.
///
/// # Errors
///
/// This validator currently does not produce contract-local errors.
pub fn validate_repo_index_status_request(repo_id: Option<&str>) -> Result<Option<String>, String> {
    Ok(repo_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string))
}

/// Validate the stable repo index request contract.
///
/// # Errors
///
/// Returns an error when the optional refresh flag is not a canonical boolean
/// or when the request identifier is blank.
pub fn validate_repo_index_request(
    repo_id: Option<&str>,
    refresh: Option<&str>,
    request_id: &str,
) -> Result<(Option<String>, bool, String), String> {
    let normalized_repo_id = repo_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    let normalized_refresh = match refresh
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("false")
    {
        "true" => true,
        "false" => false,
        other => return Err(format!("unsupported repo index refresh flag `{other}`")),
    };
    let normalized_request_id = request_id.trim();
    if normalized_request_id.is_empty() {
        return Err("repo index request id must not be blank".to_string());
    }
    Ok((
        normalized_repo_id,
        normalized_refresh,
        normalized_request_id.to_string(),
    ))
}

/// Validate the stable repo sync request contract.
///
/// # Errors
///
/// Returns an error when the repository identifier is blank or when the sync
/// mode is unsupported.
pub fn validate_repo_sync_request(
    repo_id: &str,
    mode: Option<&str>,
) -> Result<(String, String), String> {
    let normalized_repo_id = repo_id.trim();
    if normalized_repo_id.is_empty() {
        return Err("repo sync repo must not be blank".to_string());
    }
    let normalized_mode = match mode
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("ensure")
    {
        "ensure" | "refresh" | "status" => mode
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("ensure")
            .to_string(),
        other => return Err(format!("unsupported repo sync mode `{other}`")),
    };
    Ok((normalized_repo_id.to_string(), normalized_mode))
}

/// Validate the stable repo doc-coverage request contract.
///
/// # Errors
///
/// Returns an error when the repository identifier is blank.
pub fn validate_repo_doc_coverage_request(
    repo_id: &str,
    module_id: Option<&str>,
) -> Result<(String, Option<String>), String> {
    let normalized_repo_id = repo_id.trim();
    if normalized_repo_id.is_empty() {
        return Err("repo doc coverage repo must not be blank".to_string());
    }
    let normalized_module_id = module_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    Ok((normalized_repo_id.to_string(), normalized_module_id))
}

/// Validate the stable projected page-index tree request contract.
///
/// # Errors
///
/// Returns an error when the repository identifier or page identifier is
/// blank.
pub fn validate_repo_projected_page_index_tree_request(
    repo_id: &str,
    page_id: &str,
) -> Result<(String, String), String> {
    let normalized_repo_id = repo_id.trim();
    if normalized_repo_id.is_empty() {
        return Err("repo projected page-index tree repo must not be blank".to_string());
    }
    let normalized_page_id = page_id.trim();
    if normalized_page_id.is_empty() {
        return Err("repo projected page-index tree page id must not be blank".to_string());
    }
    Ok((
        normalized_repo_id.to_string(),
        normalized_page_id.to_string(),
    ))
}

/// Validate the stable refine-doc request contract.
///
/// # Errors
///
/// Returns an error when the repository identifier or entity identifier is
/// blank, or when the optional Base64-encoded user hints cannot be decoded
/// into valid UTF-8.
pub fn validate_refine_doc_request(
    repo_id: &str,
    entity_id: &str,
    user_hints_base64: Option<&str>,
) -> Result<(String, String, Option<String>), String> {
    let normalized_repo_id = repo_id.trim();
    if normalized_repo_id.is_empty() {
        return Err("refine doc repo must not be blank".to_string());
    }
    let normalized_entity_id = entity_id.trim();
    if normalized_entity_id.is_empty() {
        return Err("refine doc entity_id must not be blank".to_string());
    }
    let normalized_user_hints = user_hints_base64
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| {
            let decoded = BASE64_STANDARD
                .decode(value)
                .map_err(|error| format!("refine doc user_hints must be valid Base64: {error}"))?;
            String::from_utf8(decoded)
                .map_err(|error| format!("refine doc user_hints must be valid UTF-8: {error}"))
        })
        .transpose()?;
    Ok((
        normalized_repo_id.to_string(),
        normalized_entity_id.to_string(),
        normalized_user_hints,
    ))
}

/// Validate the stable rerank request schema for one expected embedding dimension.
///
/// # Errors
///
/// Returns an error when the rerank request schema does not match the stable
/// Rust-owned column set or Arrow types.
#[cfg(feature = "julia")]
pub fn validate_rerank_request_schema(
    schema: &Schema,
    expected_dimension: usize,
) -> Result<(), String> {
    let doc_id = schema
        .field_with_name(RERANK_REQUEST_DOC_ID_COLUMN)
        .map_err(|_| format!("missing rerank request column `{RERANK_REQUEST_DOC_ID_COLUMN}`"))?;
    if !matches!(doc_id.data_type(), DataType::Utf8) {
        return Err(format!(
            "rerank request column `{RERANK_REQUEST_DOC_ID_COLUMN}` must be Utf8"
        ));
    }

    let vector_score = schema
        .field_with_name(RERANK_REQUEST_VECTOR_SCORE_COLUMN)
        .map_err(|_| {
            format!("missing rerank request column `{RERANK_REQUEST_VECTOR_SCORE_COLUMN}`")
        })?;
    if !matches!(vector_score.data_type(), DataType::Float32) {
        return Err(format!(
            "rerank request column `{RERANK_REQUEST_VECTOR_SCORE_COLUMN}` must be Float32"
        ));
    }

    validate_embedding_field(schema, RERANK_REQUEST_EMBEDDING_COLUMN, expected_dimension)?;
    validate_embedding_field(
        schema,
        RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
        expected_dimension,
    )?;
    Ok(())
}

/// Validate the stable rerank request payload semantics for one decoded batch.
///
/// # Errors
///
/// Returns an error when the rerank request batch is empty, contains blank
/// document IDs, contains duplicate document IDs, contains non-finite or
/// out-of-range vector scores, or carries drifted `query_embedding` values
/// across rows.
#[cfg(feature = "julia")]
pub fn validate_rerank_request_batch(
    batch: &RecordBatch,
    expected_dimension: usize,
) -> Result<(), String> {
    validate_rerank_request_schema(batch.schema().as_ref(), expected_dimension)?;
    if batch.num_rows() == 0 {
        return Err("rerank request batch must contain at least one row".to_string());
    }

    let doc_ids = batch
        .column_by_name(RERANK_REQUEST_DOC_ID_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| {
            format!("rerank request column `{RERANK_REQUEST_DOC_ID_COLUMN}` must decode as Utf8")
        })?;
    let mut seen_doc_ids = HashSet::new();
    for row_index in 0..batch.num_rows() {
        let doc_id = doc_ids.value(row_index).trim();
        if doc_id.is_empty() {
            return Err(format!(
                "rerank request column `{RERANK_REQUEST_DOC_ID_COLUMN}` must not contain blank values; row {row_index} is blank"
            ));
        }
        if !seen_doc_ids.insert(doc_id.to_string()) {
            return Err(format!(
                "rerank request column `{RERANK_REQUEST_DOC_ID_COLUMN}` must be unique across one batch; row {row_index} duplicates `{doc_id}`"
            ));
        }
    }

    let vector_scores = batch
        .column_by_name(RERANK_REQUEST_VECTOR_SCORE_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<Float32Array>())
        .ok_or_else(|| {
            format!(
                "rerank request column `{RERANK_REQUEST_VECTOR_SCORE_COLUMN}` must decode as Float32"
            )
        })?;
    for row_index in 0..batch.num_rows() {
        let score = vector_scores.value(row_index);
        if !score.is_finite() {
            return Err(format!(
                "rerank request column `{RERANK_REQUEST_VECTOR_SCORE_COLUMN}` must contain finite values; row {row_index} is {score}"
            ));
        }
        if !(0.0..=1.0).contains(&score) {
            return Err(format!(
                "rerank request column `{RERANK_REQUEST_VECTOR_SCORE_COLUMN}` must stay within inclusive range [0.0, 1.0]; row {row_index} is {score}"
            ));
        }
    }

    let query_embeddings = batch
        .column_by_name(RERANK_REQUEST_QUERY_EMBEDDING_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<FixedSizeListArray>())
        .ok_or_else(|| {
            format!(
                "rerank request column `{RERANK_REQUEST_QUERY_EMBEDDING_COLUMN}` must decode as FixedSizeList<Float32>"
            )
        })?;
    let first_query_embedding = fixed_size_list_row_values(query_embeddings, 0)?;
    for row_index in 1..batch.num_rows() {
        let row_query_embedding = fixed_size_list_row_values(query_embeddings, row_index)?;
        if row_query_embedding != first_query_embedding {
            return Err(format!(
                "rerank request column `{RERANK_REQUEST_QUERY_EMBEDDING_COLUMN}` must remain stable across all rows; row {row_index} differs from row 0"
            ));
        }
    }

    Ok(())
}

/// Score one validated rerank request batch with the shared Rust-owned rerank rule.
///
/// The current stable rule blends the inbound vector score with semantic cosine
/// similarity between `embedding` and `query_embedding`:
///
/// - `semantic_score = (cosine_similarity + 1.0) / 2.0`
/// - `final_score = 0.4 * vector_score + 0.6 * semantic_score`
///
/// # Errors
///
/// Returns an error when the request batch fails validation or when any
/// embedding/query vector has zero norm.
#[cfg(feature = "julia")]
pub fn score_rerank_request_batch(
    batch: &RecordBatch,
    expected_dimension: usize,
) -> Result<Vec<RerankScoredCandidate>, String> {
    score_rerank_request_batch_with_weights(
        batch,
        expected_dimension,
        RerankScoreWeights::default(),
    )
}

/// Score one validated rerank request batch with explicit runtime-owned
/// rerank weights.
///
/// # Errors
///
/// Returns an error when the request batch fails validation, when any
/// embedding/query vector has zero norm, or when the weights are invalid.
#[cfg(feature = "julia")]
pub fn score_rerank_request_batch_with_weights(
    batch: &RecordBatch,
    expected_dimension: usize,
    weights: RerankScoreWeights,
) -> Result<Vec<RerankScoredCandidate>, String> {
    validate_rerank_request_batch(batch, expected_dimension)?;
    let weights =
        RerankScoreWeights::new(weights.vector_weight, weights.semantic_weight)?.normalized();

    let doc_ids = batch
        .column_by_name(RERANK_REQUEST_DOC_ID_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| {
            format!("rerank request column `{RERANK_REQUEST_DOC_ID_COLUMN}` must decode as Utf8")
        })?;
    let vector_scores = batch
        .column_by_name(RERANK_REQUEST_VECTOR_SCORE_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<Float32Array>())
        .ok_or_else(|| {
            format!(
                "rerank request column `{RERANK_REQUEST_VECTOR_SCORE_COLUMN}` must decode as Float32"
            )
        })?;
    let embeddings = batch
        .column_by_name(RERANK_REQUEST_EMBEDDING_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<FixedSizeListArray>())
        .ok_or_else(|| {
            format!(
                "rerank request column `{RERANK_REQUEST_EMBEDDING_COLUMN}` must decode as FixedSizeList<Float32>"
            )
        })?;
    let query_embeddings = batch
        .column_by_name(RERANK_REQUEST_QUERY_EMBEDDING_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<FixedSizeListArray>())
        .ok_or_else(|| {
            format!(
                "rerank request column `{RERANK_REQUEST_QUERY_EMBEDDING_COLUMN}` must decode as FixedSizeList<Float32>"
            )
        })?;

    let mut scored_candidates = Vec::with_capacity(batch.num_rows());
    for row_index in 0..batch.num_rows() {
        let embedding = fixed_size_list_row_values(embeddings, row_index)?;
        let query_embedding = fixed_size_list_row_values(query_embeddings, row_index)?;
        let cosine = cosine_similarity(&embedding, &query_embedding, row_index)?;
        let vector_score = f64::from(vector_scores.value(row_index));
        let semantic_score = f64::midpoint(cosine, 1.0);
        let final_score =
            weights.vector_weight * vector_score + weights.semantic_weight * semantic_score;
        scored_candidates.push(RerankScoredCandidate {
            doc_id: doc_ids.value(row_index).to_string(),
            vector_score,
            semantic_score,
            final_score,
        });
    }

    Ok(scored_candidates)
}

/// Validate the stable rerank response schema.
///
/// # Errors
///
/// Returns an error when the rerank response schema does not match the
/// Rust-owned column set or Arrow types.
#[cfg(feature = "julia")]
pub fn validate_rerank_response_schema(schema: &Schema) -> Result<(), String> {
    let doc_id = schema
        .field_with_name(RERANK_RESPONSE_DOC_ID_COLUMN)
        .map_err(|_| format!("missing rerank response column `{RERANK_RESPONSE_DOC_ID_COLUMN}`"))?;
    if !matches!(doc_id.data_type(), DataType::Utf8) {
        return Err(format!(
            "rerank response column `{RERANK_RESPONSE_DOC_ID_COLUMN}` must be Utf8"
        ));
    }

    let vector_score = schema
        .field_with_name(RERANK_RESPONSE_VECTOR_SCORE_COLUMN)
        .map_err(|_| {
            format!("missing rerank response column `{RERANK_RESPONSE_VECTOR_SCORE_COLUMN}`")
        })?;
    if !matches!(vector_score.data_type(), DataType::Float64) {
        return Err(format!(
            "rerank response column `{RERANK_RESPONSE_VECTOR_SCORE_COLUMN}` must be Float64"
        ));
    }

    let semantic_score = schema
        .field_with_name(RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN)
        .map_err(|_| {
            format!("missing rerank response column `{RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN}`")
        })?;
    if !matches!(semantic_score.data_type(), DataType::Float64) {
        return Err(format!(
            "rerank response column `{RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN}` must be Float64"
        ));
    }

    let final_score = schema
        .field_with_name(RERANK_RESPONSE_FINAL_SCORE_COLUMN)
        .map_err(|_| {
            format!("missing rerank response column `{RERANK_RESPONSE_FINAL_SCORE_COLUMN}`")
        })?;
    if !matches!(final_score.data_type(), DataType::Float64) {
        return Err(format!(
            "rerank response column `{RERANK_RESPONSE_FINAL_SCORE_COLUMN}` must be Float64"
        ));
    }

    let rank = schema
        .field_with_name(RERANK_RESPONSE_RANK_COLUMN)
        .map_err(|_| format!("missing rerank response column `{RERANK_RESPONSE_RANK_COLUMN}`"))?;
    if !matches!(rank.data_type(), DataType::Int32) {
        return Err(format!(
            "rerank response column `{RERANK_RESPONSE_RANK_COLUMN}` must be Int32"
        ));
    }

    Ok(())
}

/// Validate the stable rerank response payload semantics for one decoded batch.
///
/// # Errors
///
/// Returns an error when the rerank response batch contains blank or duplicate
/// document IDs, contains non-finite or out-of-range final scores,
/// or contains non-positive or duplicate rank values.
#[cfg(feature = "julia")]
pub fn validate_rerank_response_batch(batch: &RecordBatch) -> Result<(), String> {
    validate_rerank_response_schema(batch.schema().as_ref())?;
    if batch.num_rows() == 0 {
        return Ok(());
    }

    validate_rerank_response_doc_ids(batch)?;
    validate_rerank_response_score_column(batch, RERANK_RESPONSE_VECTOR_SCORE_COLUMN)?;
    validate_rerank_response_score_column(batch, RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN)?;
    validate_rerank_response_score_column(batch, RERANK_RESPONSE_FINAL_SCORE_COLUMN)?;
    validate_rerank_response_ranks(batch)
}

#[cfg(feature = "julia")]
fn validate_rerank_response_doc_ids(batch: &RecordBatch) -> Result<(), String> {
    let doc_ids = batch
        .column_by_name(RERANK_RESPONSE_DOC_ID_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<StringArray>())
        .ok_or_else(|| {
            format!("rerank response column `{RERANK_RESPONSE_DOC_ID_COLUMN}` must decode as Utf8")
        })?;
    let mut seen_doc_ids = HashSet::new();
    for row_index in 0..batch.num_rows() {
        let doc_id = doc_ids.value(row_index).trim();
        if doc_id.is_empty() {
            return Err(format!(
                "rerank response column `{RERANK_RESPONSE_DOC_ID_COLUMN}` must not contain blank values; row {row_index} is blank"
            ));
        }
        if !seen_doc_ids.insert(doc_id.to_string()) {
            return Err(format!(
                "rerank response column `{RERANK_RESPONSE_DOC_ID_COLUMN}` must be unique across one batch; row {row_index} duplicates `{doc_id}`"
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "julia")]
fn validate_rerank_response_score_column(
    batch: &RecordBatch,
    column_name: &'static str,
) -> Result<(), String> {
    let scores = batch
        .column_by_name(column_name)
        .and_then(|column| column.as_any().downcast_ref::<Float64Array>())
        .ok_or_else(|| format!("rerank response column `{column_name}` must decode as Float64"))?;
    for row_index in 0..batch.num_rows() {
        let score = scores.value(row_index);
        if !score.is_finite() {
            return Err(format!(
                "rerank response column `{column_name}` must contain finite values; row {row_index} is {score}"
            ));
        }
        if !(0.0..=1.0).contains(&score) {
            return Err(format!(
                "rerank response column `{column_name}` must stay within inclusive range [0.0, 1.0]; row {row_index} is {score}"
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "julia")]
fn validate_rerank_response_ranks(batch: &RecordBatch) -> Result<(), String> {
    let ranks = batch
        .column_by_name(RERANK_RESPONSE_RANK_COLUMN)
        .and_then(|column| column.as_any().downcast_ref::<Int32Array>())
        .ok_or_else(|| {
            format!("rerank response column `{RERANK_RESPONSE_RANK_COLUMN}` must decode as Int32")
        })?;
    let mut seen_ranks = HashSet::new();
    for row_index in 0..batch.num_rows() {
        let rank = ranks.value(row_index);
        if rank <= 0 {
            return Err(format!(
                "rerank response column `{RERANK_RESPONSE_RANK_COLUMN}` must contain positive values; row {row_index} is {rank}"
            ));
        }
        if !seen_ranks.insert(rank) {
            return Err(format!(
                "rerank response column `{RERANK_RESPONSE_RANK_COLUMN}` must be unique across one batch; row {row_index} duplicates `{rank}`"
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "julia")]
fn validate_embedding_field(
    schema: &Schema,
    field_name: &str,
    expected_dimension: usize,
) -> Result<(), String> {
    let field = schema
        .field_with_name(field_name)
        .map_err(|_| format!("missing rerank request column `{field_name}`"))?;
    match field.data_type() {
        DataType::FixedSizeList(inner_field, dimension)
            if matches!(inner_field.data_type(), DataType::Float32)
                && usize::try_from(*dimension).ok() == Some(expected_dimension) =>
        {
            Ok(())
        }
        DataType::FixedSizeList(inner_field, dimension)
            if matches!(inner_field.data_type(), DataType::Float32) =>
        {
            Err(format!(
                "rerank request column `{field_name}` must use dimension {expected_dimension}, got {dimension}"
            ))
        }
        _ => Err(format!(
            "rerank request column `{field_name}` must be FixedSizeList<Float32>"
        )),
    }
}

#[cfg(feature = "julia")]
fn fixed_size_list_row_values(
    array: &FixedSizeListArray,
    row_index: usize,
) -> Result<Vec<f32>, String> {
    let row = array.value(row_index);
    let values = row.as_any().downcast_ref::<Float32Array>().ok_or_else(|| {
        "rerank request fixed-size-list values must decode as Float32".to_string()
    })?;
    Ok((0..values.len()).map(|index| values.value(index)).collect())
}

#[cfg(feature = "julia")]
fn cosine_similarity(left: &[f32], right: &[f32], row_index: usize) -> Result<f64, String> {
    let left_norm = left
        .iter()
        .map(|value| f64::from(*value) * f64::from(*value))
        .sum::<f64>()
        .sqrt();
    if left_norm == 0.0 {
        return Err(format!(
            "rerank request column `{RERANK_REQUEST_EMBEDDING_COLUMN}` must not contain zero-norm vectors; row {row_index} is zero"
        ));
    }

    let right_norm = right
        .iter()
        .map(|value| f64::from(*value) * f64::from(*value))
        .sum::<f64>()
        .sqrt();
    if right_norm == 0.0 {
        return Err(format!(
            "rerank request column `{RERANK_REQUEST_QUERY_EMBEDDING_COLUMN}` must not contain zero-norm vectors; row {row_index} is zero"
        ));
    }

    let dot = left
        .iter()
        .zip(right.iter())
        .map(|(left_value, right_value)| f64::from(*left_value) * f64::from(*right_value))
        .sum::<f64>();
    Ok((dot / (left_norm * right_norm)).clamp(-1.0, 1.0))
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "julia")]
    use std::fmt::Display;

    #[cfg(feature = "julia")]
    use super::validate_sql_query_request;
    use super::{
        ANALYSIS_CODE_AST_ROUTE, ANALYSIS_MARKDOWN_ROUTE, ANALYSIS_REFINE_DOC_ROUTE,
        ANALYSIS_REPO_DOC_COVERAGE_ROUTE, ANALYSIS_REPO_INDEX_ROUTE,
        ANALYSIS_REPO_INDEX_STATUS_ROUTE, ANALYSIS_REPO_OVERVIEW_ROUTE,
        ANALYSIS_REPO_PROJECTED_PAGE_INDEX_TREE_ROUTE, ANALYSIS_REPO_SYNC_ROUTE,
        GRAPH_NEIGHBORS_DEFAULT_HOPS, GRAPH_NEIGHBORS_DEFAULT_LIMIT, GRAPH_NEIGHBORS_ROUTE,
        QUERY_SQL_ROUTE, REPO_SEARCH_DEFAULT_LIMIT, REPO_SEARCH_DOC_ID_COLUMN,
        REPO_SEARCH_LANGUAGE_COLUMN, REPO_SEARCH_PATH_COLUMN, REPO_SEARCH_ROUTE,
        REPO_SEARCH_SCORE_COLUMN, REPO_SEARCH_TITLE_COLUMN, RERANK_REQUEST_DOC_ID_COLUMN,
        RERANK_REQUEST_EMBEDDING_COLUMN, RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
        RERANK_REQUEST_VECTOR_SCORE_COLUMN, RERANK_RESPONSE_DOC_ID_COLUMN,
        RERANK_RESPONSE_FINAL_SCORE_COLUMN, RERANK_RESPONSE_RANK_COLUMN,
        RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN, RERANK_RESPONSE_VECTOR_SCORE_COLUMN, RERANK_ROUTE,
        SEARCH_AST_ROUTE, SEARCH_ATTACHMENTS_ROUTE, SEARCH_AUTOCOMPLETE_ROUTE,
        SEARCH_DEFINITION_ROUTE, SEARCH_INTENT_ROUTE, SEARCH_KNOWLEDGE_ROUTE,
        SEARCH_REFERENCES_ROUTE, SEARCH_SYMBOLS_ROUTE, VFS_CONTENT_ROUTE, VFS_RESOLVE_ROUTE,
        WENDAO_ANALYSIS_LINE_HEADER, WENDAO_ANALYSIS_PATH_HEADER, WENDAO_ANALYSIS_REPO_HEADER,
        WENDAO_ATTACHMENT_SEARCH_CASE_SENSITIVE_HEADER,
        WENDAO_ATTACHMENT_SEARCH_EXT_FILTERS_HEADER, WENDAO_ATTACHMENT_SEARCH_KIND_FILTERS_HEADER,
        WENDAO_AUTOCOMPLETE_LIMIT_HEADER, WENDAO_AUTOCOMPLETE_PREFIX_HEADER,
        WENDAO_DEFINITION_LINE_HEADER, WENDAO_DEFINITION_PATH_HEADER,
        WENDAO_DEFINITION_QUERY_HEADER, WENDAO_GRAPH_DIRECTION_HEADER, WENDAO_GRAPH_HOPS_HEADER,
        WENDAO_GRAPH_LIMIT_HEADER, WENDAO_GRAPH_NODE_ID_HEADER, WENDAO_REFINE_DOC_ENTITY_ID_HEADER,
        WENDAO_REFINE_DOC_REPO_HEADER, WENDAO_REFINE_DOC_USER_HINTS_HEADER,
        WENDAO_REPO_DOC_COVERAGE_MODULE_HEADER, WENDAO_REPO_DOC_COVERAGE_REPO_HEADER,
        WENDAO_REPO_INDEX_REFRESH_HEADER, WENDAO_REPO_INDEX_REPO_HEADER,
        WENDAO_REPO_INDEX_REQUEST_ID_HEADER, WENDAO_REPO_INDEX_STATUS_REPO_HEADER,
        WENDAO_REPO_OVERVIEW_REPO_HEADER, WENDAO_REPO_PROJECTED_PAGE_INDEX_TREE_PAGE_ID_HEADER,
        WENDAO_REPO_PROJECTED_PAGE_INDEX_TREE_REPO_HEADER,
        WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER, WENDAO_REPO_SEARCH_LIMIT_HEADER,
        WENDAO_REPO_SEARCH_QUERY_HEADER, WENDAO_REPO_SEARCH_REPO_HEADER,
        WENDAO_REPO_SYNC_MODE_HEADER, WENDAO_REPO_SYNC_REPO_HEADER, WENDAO_RERANK_DIMENSION_HEADER,
        WENDAO_SCHEMA_VERSION_HEADER, WENDAO_SEARCH_LIMIT_HEADER, WENDAO_SEARCH_QUERY_HEADER,
        WENDAO_SQL_QUERY_HEADER, WENDAO_VFS_PATH_HEADER, flight_descriptor_path,
        normalize_flight_route, validate_attachment_search_request, validate_autocomplete_request,
        validate_code_ast_analysis_request, validate_definition_request,
        validate_graph_neighbors_request, validate_markdown_analysis_request,
        validate_refine_doc_request, validate_repo_doc_coverage_request,
        validate_repo_index_request, validate_repo_index_status_request,
        validate_repo_overview_request, validate_repo_projected_page_index_tree_request,
        validate_repo_search_request, validate_repo_sync_request, validate_vfs_content_request,
        validate_vfs_resolve_request,
    };

    #[cfg(feature = "julia")]
    fn must_ok<T, E: Display>(result: Result<T, E>, context: &str) -> T {
        result.unwrap_or_else(|error| panic!("{context}: {error}"))
    }

    #[cfg(feature = "julia")]
    fn must_err<T, E>(result: Result<T, E>, context: &str) -> E {
        match result {
            Ok(_) => panic!("{context}"),
            Err(error) => error,
        }
    }

    #[test]
    fn query_contract_exposes_stable_routes_and_header() {
        assert_eq!(WENDAO_SCHEMA_VERSION_HEADER, "x-wendao-schema-version");
        assert_eq!(
            WENDAO_REPO_SEARCH_QUERY_HEADER,
            "x-wendao-repo-search-query"
        );
        assert_eq!(
            WENDAO_REPO_SEARCH_LIMIT_HEADER,
            "x-wendao-repo-search-limit"
        );
        assert_eq!(WENDAO_REPO_SEARCH_REPO_HEADER, "x-wendao-repo-search-repo");
        assert_eq!(
            WENDAO_REPO_DOC_COVERAGE_REPO_HEADER,
            "x-wendao-repo-doc-coverage-repo"
        );
        assert_eq!(
            WENDAO_REPO_DOC_COVERAGE_MODULE_HEADER,
            "x-wendao-repo-doc-coverage-module"
        );
        assert_eq!(WENDAO_REPO_INDEX_REPO_HEADER, "x-wendao-repo-index-repo");
        assert_eq!(
            WENDAO_REPO_INDEX_REFRESH_HEADER,
            "x-wendao-repo-index-refresh"
        );
        assert_eq!(
            WENDAO_REPO_INDEX_REQUEST_ID_HEADER,
            "x-wendao-repo-index-request-id"
        );
        assert_eq!(WENDAO_REPO_SYNC_REPO_HEADER, "x-wendao-repo-sync-repo");
        assert_eq!(WENDAO_REPO_SYNC_MODE_HEADER, "x-wendao-repo-sync-mode");
        assert_eq!(
            WENDAO_REPO_PROJECTED_PAGE_INDEX_TREE_REPO_HEADER,
            "x-wendao-repo-projected-page-index-tree-repo"
        );
        assert_eq!(
            WENDAO_REPO_PROJECTED_PAGE_INDEX_TREE_PAGE_ID_HEADER,
            "x-wendao-repo-projected-page-index-tree-page-id"
        );
        assert_eq!(
            WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER,
            "x-wendao-repo-search-language-filters"
        );
        assert_eq!(
            WENDAO_RERANK_DIMENSION_HEADER,
            "x-wendao-rerank-embedding-dimension"
        );
        assert_eq!(WENDAO_SEARCH_QUERY_HEADER, "x-wendao-search-query");
        assert_eq!(WENDAO_SEARCH_LIMIT_HEADER, "x-wendao-search-limit");
        assert_eq!(WENDAO_SQL_QUERY_HEADER, "x-wendao-sql-query");
        assert_eq!(WENDAO_DEFINITION_QUERY_HEADER, "x-wendao-definition-query");
        assert_eq!(WENDAO_DEFINITION_PATH_HEADER, "x-wendao-definition-path");
        assert_eq!(WENDAO_DEFINITION_LINE_HEADER, "x-wendao-definition-line");
        assert_eq!(
            WENDAO_AUTOCOMPLETE_PREFIX_HEADER,
            "x-wendao-autocomplete-prefix"
        );
        assert_eq!(
            WENDAO_AUTOCOMPLETE_LIMIT_HEADER,
            "x-wendao-autocomplete-limit"
        );
        assert_eq!(WENDAO_VFS_PATH_HEADER, "x-wendao-vfs-path");
        assert_eq!(WENDAO_GRAPH_NODE_ID_HEADER, "x-wendao-graph-node-id");
        assert_eq!(WENDAO_GRAPH_DIRECTION_HEADER, "x-wendao-graph-direction");
        assert_eq!(WENDAO_GRAPH_HOPS_HEADER, "x-wendao-graph-hops");
        assert_eq!(WENDAO_GRAPH_LIMIT_HEADER, "x-wendao-graph-limit");
        assert_eq!(WENDAO_ANALYSIS_PATH_HEADER, "x-wendao-analysis-path");
        assert_eq!(WENDAO_ANALYSIS_REPO_HEADER, "x-wendao-analysis-repo");
        assert_eq!(WENDAO_ANALYSIS_LINE_HEADER, "x-wendao-analysis-line");
        assert_eq!(
            WENDAO_ATTACHMENT_SEARCH_EXT_FILTERS_HEADER,
            "x-wendao-attachment-search-ext-filters"
        );
        assert_eq!(
            WENDAO_ATTACHMENT_SEARCH_KIND_FILTERS_HEADER,
            "x-wendao-attachment-search-kind-filters"
        );
        assert_eq!(
            WENDAO_ATTACHMENT_SEARCH_CASE_SENSITIVE_HEADER,
            "x-wendao-attachment-search-case-sensitive"
        );
        assert_eq!(REPO_SEARCH_ROUTE, "/search/repos/main");
        assert_eq!(SEARCH_INTENT_ROUTE, "/search/intent");
        assert_eq!(SEARCH_KNOWLEDGE_ROUTE, "/search/knowledge");
        assert_eq!(SEARCH_ATTACHMENTS_ROUTE, "/search/attachments");
        assert_eq!(SEARCH_AST_ROUTE, "/search/ast");
        assert_eq!(SEARCH_REFERENCES_ROUTE, "/search/references");
        assert_eq!(SEARCH_SYMBOLS_ROUTE, "/search/symbols");
        assert_eq!(SEARCH_DEFINITION_ROUTE, "/search/definition");
        assert_eq!(SEARCH_AUTOCOMPLETE_ROUTE, "/search/autocomplete");
        assert_eq!(QUERY_SQL_ROUTE, "/query/sql");
        assert_eq!(VFS_RESOLVE_ROUTE, "/vfs/resolve");
        assert_eq!(VFS_CONTENT_ROUTE, "/vfs/content");
        assert_eq!(GRAPH_NEIGHBORS_ROUTE, "/graph/neighbors");
        assert_eq!(ANALYSIS_MARKDOWN_ROUTE, "/analysis/markdown");
        assert_eq!(ANALYSIS_CODE_AST_ROUTE, "/analysis/code-ast");
        assert_eq!(ANALYSIS_REPO_INDEX_ROUTE, "/analysis/repo-index");
        assert_eq!(ANALYSIS_REPO_SYNC_ROUTE, "/analysis/repo-sync");
        assert_eq!(
            ANALYSIS_REPO_PROJECTED_PAGE_INDEX_TREE_ROUTE,
            "/analysis/repo-projected-page-index-tree"
        );
        assert_eq!(RERANK_ROUTE, "/rerank");
        assert_eq!(REPO_SEARCH_DEFAULT_LIMIT, 10);
        assert_eq!(GRAPH_NEIGHBORS_DEFAULT_HOPS, 2);
        assert_eq!(GRAPH_NEIGHBORS_DEFAULT_LIMIT, 50);
        assert_eq!(REPO_SEARCH_DOC_ID_COLUMN, "doc_id");
        assert_eq!(REPO_SEARCH_PATH_COLUMN, "path");
        assert_eq!(REPO_SEARCH_TITLE_COLUMN, "title");
        assert_eq!(REPO_SEARCH_SCORE_COLUMN, "score");
        assert_eq!(REPO_SEARCH_LANGUAGE_COLUMN, "language");
        assert_eq!(RERANK_REQUEST_DOC_ID_COLUMN, "doc_id");
        assert_eq!(RERANK_REQUEST_VECTOR_SCORE_COLUMN, "vector_score");
        assert_eq!(RERANK_REQUEST_EMBEDDING_COLUMN, "embedding");
        assert_eq!(RERANK_REQUEST_QUERY_EMBEDDING_COLUMN, "query_embedding");
        assert_eq!(RERANK_RESPONSE_DOC_ID_COLUMN, "doc_id");
        assert_eq!(RERANK_RESPONSE_VECTOR_SCORE_COLUMN, "vector_score");
        assert_eq!(RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN, "semantic_score");
        assert_eq!(RERANK_RESPONSE_FINAL_SCORE_COLUMN, "final_score");
        assert_eq!(RERANK_RESPONSE_RANK_COLUMN, "rank");
    }

    #[test]
    fn normalize_flight_route_enforces_canonical_leading_slash() {
        assert_eq!(
            normalize_flight_route("search/repos/main").as_deref(),
            Ok("/search/repos/main")
        );
        assert_eq!(normalize_flight_route("/rerank").as_deref(), Ok("/rerank"));
    }

    #[test]
    fn normalize_flight_route_rejects_empty_segments() {
        assert!(normalize_flight_route("").is_err());
        assert!(normalize_flight_route("/").is_err());
    }

    #[test]
    fn descriptor_path_matches_stable_query_route() {
        assert_eq!(
            flight_descriptor_path(REPO_SEARCH_ROUTE),
            Ok(vec![
                "search".to_string(),
                "repos".to_string(),
                "main".to_string()
            ])
        );
        assert_eq!(
            flight_descriptor_path(SEARCH_INTENT_ROUTE),
            Ok(vec!["search".to_string(), "intent".to_string()])
        );
        assert_eq!(
            flight_descriptor_path(SEARCH_KNOWLEDGE_ROUTE),
            Ok(vec!["search".to_string(), "knowledge".to_string()])
        );
        assert_eq!(
            flight_descriptor_path(SEARCH_ATTACHMENTS_ROUTE),
            Ok(vec!["search".to_string(), "attachments".to_string()])
        );
        assert_eq!(
            flight_descriptor_path(SEARCH_AST_ROUTE),
            Ok(vec!["search".to_string(), "ast".to_string()])
        );
        assert_eq!(
            flight_descriptor_path(SEARCH_REFERENCES_ROUTE),
            Ok(vec!["search".to_string(), "references".to_string()])
        );
        assert_eq!(
            flight_descriptor_path(SEARCH_SYMBOLS_ROUTE),
            Ok(vec!["search".to_string(), "symbols".to_string()])
        );
        assert_eq!(
            flight_descriptor_path(SEARCH_DEFINITION_ROUTE),
            Ok(vec!["search".to_string(), "definition".to_string()])
        );
        assert_eq!(
            flight_descriptor_path(SEARCH_AUTOCOMPLETE_ROUTE),
            Ok(vec!["search".to_string(), "autocomplete".to_string()])
        );
        assert_eq!(
            flight_descriptor_path(QUERY_SQL_ROUTE),
            Ok(vec!["query".to_string(), "sql".to_string()])
        );
        assert_eq!(
            flight_descriptor_path(VFS_RESOLVE_ROUTE),
            Ok(vec!["vfs".to_string(), "resolve".to_string()])
        );
        assert_eq!(
            flight_descriptor_path(VFS_CONTENT_ROUTE),
            Ok(vec!["vfs".to_string(), "content".to_string()])
        );
        assert_eq!(
            flight_descriptor_path(GRAPH_NEIGHBORS_ROUTE),
            Ok(vec!["graph".to_string(), "neighbors".to_string()])
        );
        assert_eq!(
            flight_descriptor_path(ANALYSIS_MARKDOWN_ROUTE),
            Ok(vec!["analysis".to_string(), "markdown".to_string()])
        );
        assert_eq!(
            flight_descriptor_path(ANALYSIS_CODE_AST_ROUTE),
            Ok(vec!["analysis".to_string(), "code-ast".to_string()])
        );
        assert_eq!(
            flight_descriptor_path(ANALYSIS_REPO_INDEX_ROUTE),
            Ok(vec!["analysis".to_string(), "repo-index".to_string()])
        );
        assert_eq!(
            flight_descriptor_path(ANALYSIS_REPO_SYNC_ROUTE),
            Ok(vec!["analysis".to_string(), "repo-sync".to_string()])
        );
        assert_eq!(
            flight_descriptor_path(ANALYSIS_REPO_PROJECTED_PAGE_INDEX_TREE_ROUTE),
            Ok(vec![
                "analysis".to_string(),
                "repo-projected-page-index-tree".to_string()
            ])
        );
        assert_eq!(
            flight_descriptor_path(RERANK_ROUTE),
            Ok(vec!["rerank".to_string()])
        );
    }

    #[test]
    fn repo_search_request_validation_accepts_stable_request() {
        assert!(
            validate_repo_search_request("rerank rust traits", 25, &[], &[], &[], &[], &[]).is_ok()
        );
    }

    #[test]
    fn repo_search_request_validation_rejects_blank_query_text() {
        assert_eq!(
            validate_repo_search_request("   ", REPO_SEARCH_DEFAULT_LIMIT, &[], &[], &[], &[], &[],),
            Err("repo search query text must not be blank".to_string())
        );
    }

    #[test]
    fn repo_search_request_validation_rejects_zero_limit() {
        assert_eq!(
            validate_repo_search_request("rerank rust traits", 0, &[], &[], &[], &[], &[]),
            Err("repo search limit must be greater than zero".to_string())
        );
    }

    #[test]
    fn attachment_search_request_validation_accepts_stable_request() {
        assert!(
            validate_attachment_search_request(
                "screenshot",
                REPO_SEARCH_DEFAULT_LIMIT,
                &["png".to_string()],
                &["image".to_string()],
            )
            .is_ok()
        );
    }

    #[test]
    fn attachment_search_request_validation_rejects_blank_extension_filters() {
        assert_eq!(
            validate_attachment_search_request(
                "screenshot",
                REPO_SEARCH_DEFAULT_LIMIT,
                &["png".to_string(), "   ".to_string()],
                &[],
            ),
            Err("attachment search extension filters must not contain blank values".to_string())
        );
    }

    #[test]
    fn attachment_search_request_validation_rejects_blank_kind_filters() {
        assert_eq!(
            validate_attachment_search_request(
                "screenshot",
                REPO_SEARCH_DEFAULT_LIMIT,
                &[],
                &["image".to_string(), "   ".to_string()],
            ),
            Err("attachment search kind filters must not contain blank values".to_string())
        );
    }

    #[test]
    fn markdown_analysis_request_validation_accepts_stable_request() {
        assert!(validate_markdown_analysis_request("docs/analysis.md").is_ok());
    }

    #[test]
    fn repo_doc_coverage_route_constant_is_stable() {
        assert_eq!(
            ANALYSIS_REPO_DOC_COVERAGE_ROUTE,
            "/analysis/repo-doc-coverage"
        );
    }

    #[test]
    fn repo_overview_route_constant_and_header_are_stable() {
        assert_eq!(ANALYSIS_REPO_OVERVIEW_ROUTE, "/analysis/repo-overview");
        assert_eq!(
            WENDAO_REPO_OVERVIEW_REPO_HEADER,
            "x-wendao-repo-overview-repo"
        );
    }

    #[test]
    fn repo_index_status_route_constant_and_header_are_stable() {
        assert_eq!(
            ANALYSIS_REPO_INDEX_STATUS_ROUTE,
            "/analysis/repo-index-status"
        );
        assert_eq!(
            WENDAO_REPO_INDEX_STATUS_REPO_HEADER,
            "x-wendao-repo-index-status-repo"
        );
    }

    #[test]
    fn repo_index_route_constants_and_headers_are_stable() {
        assert_eq!(ANALYSIS_REPO_INDEX_ROUTE, "/analysis/repo-index");
        assert_eq!(WENDAO_REPO_INDEX_REPO_HEADER, "x-wendao-repo-index-repo");
        assert_eq!(
            WENDAO_REPO_INDEX_REFRESH_HEADER,
            "x-wendao-repo-index-refresh"
        );
        assert_eq!(
            WENDAO_REPO_INDEX_REQUEST_ID_HEADER,
            "x-wendao-repo-index-request-id"
        );
    }

    #[test]
    fn repo_projected_page_index_tree_route_constant_is_stable() {
        assert_eq!(
            ANALYSIS_REPO_PROJECTED_PAGE_INDEX_TREE_ROUTE,
            "/analysis/repo-projected-page-index-tree"
        );
    }

    #[test]
    fn refine_doc_route_constant_and_headers_are_stable() {
        assert_eq!(ANALYSIS_REFINE_DOC_ROUTE, "/analysis/refine-doc");
        assert_eq!(WENDAO_REFINE_DOC_REPO_HEADER, "x-wendao-refine-doc-repo");
        assert_eq!(
            WENDAO_REFINE_DOC_ENTITY_ID_HEADER,
            "x-wendao-refine-doc-entity-id"
        );
        assert_eq!(
            WENDAO_REFINE_DOC_USER_HINTS_HEADER,
            "x-wendao-refine-doc-user-hints-b64"
        );
    }

    #[test]
    fn definition_request_validation_accepts_stable_request() {
        assert!(validate_definition_request("AlphaService", Some("src/lib.rs"), Some(7)).is_ok());
    }

    #[test]
    fn definition_request_validation_rejects_blank_query() {
        assert_eq!(
            validate_definition_request("   ", Some("src/lib.rs"), Some(7)),
            Err("definition query text must not be blank".to_string())
        );
    }

    #[test]
    fn definition_request_validation_rejects_blank_source_path() {
        assert_eq!(
            validate_definition_request("AlphaService", Some("   "), Some(7)),
            Err("definition source path must not be blank".to_string())
        );
    }

    #[test]
    fn definition_request_validation_rejects_zero_source_line() {
        assert_eq!(
            validate_definition_request("AlphaService", Some("src/lib.rs"), Some(0)),
            Err("definition source line must be greater than zero".to_string())
        );
    }

    #[test]
    fn autocomplete_request_validation_accepts_stable_request() {
        assert!(validate_autocomplete_request("Alpha", 5).is_ok());
        assert!(validate_autocomplete_request("", 5).is_ok());
    }

    #[test]
    fn autocomplete_request_validation_rejects_zero_limit() {
        assert_eq!(
            validate_autocomplete_request("Alpha", 0),
            Err("autocomplete limit must be greater than zero".to_string())
        );
    }

    #[test]
    fn autocomplete_request_validation_rejects_blank_prefix() {
        assert_eq!(
            validate_autocomplete_request("   ", 5),
            Err("autocomplete prefix must not be blank".to_string())
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn sql_query_request_validation_accepts_read_only_query() {
        assert!(validate_sql_query_request("SELECT doc_id FROM repo_entity").is_ok());
    }

    #[cfg(feature = "julia")]
    #[test]
    fn sql_query_request_validation_rejects_blank_query() {
        assert_eq!(
            validate_sql_query_request("   "),
            Err("SQL query text must not be blank".to_string())
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn sql_query_request_validation_rejects_multiple_statements() {
        assert_eq!(
            validate_sql_query_request("SELECT 1; SELECT 2"),
            Err("SQL query text must contain exactly one statement".to_string())
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn sql_query_request_validation_rejects_non_query_statement() {
        assert_eq!(
            validate_sql_query_request("CREATE VIEW demo AS SELECT 1"),
            Err("SQL query text must be a read-only query statement".to_string())
        );
    }

    #[test]
    fn vfs_resolve_request_validation_accepts_stable_request() {
        assert!(validate_vfs_resolve_request("main/docs/index.md").is_ok());
    }

    #[test]
    fn vfs_resolve_request_validation_rejects_blank_path() {
        assert_eq!(
            validate_vfs_resolve_request("   "),
            Err("VFS resolve requires a non-empty path".to_string())
        );
    }

    #[test]
    fn vfs_content_request_validation_accepts_stable_request() {
        assert!(validate_vfs_content_request("main/docs/index.md").is_ok());
    }

    #[test]
    fn vfs_content_request_validation_rejects_blank_path() {
        assert_eq!(
            validate_vfs_content_request("   "),
            Err("VFS content requires a non-empty path".to_string())
        );
    }

    #[test]
    fn graph_neighbors_request_validation_accepts_canonical_request() {
        assert_eq!(
            validate_graph_neighbors_request(
                "kernel/docs/index.md",
                Some("outgoing"),
                Some(3),
                Some(25),
            ),
            Ok((
                "kernel/docs/index.md".to_string(),
                "outgoing".to_string(),
                3,
                25,
            ))
        );
    }

    #[test]
    fn graph_neighbors_request_validation_normalizes_defaults_and_clamps_bounds() {
        assert_eq!(
            validate_graph_neighbors_request(
                "kernel/docs/index.md",
                Some("invalid"),
                Some(0),
                Some(999)
            ),
            Ok((
                "kernel/docs/index.md".to_string(),
                "both".to_string(),
                1,
                300,
            ))
        );
        assert_eq!(
            validate_graph_neighbors_request("kernel/docs/index.md", None, None, None),
            Ok((
                "kernel/docs/index.md".to_string(),
                "both".to_string(),
                GRAPH_NEIGHBORS_DEFAULT_HOPS,
                GRAPH_NEIGHBORS_DEFAULT_LIMIT,
            ))
        );
    }

    #[test]
    fn graph_neighbors_request_validation_rejects_blank_node_id() {
        assert_eq!(
            validate_graph_neighbors_request("   ", Some("both"), Some(2), Some(20)),
            Err("graph neighbors requires a non-empty node id".to_string())
        );
    }

    #[test]
    fn markdown_analysis_request_validation_rejects_blank_path() {
        assert_eq!(
            validate_markdown_analysis_request("   "),
            Err("markdown analysis path must not be blank".to_string())
        );
    }

    #[test]
    fn code_ast_analysis_request_validation_accepts_stable_request() {
        assert!(validate_code_ast_analysis_request("src/lib.jl", "demo", Some(7)).is_ok());
    }

    #[test]
    fn code_ast_analysis_request_validation_rejects_blank_repo() {
        assert_eq!(
            validate_code_ast_analysis_request("src/lib.jl", "   ", Some(7)),
            Err("code AST analysis repo must not be blank".to_string())
        );
    }

    #[test]
    fn code_ast_analysis_request_validation_rejects_zero_line_hint() {
        assert_eq!(
            validate_code_ast_analysis_request("src/lib.jl", "demo", Some(0)),
            Err("code AST analysis line hint must be greater than zero".to_string())
        );
    }

    #[test]
    fn repo_doc_coverage_request_validation_accepts_stable_request() {
        assert_eq!(
            validate_repo_doc_coverage_request("gateway-sync", Some("GatewaySyncPkg")),
            Ok((
                "gateway-sync".to_string(),
                Some("GatewaySyncPkg".to_string()),
            ))
        );
        assert_eq!(
            validate_repo_doc_coverage_request("gateway-sync", Some("   ")),
            Ok(("gateway-sync".to_string(), None))
        );
    }

    #[test]
    fn repo_overview_request_validation_accepts_stable_request() {
        assert_eq!(
            validate_repo_overview_request("gateway-sync"),
            Ok("gateway-sync".to_string())
        );
    }

    #[test]
    fn repo_index_status_request_validation_accepts_stable_request() {
        assert_eq!(
            validate_repo_index_status_request(Some("gateway-sync")),
            Ok(Some("gateway-sync".to_string()))
        );
        assert_eq!(validate_repo_index_status_request(Some("   ")), Ok(None));
        assert_eq!(validate_repo_index_status_request(None), Ok(None));
    }

    #[test]
    fn repo_index_request_validation_accepts_stable_request() {
        assert_eq!(
            validate_repo_index_request(Some("gateway-sync"), Some("true"), "req-123"),
            Ok((
                Some("gateway-sync".to_string()),
                true,
                "req-123".to_string()
            ))
        );
        assert_eq!(
            validate_repo_index_request(Some("   "), None, "req-456"),
            Ok((None, false, "req-456".to_string()))
        );
    }

    #[test]
    fn repo_sync_request_validation_accepts_stable_request() {
        assert_eq!(
            validate_repo_sync_request("gateway-sync", Some("status")),
            Ok(("gateway-sync".to_string(), "status".to_string()))
        );
        assert_eq!(
            validate_repo_sync_request("gateway-sync", Some("   ")),
            Ok(("gateway-sync".to_string(), "ensure".to_string()))
        );
        assert_eq!(
            validate_repo_sync_request("gateway-sync", None),
            Ok(("gateway-sync".to_string(), "ensure".to_string()))
        );
    }

    #[test]
    fn repo_overview_request_validation_rejects_blank_repo() {
        assert_eq!(
            validate_repo_overview_request("   "),
            Err("repo overview repo must not be blank".to_string())
        );
    }

    #[test]
    fn repo_doc_coverage_request_validation_rejects_blank_repo() {
        assert_eq!(
            validate_repo_doc_coverage_request("   ", Some("GatewaySyncPkg")),
            Err("repo doc coverage repo must not be blank".to_string())
        );
    }

    #[test]
    fn repo_sync_request_validation_rejects_blank_repo() {
        assert_eq!(
            validate_repo_sync_request("   ", Some("status")),
            Err("repo sync repo must not be blank".to_string())
        );
    }

    #[test]
    fn repo_sync_request_validation_rejects_invalid_mode() {
        assert_eq!(
            validate_repo_sync_request("gateway-sync", Some("bogus")),
            Err("unsupported repo sync mode `bogus`".to_string())
        );
    }

    #[test]
    fn repo_index_request_validation_rejects_invalid_refresh_flag() {
        assert_eq!(
            validate_repo_index_request(Some("gateway-sync"), Some("bogus"), "req-123"),
            Err("unsupported repo index refresh flag `bogus`".to_string())
        );
    }

    #[test]
    fn repo_index_request_validation_rejects_blank_request_id() {
        assert_eq!(
            validate_repo_index_request(Some("gateway-sync"), Some("false"), "   "),
            Err("repo index request id must not be blank".to_string())
        );
    }

    #[test]
    fn repo_projected_page_index_tree_request_validation_accepts_stable_request() {
        assert_eq!(
            validate_repo_projected_page_index_tree_request(
                "gateway-sync",
                "repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md"
            ),
            Ok((
                "gateway-sync".to_string(),
                "repo:gateway-sync:projection:reference:doc:repo:gateway-sync:doc:docs/solve.md"
                    .to_string(),
            ))
        );
    }

    #[test]
    fn repo_projected_page_index_tree_request_validation_rejects_blank_repo() {
        assert_eq!(
            validate_repo_projected_page_index_tree_request("   ", "repo:gateway-sync:page"),
            Err("repo projected page-index tree repo must not be blank".to_string())
        );
    }

    #[test]
    fn repo_projected_page_index_tree_request_validation_rejects_blank_page_id() {
        assert_eq!(
            validate_repo_projected_page_index_tree_request("gateway-sync", "   "),
            Err("repo projected page-index tree page id must not be blank".to_string())
        );
    }

    #[test]
    fn refine_doc_request_validation_accepts_base64_user_hints() {
        assert_eq!(
            validate_refine_doc_request(
                "gateway-sync",
                "repo:gateway-sync:symbol:GatewaySyncPkg.solve",
                Some("RXhwbGFpbiB0aGlzIGVudHJ5cG9pbnQ="),
            ),
            Ok((
                "gateway-sync".to_string(),
                "repo:gateway-sync:symbol:GatewaySyncPkg.solve".to_string(),
                Some("Explain this entrypoint".to_string()),
            ))
        );
        assert_eq!(
            validate_refine_doc_request(
                "gateway-sync",
                "repo:gateway-sync:symbol:GatewaySyncPkg.solve",
                Some("   "),
            ),
            Ok((
                "gateway-sync".to_string(),
                "repo:gateway-sync:symbol:GatewaySyncPkg.solve".to_string(),
                None,
            ))
        );
    }

    #[test]
    fn refine_doc_request_validation_rejects_blank_repo() {
        assert_eq!(
            validate_refine_doc_request(
                "   ",
                "repo:gateway-sync:symbol:GatewaySyncPkg.solve",
                None,
            ),
            Err("refine doc repo must not be blank".to_string())
        );
    }

    #[test]
    fn refine_doc_request_validation_rejects_blank_entity_id() {
        assert_eq!(
            validate_refine_doc_request("gateway-sync", "   ", None),
            Err("refine doc entity_id must not be blank".to_string())
        );
    }

    #[test]
    fn refine_doc_request_validation_rejects_invalid_base64_user_hints() {
        let error = validate_refine_doc_request(
            "gateway-sync",
            "repo:gateway-sync:symbol:GatewaySyncPkg.solve",
            Some("%%%"),
        )
        .expect_err("invalid base64 user hints should fail");
        assert!(error.starts_with("refine doc user_hints must be valid Base64:"));
    }

    #[test]
    fn repo_search_request_validation_rejects_blank_language_filters() {
        assert_eq!(
            validate_repo_search_request(
                "rerank rust traits",
                REPO_SEARCH_DEFAULT_LIMIT,
                &["rust".to_string(), "   ".to_string()],
                &[],
                &[],
                &[],
                &[],
            ),
            Err("repo search language filters must not contain blank values".to_string())
        );
    }

    #[test]
    fn repo_search_request_validation_rejects_blank_path_prefixes() {
        assert_eq!(
            validate_repo_search_request(
                "rerank rust traits",
                REPO_SEARCH_DEFAULT_LIMIT,
                &[],
                &["src/".to_string(), "   ".to_string()],
                &[],
                &[],
                &[],
            ),
            Err("repo search path prefixes must not contain blank values".to_string())
        );
    }

    #[test]
    fn repo_search_request_validation_rejects_blank_title_filters() {
        assert_eq!(
            validate_repo_search_request(
                "rerank rust traits",
                REPO_SEARCH_DEFAULT_LIMIT,
                &[],
                &[],
                &["README".to_string(), "   ".to_string()],
                &[],
                &[],
            ),
            Err("repo search title filters must not contain blank values".to_string())
        );
    }

    #[test]
    fn repo_search_request_validation_rejects_blank_tag_filters() {
        assert_eq!(
            validate_repo_search_request(
                "rerank rust traits",
                REPO_SEARCH_DEFAULT_LIMIT,
                &[],
                &[],
                &[],
                &["lang:rust".to_string(), "   ".to_string()],
                &[],
            ),
            Err("repo search tag filters must not contain blank values".to_string())
        );
    }

    #[test]
    fn repo_search_request_validation_rejects_blank_filename_filters() {
        assert_eq!(
            validate_repo_search_request(
                "rerank rust traits",
                REPO_SEARCH_DEFAULT_LIMIT,
                &[],
                &[],
                &[],
                &[],
                &["lib".to_string(), "   ".to_string()],
            ),
            Err("repo search filename filters must not contain blank values".to_string())
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_request_schema_validation_accepts_stable_shape() {
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
        ]);

        assert!(super::validate_rerank_request_schema(&schema, 3).is_ok());
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_request_schema_validation_rejects_wrong_scalar_type() {
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float64, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
        ]);

        assert_eq!(
            super::validate_rerank_request_schema(&schema, 3),
            Err("rerank request column `vector_score` must be Float32".to_string())
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_request_schema_validation_rejects_dimension_drift() {
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 2),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 2),
                false,
            ),
        ]);

        assert_eq!(
            super::validate_rerank_request_schema(&schema, 3),
            Err("rerank request column `embedding` must use dimension 3, got 2".to_string())
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_request_batch_validation_accepts_stable_semantics() {
        use arrow_array::types::Float32Type;
        use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
        ]));
        let batch = must_ok(
            RecordBatch::try_new(
                schema,
                vec![
                    Arc::new(StringArray::from(vec!["doc-1", "doc-2"])),
                    Arc::new(Float32Array::from(vec![0.9_f32, 0.8_f32])),
                    Arc::new(
                        FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                            vec![
                                Some(vec![Some(0.1_f32), Some(0.2_f32), Some(0.3_f32)]),
                                Some(vec![Some(0.4_f32), Some(0.5_f32), Some(0.6_f32)]),
                            ],
                            3,
                        ),
                    ),
                    Arc::new(
                        FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                            vec![
                                Some(vec![Some(0.7_f32), Some(0.8_f32), Some(0.9_f32)]),
                                Some(vec![Some(0.7_f32), Some(0.8_f32), Some(0.9_f32)]),
                            ],
                            3,
                        ),
                    ),
                ],
            ),
            "record batch should build",
        );

        assert!(super::validate_rerank_request_batch(&batch, 3).is_ok());
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_request_batch_validation_rejects_blank_doc_id() {
        use arrow_array::types::Float32Type;
        use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
        ]));
        let batch = must_ok(
            RecordBatch::try_new(
                schema,
                vec![
                    Arc::new(StringArray::from(vec![" "])),
                    Arc::new(Float32Array::from(vec![0.9_f32])),
                    Arc::new(
                        FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                            vec![Some(vec![Some(0.1_f32), Some(0.2_f32), Some(0.3_f32)])],
                            3,
                        ),
                    ),
                    Arc::new(
                        FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                            vec![Some(vec![Some(0.7_f32), Some(0.8_f32), Some(0.9_f32)])],
                            3,
                        ),
                    ),
                ],
            ),
            "record batch should build",
        );

        assert_eq!(
            super::validate_rerank_request_batch(&batch, 3),
            Err(
                "rerank request column `doc_id` must not contain blank values; row 0 is blank"
                    .to_string()
            )
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_request_batch_validation_rejects_duplicate_doc_id() {
        use arrow_array::types::Float32Type;
        use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
        ]));
        let batch = must_ok(
            RecordBatch::try_new(
                schema,
                vec![
                    Arc::new(StringArray::from(vec!["doc-1", "doc-1"])),
                    Arc::new(Float32Array::from(vec![0.9_f32, 0.8_f32])),
                    Arc::new(
                        FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                            vec![
                                Some(vec![Some(0.1_f32), Some(0.2_f32), Some(0.3_f32)]),
                                Some(vec![Some(0.4_f32), Some(0.5_f32), Some(0.6_f32)]),
                            ],
                            3,
                        ),
                    ),
                    Arc::new(
                        FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                            vec![
                                Some(vec![Some(0.7_f32), Some(0.8_f32), Some(0.9_f32)]),
                                Some(vec![Some(0.7_f32), Some(0.8_f32), Some(0.9_f32)]),
                            ],
                            3,
                        ),
                    ),
                ],
            ),
            "record batch should build",
        );

        assert_eq!(
            super::validate_rerank_request_batch(&batch, 3),
            Err(
                "rerank request column `doc_id` must be unique across one batch; row 1 duplicates `doc-1`"
                    .to_string()
            )
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_request_batch_validation_rejects_out_of_range_vector_score() {
        use arrow_array::types::Float32Type;
        use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
        ]));
        let batch = must_ok(
            RecordBatch::try_new(
                schema,
                vec![
                    Arc::new(StringArray::from(vec!["doc-1"])),
                    Arc::new(Float32Array::from(vec![1.2_f32])),
                    Arc::new(
                        FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                            vec![Some(vec![Some(0.1_f32), Some(0.2_f32), Some(0.3_f32)])],
                            3,
                        ),
                    ),
                    Arc::new(
                        FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                            vec![Some(vec![Some(0.7_f32), Some(0.8_f32), Some(0.9_f32)])],
                            3,
                        ),
                    ),
                ],
            ),
            "record batch should build",
        );

        assert_eq!(
            super::validate_rerank_request_batch(&batch, 3),
            Err(
                "rerank request column `vector_score` must stay within inclusive range [0.0, 1.0]; row 0 is 1.2"
                    .to_string()
            )
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_request_batch_validation_rejects_query_embedding_drift() {
        use arrow_array::types::Float32Type;
        use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
        ]));
        let batch = must_ok(
            RecordBatch::try_new(
                schema,
                vec![
                    Arc::new(StringArray::from(vec!["doc-1", "doc-2"])),
                    Arc::new(Float32Array::from(vec![0.9_f32, 0.8_f32])),
                    Arc::new(
                        FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                            vec![
                                Some(vec![Some(0.1_f32), Some(0.2_f32), Some(0.3_f32)]),
                                Some(vec![Some(0.4_f32), Some(0.5_f32), Some(0.6_f32)]),
                            ],
                            3,
                        ),
                    ),
                    Arc::new(
                        FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                            vec![
                                Some(vec![Some(0.7_f32), Some(0.8_f32), Some(0.9_f32)]),
                                Some(vec![Some(1.0_f32), Some(1.1_f32), Some(1.2_f32)]),
                            ],
                            3,
                        ),
                    ),
                ],
            ),
            "record batch should build",
        );

        assert_eq!(
            super::validate_rerank_request_batch(&batch, 3),
            Err(
                "rerank request column `query_embedding` must remain stable across all rows; row 1 differs from row 0"
                    .to_string()
            )
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_request_batch_scoring_blends_vector_and_semantic_similarity() {
        use arrow_array::types::Float32Type;
        use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
        ]));
        let batch = must_ok(
            RecordBatch::try_new(
                schema,
                vec![
                    Arc::new(StringArray::from(vec!["doc-0", "doc-1"])),
                    Arc::new(Float32Array::from(vec![0.5_f32, 0.8_f32])),
                    Arc::new(
                        FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                            vec![
                                Some(vec![Some(1.0_f32), Some(0.0_f32), Some(0.0_f32)]),
                                Some(vec![Some(0.0_f32), Some(1.0_f32), Some(0.0_f32)]),
                            ],
                            3,
                        ),
                    ),
                    Arc::new(
                        FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                            vec![
                                Some(vec![Some(1.0_f32), Some(0.0_f32), Some(0.0_f32)]),
                                Some(vec![Some(1.0_f32), Some(0.0_f32), Some(0.0_f32)]),
                            ],
                            3,
                        ),
                    ),
                ],
            ),
            "record batch should build",
        );

        let scored = must_ok(
            super::score_rerank_request_batch(&batch, 3),
            "rerank scoring should succeed",
        );

        assert_eq!(scored.len(), 2);
        assert_eq!(scored[0].doc_id, "doc-0");
        assert!((scored[0].vector_score - 0.5).abs() < 1e-6);
        assert!((scored[0].semantic_score - 1.0).abs() < 1e-6);
        assert!((scored[0].final_score - 0.8).abs() < 1e-6);
        assert_eq!(scored[1].doc_id, "doc-1");
        assert!((scored[1].vector_score - 0.8).abs() < 1e-6);
        assert!((scored[1].semantic_score - 0.5).abs() < 1e-6);
        assert!((scored[1].final_score - 0.62).abs() < 1e-6);
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_score_weights_normalize_runtime_policy() {
        let weights = must_ok(
            super::RerankScoreWeights::new(2.0, 3.0),
            "weights should validate",
        );
        let normalized = weights.normalized();

        assert!((normalized.vector_weight - 0.4).abs() < 1e-6);
        assert!((normalized.semantic_weight - 0.6).abs() < 1e-6);
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_score_weights_reject_zero_sum_policy() {
        let error = must_err(
            super::RerankScoreWeights::new(0.0, 0.0),
            "zero-sum weights should fail",
        );
        assert_eq!(error, "rerank score weights must sum to greater than zero");
    }

    #[cfg(feature = "julia")]
    #[test]
    fn score_rerank_request_batch_with_weights_respects_runtime_policy() {
        use arrow_array::types::Float32Type;
        use arrow_array::{FixedSizeListArray, Float32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_REQUEST_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(RERANK_REQUEST_VECTOR_SCORE_COLUMN, DataType::Float32, false),
            Field::new(
                RERANK_REQUEST_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
            Field::new(
                RERANK_REQUEST_QUERY_EMBEDDING_COLUMN,
                DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 3),
                false,
            ),
        ]));
        let batch = must_ok(
            RecordBatch::try_new(
                schema,
                vec![
                    Arc::new(StringArray::from(vec!["doc-0", "doc-1"])),
                    Arc::new(Float32Array::from(vec![0.5_f32, 0.8_f32])),
                    Arc::new(
                        FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                            vec![
                                Some(vec![Some(1.0_f32), Some(0.0_f32), Some(0.0_f32)]),
                                Some(vec![Some(0.0_f32), Some(1.0_f32), Some(0.0_f32)]),
                            ],
                            3,
                        ),
                    ),
                    Arc::new(
                        FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                            vec![
                                Some(vec![Some(1.0_f32), Some(0.0_f32), Some(0.0_f32)]),
                                Some(vec![Some(1.0_f32), Some(0.0_f32), Some(0.0_f32)]),
                            ],
                            3,
                        ),
                    ),
                ],
            ),
            "record batch should build",
        );

        let scored = must_ok(
            super::score_rerank_request_batch_with_weights(
                &batch,
                3,
                must_ok(
                    super::RerankScoreWeights::new(0.9, 0.1),
                    "weights should validate",
                ),
            ),
            "rerank scoring should succeed",
        );

        assert!((scored[0].final_score - 0.55).abs() < 1e-6);
        assert!((scored[1].final_score - 0.77).abs() < 1e-6);
        assert!(scored[1].final_score > scored[0].final_score);
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_response_schema_validation_accepts_stable_shape() {
        use arrow_schema::{DataType, Field, Schema};

        let schema = Schema::new(vec![
            Field::new(RERANK_RESPONSE_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(
                RERANK_RESPONSE_VECTOR_SCORE_COLUMN,
                DataType::Float64,
                false,
            ),
            Field::new(
                RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN,
                DataType::Float64,
                false,
            ),
            Field::new(RERANK_RESPONSE_FINAL_SCORE_COLUMN, DataType::Float64, false),
            Field::new(RERANK_RESPONSE_RANK_COLUMN, DataType::Int32, false),
        ]);

        assert!(super::validate_rerank_response_schema(&schema).is_ok());
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_response_schema_validation_rejects_wrong_rank_type() {
        use arrow_schema::{DataType, Field, Schema};

        let schema = Schema::new(vec![
            Field::new(RERANK_RESPONSE_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(
                RERANK_RESPONSE_VECTOR_SCORE_COLUMN,
                DataType::Float64,
                false,
            ),
            Field::new(
                RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN,
                DataType::Float64,
                false,
            ),
            Field::new(RERANK_RESPONSE_FINAL_SCORE_COLUMN, DataType::Float64, false),
            Field::new(RERANK_RESPONSE_RANK_COLUMN, DataType::UInt32, false),
        ]);

        assert_eq!(
            super::validate_rerank_response_schema(&schema),
            Err("rerank response column `rank` must be Int32".to_string())
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_response_batch_validation_accepts_stable_semantics() {
        use arrow_array::{Float64Array, Int32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_RESPONSE_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(
                RERANK_RESPONSE_VECTOR_SCORE_COLUMN,
                DataType::Float64,
                false,
            ),
            Field::new(
                RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN,
                DataType::Float64,
                false,
            ),
            Field::new(RERANK_RESPONSE_FINAL_SCORE_COLUMN, DataType::Float64, false),
            Field::new(RERANK_RESPONSE_RANK_COLUMN, DataType::Int32, false),
        ]));
        let batch = must_ok(
            RecordBatch::try_new(
                schema,
                vec![
                    Arc::new(StringArray::from(vec!["doc-1", "doc-2"])),
                    Arc::new(Float64Array::from(vec![0.91_f64, 0.82_f64])),
                    Arc::new(Float64Array::from(vec![0.97_f64, 0.91_f64])),
                    Arc::new(Float64Array::from(vec![0.97_f64, 0.91_f64])),
                    Arc::new(Int32Array::from(vec![1_i32, 2_i32])),
                ],
            ),
            "record batch should build",
        );

        assert!(super::validate_rerank_response_batch(&batch).is_ok());
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_response_batch_validation_rejects_duplicate_rank() {
        use arrow_array::{Float64Array, Int32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_RESPONSE_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(
                RERANK_RESPONSE_VECTOR_SCORE_COLUMN,
                DataType::Float64,
                false,
            ),
            Field::new(
                RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN,
                DataType::Float64,
                false,
            ),
            Field::new(RERANK_RESPONSE_FINAL_SCORE_COLUMN, DataType::Float64, false),
            Field::new(RERANK_RESPONSE_RANK_COLUMN, DataType::Int32, false),
        ]));
        let batch = must_ok(
            RecordBatch::try_new(
                schema,
                vec![
                    Arc::new(StringArray::from(vec!["doc-1", "doc-2"])),
                    Arc::new(Float64Array::from(vec![0.91_f64, 0.82_f64])),
                    Arc::new(Float64Array::from(vec![0.97_f64, 0.91_f64])),
                    Arc::new(Float64Array::from(vec![0.97_f64, 0.91_f64])),
                    Arc::new(Int32Array::from(vec![1_i32, 1_i32])),
                ],
            ),
            "record batch should build",
        );

        assert_eq!(
            super::validate_rerank_response_batch(&batch),
            Err(
                "rerank response column `rank` must be unique across one batch; row 1 duplicates `1`"
                    .to_string()
            )
        );
    }

    #[cfg(feature = "julia")]
    #[test]
    fn rerank_response_batch_validation_rejects_out_of_range_final_score() {
        use arrow_array::{Float64Array, Int32Array, RecordBatch, StringArray};
        use arrow_schema::{DataType, Field, Schema};
        use std::sync::Arc;

        let schema = Arc::new(Schema::new(vec![
            Field::new(RERANK_RESPONSE_DOC_ID_COLUMN, DataType::Utf8, false),
            Field::new(
                RERANK_RESPONSE_VECTOR_SCORE_COLUMN,
                DataType::Float64,
                false,
            ),
            Field::new(
                RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN,
                DataType::Float64,
                false,
            ),
            Field::new(RERANK_RESPONSE_FINAL_SCORE_COLUMN, DataType::Float64, false),
            Field::new(RERANK_RESPONSE_RANK_COLUMN, DataType::Int32, false),
        ]));
        let batch = must_ok(
            RecordBatch::try_new(
                schema,
                vec![
                    Arc::new(StringArray::from(vec!["doc-1"])),
                    Arc::new(Float64Array::from(vec![0.9_f64])),
                    Arc::new(Float64Array::from(vec![0.95_f64])),
                    Arc::new(Float64Array::from(vec![1.2_f64])),
                    Arc::new(Int32Array::from(vec![1_i32])),
                ],
            ),
            "record batch should build",
        );

        assert_eq!(
            super::validate_rerank_response_batch(&batch),
            Err(
                "rerank response column `final_score` must stay within inclusive range [0.0, 1.0]; row 0 is 1.2"
                    .to_string()
            )
        );
    }
}
