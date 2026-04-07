use std::sync::Arc;

use async_trait::async_trait;
use xiuxian_vector::LanceRecordBatch;
use xiuxian_wendao_runtime::transport::{
    RepoSearchFlightRequest, RepoSearchFlightRouteProvider, RerankScoreWeights, WendaoFlightService,
};

use super::service::build_studio_search_flight_service_with_repo_provider;
use crate::gateway::studio::router::{GatewayState, StudioState};
use crate::repo_index::RepoCodeDocument;
use crate::search::SearchPlaneService;
use crate::search::repo_search::search_repo_content_batch;

/// Runtime-backed repo-search Flight provider that reads from the Wendao search plane.
#[derive(Clone)]
pub struct StudioRepoSearchFlightRouteProvider {
    search_plane: Arc<SearchPlaneService>,
}

impl std::fmt::Debug for StudioRepoSearchFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioRepoSearchFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

impl StudioRepoSearchFlightRouteProvider {
    /// Create one Studio repo-search Flight provider backed by the Wendao
    /// search plane.
    #[must_use]
    pub fn new(search_plane: Arc<SearchPlaneService>) -> Self {
        Self { search_plane }
    }
}

#[async_trait]
impl RepoSearchFlightRouteProvider for StudioRepoSearchFlightRouteProvider {
    async fn repo_search_batch(
        &self,
        request: &RepoSearchFlightRequest,
    ) -> Result<LanceRecordBatch, String> {
        if request.repo_id.trim().is_empty() {
            return Err("repo-search Flight request repo_id must not be blank".to_string());
        }
        search_repo_content_batch(self.search_plane.as_ref(), request)
            .await
            .map_err(|error| format!("repo-search Flight provider failed: {error}"))
    }
}

/// Build one runtime-owned Flight service from the Wendao search plane.
///
/// # Errors
///
/// Returns an error when the runtime Flight service cannot be constructed for
/// the requested schema version and rerank dimension.
pub fn build_repo_search_flight_service(
    search_plane: Arc<SearchPlaneService>,
    expected_schema_version: impl Into<String>,
    rerank_dimension: usize,
) -> Result<WendaoFlightService, String> {
    build_repo_search_flight_service_with_weights(
        search_plane,
        expected_schema_version,
        rerank_dimension,
        RerankScoreWeights::default(),
    )
}

/// Build one runtime-owned Flight service from the Wendao search plane with
/// explicit rerank score weights.
///
/// # Errors
///
/// Returns an error when the runtime Flight service cannot be constructed for
/// the requested schema version, rerank dimension, and rerank score weights.
pub fn build_repo_search_flight_service_with_weights(
    search_plane: Arc<SearchPlaneService>,
    expected_schema_version: impl Into<String>,
    rerank_dimension: usize,
    rerank_weights: RerankScoreWeights,
) -> Result<WendaoFlightService, String> {
    let provider = Arc::new(StudioRepoSearchFlightRouteProvider::new(search_plane));
    WendaoFlightService::new_with_provider(
        expected_schema_version,
        provider,
        rerank_dimension,
        rerank_weights,
    )
}

/// Build one runtime-owned Flight service from the Wendao search plane and the
/// current Studio-owned semantic search providers.
///
/// # Errors
///
/// Returns an error when the runtime Flight service cannot be constructed for
/// the requested schema version and rerank dimension.
pub fn build_studio_flight_service(
    search_plane: Arc<SearchPlaneService>,
    gateway_state: Arc<GatewayState>,
    expected_schema_version: impl Into<String>,
    rerank_dimension: usize,
) -> Result<WendaoFlightService, String> {
    build_studio_flight_service_with_weights(
        search_plane,
        gateway_state,
        expected_schema_version,
        rerank_dimension,
        RerankScoreWeights::default(),
    )
}

/// Build one runtime-owned Flight service from the Wendao search plane and the
/// current Studio-owned semantic search providers with explicit rerank weights.
///
/// # Errors
///
/// Returns an error when the runtime Flight service cannot be constructed for
/// the requested schema version, rerank dimension, and rerank score weights.
pub fn build_studio_flight_service_with_weights(
    search_plane: Arc<SearchPlaneService>,
    gateway_state: Arc<GatewayState>,
    expected_schema_version: impl Into<String>,
    rerank_dimension: usize,
    rerank_weights: RerankScoreWeights,
) -> Result<WendaoFlightService, String> {
    let provider = Arc::new(StudioRepoSearchFlightRouteProvider::new(search_plane));
    build_studio_search_flight_service_with_repo_provider(
        expected_schema_version,
        provider,
        gateway_state,
        rerank_dimension,
        rerank_weights,
    )
}

/// Build one runtime-owned Flight service from the Wendao search plane and one
/// Studio bootstrap state resolved from explicit project/config roots.
///
/// # Errors
///
/// Returns an error when the plugin registry cannot be bootstrapped or when the
/// runtime Flight service cannot be constructed for the requested schema
/// version and rerank dimension.
pub fn build_studio_flight_service_for_roots(
    search_plane: Arc<SearchPlaneService>,
    project_root: std::path::PathBuf,
    config_root: std::path::PathBuf,
    expected_schema_version: impl Into<String>,
    rerank_dimension: usize,
) -> Result<WendaoFlightService, String> {
    build_studio_flight_service_for_roots_with_weights(
        search_plane,
        project_root,
        config_root,
        expected_schema_version,
        rerank_dimension,
        RerankScoreWeights::default(),
    )
}

/// Build one runtime-owned Flight service from the Wendao search plane and one
/// Studio bootstrap state resolved from explicit project/config roots with
/// explicit rerank score weights.
///
/// # Errors
///
/// Returns an error when the plugin registry cannot be bootstrapped or when the
/// runtime Flight service cannot be constructed for the requested schema
/// version, rerank dimension, and rerank score weights.
pub fn build_studio_flight_service_for_roots_with_weights(
    search_plane: Arc<SearchPlaneService>,
    project_root: std::path::PathBuf,
    config_root: std::path::PathBuf,
    expected_schema_version: impl Into<String>,
    rerank_dimension: usize,
    rerank_weights: RerankScoreWeights,
) -> Result<WendaoFlightService, String> {
    let plugin_registry = Arc::new(
        crate::analyzers::bootstrap_builtin_registry()
            .map_err(|error| format!("bootstrap registry: {error}"))?,
    );
    let studio = StudioState::new_with_bootstrap_ui_config_for_roots_and_search_plane(
        plugin_registry,
        project_root,
        config_root,
        search_plane.as_ref().clone(),
    );
    let gateway_state = Arc::new(GatewayState {
        index: None,
        signal_tx: None,
        studio: Arc::new(studio),
    });
    build_studio_flight_service_with_weights(
        search_plane,
        gateway_state,
        expected_schema_version,
        rerank_dimension,
        rerank_weights,
    )
}

/// Seed one minimal repo-content sample into the search plane for Flight smoke
/// tests and local bring-up.
///
/// # Errors
///
/// Returns an error when the repo identifier is blank or when repo-content
/// publication fails.
pub async fn bootstrap_sample_repo_search_content(
    search_plane: &SearchPlaneService,
    repo_id: impl AsRef<str>,
) -> Result<(), String> {
    let repo_id = repo_id.as_ref().trim();
    if repo_id.is_empty() {
        return Err("sample repo-search bootstrap repo_id must not be blank".to_string());
    }

    let documents = vec![
        RepoCodeDocument {
            path: "src/lib.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from("pub fn alpha_beta() {}\n"),
            size_bytes: 23,
            modified_unix_ms: 10,
        },
        RepoCodeDocument {
            path: "src/flight.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from("pub fn flight_router() -> &'static str { \"flight\" }\n"),
            size_bytes: 52,
            modified_unix_ms: 10,
        },
        RepoCodeDocument {
            path: "src/search.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from(
                "pub fn repo_search_kernel() -> &'static str { \"searchonlytoken semantic search kernel\" }\n",
            ),
            size_bytes: 88,
            modified_unix_ms: 10,
        },
        RepoCodeDocument {
            path: "src/flight_search.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from(
                "pub fn flight_search_bridge() -> &'static str { \"flightbridgetoken flight search bridge\" }\n",
            ),
            size_bytes: 92,
            modified_unix_ms: 10,
        },
        RepoCodeDocument {
            path: "src/camelbridge.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from(
                "pub fn camel_bridge_lower() -> &'static str { \"camelbridgetoken\" }\n",
            ),
            size_bytes: 70,
            modified_unix_ms: 10,
        },
        RepoCodeDocument {
            path: "src/a_rank.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from(
                "pub fn alpha_rank() -> &'static str { \"ranktieexacttoken\" }\n",
            ),
            size_bytes: 62,
            modified_unix_ms: 10,
        },
        RepoCodeDocument {
            path: "src/z_rank.rs".to_string(),
            language: Some("rust".to_string()),
            contents: Arc::<str>::from(
                "pub fn zeta_rank() -> &'static str { \"ranktieexacttoken\" }\n",
            ),
            size_bytes: 61,
            modified_unix_ms: 10,
        },
        RepoCodeDocument {
            path: "README.md".to_string(),
            language: Some("markdown".to_string()),
            contents: Arc::<str>::from(
                "# alpha repo\nThis repo mentions alpha beta flight search.\n",
            ),
            size_bytes: 56,
            modified_unix_ms: 10,
        },
        RepoCodeDocument {
            path: "docs/CamelBridge.md".to_string(),
            language: Some("markdown".to_string()),
            contents: Arc::<str>::from(
                "# CamelBridgeToken\nExact-case bridge token for flight ranking.\n",
            ),
            size_bytes: 64,
            modified_unix_ms: 10,
        },
    ];

    search_plane
        .publish_repo_content_chunks_with_revision(repo_id, &documents, Some("flight-smoke-rev"))
        .await
        .map_err(|error| {
            format!("failed to bootstrap sample repo-search content for `{repo_id}`: {error}")
        })
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    use arrow_flight::flight_service_server::FlightService;
    use arrow_flight::{FlightDescriptor, FlightInfo};
    use tempfile::tempdir;
    use tonic::Request;
    use xiuxian_vector::{LanceFloat64Array, LanceRecordBatch, LanceStringArray};

    use super::{
        StudioRepoSearchFlightRouteProvider, bootstrap_sample_repo_search_content,
        build_repo_search_flight_service, build_studio_flight_service,
        build_studio_flight_service_for_roots,
    };
    use crate::analyzers::bootstrap_builtin_registry;
    use crate::gateway::studio::router::{GatewayState, StudioState};
    use crate::gateway::studio::search::build_symbol_index;
    use crate::gateway::studio::test_support::init_git_repository;
    use crate::gateway::studio::types::{UiConfig, UiProjectConfig};
    use crate::repo_index::RepoCodeDocument;
    use crate::search::{SearchMaintenancePolicy, SearchManifestKeyspace, SearchPlaneService};
    #[cfg(feature = "julia")]
    use xiuxian_wendao_runtime::transport::{
        ANALYSIS_CODE_AST_ROUTE, WENDAO_ANALYSIS_LINE_HEADER, WENDAO_ANALYSIS_REPO_HEADER,
    };
    use xiuxian_wendao_runtime::transport::{
        ANALYSIS_MARKDOWN_ROUTE, RepoSearchFlightRequest, RepoSearchFlightRouteProvider,
        SEARCH_SYMBOLS_ROUTE, WENDAO_ANALYSIS_PATH_HEADER, WENDAO_SCHEMA_VERSION_HEADER,
        WENDAO_SEARCH_LIMIT_HEADER, WENDAO_SEARCH_QUERY_HEADER, flight_descriptor_path,
    };

    #[derive(Default)]
    struct RepoSearchRequestFilters {
        language_filters: HashSet<String>,
        path_prefixes: HashSet<String>,
        title_filters: HashSet<String>,
        tag_filters: HashSet<String>,
        filename_filters: HashSet<String>,
    }

    fn tempdir_or_panic(context: &str) -> tempfile::TempDir {
        tempdir().unwrap_or_else(|error| panic!("{context}: {error}"))
    }

    fn create_dir_all_or_panic(path: impl AsRef<Path>, context: &str) {
        std::fs::create_dir_all(path).unwrap_or_else(|error| panic!("{context}: {error}"));
    }

    fn write_file_or_panic(path: impl AsRef<Path>, contents: &str, context: &str) {
        std::fs::write(path, contents).unwrap_or_else(|error| panic!("{context}: {error}"));
    }

    #[cfg(feature = "julia")]
    fn init_git_repo_or_panic(path: impl AsRef<Path>, context: &str) {
        let _ = context;
        init_git_repository(path);
    }

    fn repo_document(path: &str, language: &str, contents: &str) -> RepoCodeDocument {
        RepoCodeDocument {
            path: path.to_string(),
            language: Some(language.to_string()),
            contents: Arc::<str>::from(contents),
            size_bytes: u64::try_from(contents.len())
                .unwrap_or_else(|error| panic!("document length should fit: {error}")),
            modified_unix_ms: 10,
        }
    }

    fn repo_search_request(
        repo_id: &str,
        query_text: &str,
        limit: usize,
        filters: RepoSearchRequestFilters,
    ) -> RepoSearchFlightRequest {
        RepoSearchFlightRequest {
            repo_id: repo_id.to_string(),
            query_text: query_text.to_string(),
            limit,
            language_filters: filters.language_filters,
            path_prefixes: filters.path_prefixes,
            title_filters: filters.title_filters,
            tag_filters: filters.tag_filters,
            filename_filters: filters.filename_filters,
        }
    }

    fn string_column<'a>(batch: &'a LanceRecordBatch, column: &str) -> &'a LanceStringArray {
        let Some(column) = batch
            .column_by_name(column)
            .and_then(|column| column.as_any().downcast_ref::<LanceStringArray>())
        else {
            panic!("`{column}` should decode as Utf8");
        };
        column
    }

    fn float_column<'a>(batch: &'a LanceRecordBatch, column: &str) -> &'a LanceFloat64Array {
        let Some(column) = batch
            .column_by_name(column)
            .and_then(|column| column.as_any().downcast_ref::<LanceFloat64Array>())
        else {
            panic!("`{column}` should decode as Float64");
        };
        column
    }

    fn first_ticket(flight_info: &FlightInfo, context: &str) -> String {
        let Some(endpoint) = flight_info.endpoint.first() else {
            panic!("{context} should emit one ticket");
        };
        let Some(ticket) = endpoint.ticket.as_ref() else {
            panic!("{context} should emit one ticket");
        };
        String::from_utf8_lossy(ticket.ticket.as_ref()).into_owned()
    }

    async fn repo_search_batch_or_panic(
        provider: &StudioRepoSearchFlightRouteProvider,
        request: &RepoSearchFlightRequest,
        context: &str,
    ) -> LanceRecordBatch {
        provider
            .repo_search_batch(request)
            .await
            .unwrap_or_else(|error| panic!("{context}: {error}"))
    }

    fn populate_search_headers(
        metadata: &mut tonic::metadata::MetadataMap,
        query: &str,
        limit: usize,
    ) {
        metadata.insert(
            WENDAO_SCHEMA_VERSION_HEADER,
            "v2".parse()
                .unwrap_or_else(|error| panic!("schema metadata: {error}")),
        );
        metadata.insert(
            WENDAO_SEARCH_QUERY_HEADER,
            query
                .parse()
                .unwrap_or_else(|error| panic!("query metadata: {error}")),
        );
        metadata.insert(
            WENDAO_SEARCH_LIMIT_HEADER,
            limit
                .to_string()
                .parse()
                .unwrap_or_else(|error| panic!("limit metadata: {error}")),
        );
    }

    fn populate_markdown_analysis_headers(metadata: &mut tonic::metadata::MetadataMap, path: &str) {
        metadata.insert(
            WENDAO_SCHEMA_VERSION_HEADER,
            "v2".parse()
                .unwrap_or_else(|error| panic!("schema metadata: {error}")),
        );
        metadata.insert(
            WENDAO_ANALYSIS_PATH_HEADER,
            path.parse()
                .unwrap_or_else(|error| panic!("analysis path metadata: {error}")),
        );
    }

    #[cfg(feature = "julia")]
    fn populate_code_ast_analysis_headers(
        metadata: &mut tonic::metadata::MetadataMap,
        path: &str,
        repo_id: &str,
        line_hint: Option<usize>,
    ) {
        populate_markdown_analysis_headers(metadata, path);
        metadata.insert(
            WENDAO_ANALYSIS_REPO_HEADER,
            repo_id
                .parse()
                .unwrap_or_else(|error| panic!("analysis repo metadata: {error}")),
        );
        if let Some(line_hint) = line_hint {
            metadata.insert(
                WENDAO_ANALYSIS_LINE_HEADER,
                line_hint
                    .to_string()
                    .parse()
                    .unwrap_or_else(|error| panic!("analysis line metadata: {error}")),
            );
        }
    }

    fn test_studio_state(search_plane_root: PathBuf) -> StudioState {
        StudioState::new_with_bootstrap_ui_config_and_search_plane_root(
            Arc::new(
                bootstrap_builtin_registry()
                    .unwrap_or_else(|error| panic!("bootstrap registry: {error}")),
            ),
            search_plane_root,
        )
    }

    #[tokio::test]
    async fn studio_repo_search_flight_provider_reads_repo_content_hits() {
        let temp_dir = tempdir_or_panic("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        create_dir_all_or_panic(&project_root, "project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-repo-search-provider"),
            SearchMaintenancePolicy::default(),
        ));
        let repo_id = "alpha/repo";
        let documents = vec![
            repo_document("src/lib.rs", "rust", "pub fn alpha_beta() {}\n"),
            repo_document("src/other.rs", "rust", "pub fn unrelated() {}\n"),
        ];
        service
            .publish_repo_content_chunks_with_revision(repo_id, &documents, Some("rev-1"))
            .await
            .unwrap_or_else(|error| panic!("repo content publication should succeed: {error}"));

        let provider = StudioRepoSearchFlightRouteProvider::new(Arc::clone(&service));
        let batch = repo_search_batch_or_panic(
            &provider,
            &repo_search_request(repo_id, "alpha", 5, RepoSearchRequestFilters::default()),
            "provider should materialize one search batch",
        )
        .await;

        let doc_ids = string_column(&batch, "doc_id");
        let paths = string_column(&batch, "path");
        let languages = string_column(&batch, "language");

        assert_eq!(batch.num_rows(), 1);
        assert_eq!(doc_ids.value(0), "lib.rs");
        assert_eq!(paths.value(0), "src/lib.rs");
        assert_eq!(languages.value(0), "rust");
    }

    #[tokio::test]
    async fn build_studio_flight_service_accepts_runtime_studio_providers() {
        let temp_dir = tempdir_or_panic("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        create_dir_all_or_panic(
            project_root.join("packages/rust/crates/demo/src"),
            "project fixture dirs should build",
        );
        write_file_or_panic(
            project_root.join("packages/rust/crates/demo/src/lib.rs"),
            "pub struct AlphaService;\npub fn alpha_handler() {}\n",
            "project fixture file should write",
        );

        let mut studio = test_studio_state(project_root.join("studio-flight-service"));
        studio.project_root = project_root.clone();
        studio.config_root = project_root.clone();
        studio.set_ui_config(UiConfig {
            projects: vec![UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec!["packages".to_string()],
            }],
            repo_projects: Vec::new(),
        });
        let warmed_index = build_symbol_index(
            studio.project_root.as_path(),
            studio.config_root.as_path(),
            studio.configured_projects().as_slice(),
        );
        studio.symbol_index_coordinator.set_ready_index_for_test(
            studio.configured_projects().as_slice(),
            Arc::clone(&studio.symbol_index),
            warmed_index,
        );
        let state = Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(studio),
        });

        let search_plane = Arc::new(SearchPlaneService::with_paths(
            project_root,
            storage_root,
            SearchManifestKeyspace::new("xiuxian:test:studio-flight-service"),
            SearchMaintenancePolicy::default(),
        ));
        let flight_service = build_studio_flight_service(search_plane, state, "v2", 3)
            .unwrap_or_else(|error| panic!("studio flight service should build: {error}"));
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(SEARCH_SYMBOLS_ROUTE)
                .unwrap_or_else(|error| panic!("descriptor path: {error}")),
        );
        let mut request = Request::new(descriptor);
        populate_search_headers(request.metadata_mut(), "alpha", 5);

        let response = flight_service
            .get_flight_info(request)
            .await
            .unwrap_or_else(|error| {
                panic!("studio flight service should resolve symbols route: {error}")
            });
        let ticket = first_ticket(&response.into_inner(), "symbols route");

        assert_eq!(ticket, SEARCH_SYMBOLS_ROUTE);
    }

    #[tokio::test]
    async fn build_studio_flight_service_for_roots_accepts_runtime_studio_providers() {
        let temp_dir = tempdir_or_panic("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        create_dir_all_or_panic(
            project_root.join("packages/rust/crates/demo/src"),
            "project fixture dirs should build",
        );
        write_file_or_panic(
            project_root.join("packages/rust/crates/demo/src/lib.rs"),
            "pub struct AlphaService;\npub fn alpha_handler() {}\n",
            "project fixture file should write",
        );
        write_file_or_panic(
            project_root.join("wendao.toml"),
            r#"
[link_graph.projects.kernel]
root = "."
dirs = ["packages"]
"#,
            "wendao.toml should write",
        );

        let search_plane = Arc::new(SearchPlaneService::with_paths(
            project_root.clone(),
            storage_root,
            SearchManifestKeyspace::new("xiuxian:test:studio-flight-service-roots"),
            SearchMaintenancePolicy::default(),
        ));
        let flight_service = build_studio_flight_service_for_roots(
            search_plane,
            project_root.clone(),
            project_root.clone(),
            "v2",
            3,
        )
        .unwrap_or_else(|error| panic!("studio flight service should build from roots: {error}"));
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(SEARCH_SYMBOLS_ROUTE)
                .unwrap_or_else(|error| panic!("descriptor path: {error}")),
        );
        let mut request = Request::new(descriptor);
        populate_search_headers(request.metadata_mut(), "alpha", 5);

        let response = flight_service
            .get_flight_info(request)
            .await
            .unwrap_or_else(|error| {
                panic!("studio flight service should resolve symbols route: {error}")
            });
        let ticket = first_ticket(&response.into_inner(), "symbols route");

        assert_eq!(ticket, SEARCH_SYMBOLS_ROUTE);
    }

    #[tokio::test]
    async fn build_studio_flight_service_for_roots_accepts_markdown_analysis_routes() {
        let temp_dir = tempdir_or_panic("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        create_dir_all_or_panic(project_root.join("docs"), "project docs dir should build");
        write_file_or_panic(
            project_root.join("docs/analysis.md"),
            "# Analysis Kernel\n\n## Inputs\n- [ ] Parse markdown\n",
            "project markdown fixture should write",
        );
        write_file_or_panic(
            project_root.join("wendao.toml"),
            r#"
[link_graph.projects.kernel]
root = "."
dirs = ["docs"]
"#,
            "wendao.toml should write",
        );

        let search_plane = Arc::new(SearchPlaneService::with_paths(
            project_root.clone(),
            storage_root,
            SearchManifestKeyspace::new("xiuxian:test:flight-studio-service-roots-markdown"),
            SearchMaintenancePolicy::default(),
        ));
        let flight_service = build_studio_flight_service_for_roots(
            search_plane,
            project_root.clone(),
            project_root.clone(),
            "v2",
            3,
        )
        .unwrap_or_else(|error| panic!("studio flight service should build from roots: {error}"));
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(ANALYSIS_MARKDOWN_ROUTE)
                .unwrap_or_else(|error| panic!("descriptor path: {error}")),
        );
        let mut request = Request::new(descriptor);
        populate_markdown_analysis_headers(request.metadata_mut(), "kernel/docs/analysis.md");

        let response = flight_service
            .get_flight_info(request)
            .await
            .unwrap_or_else(|error| {
                panic!("studio flight service should resolve markdown analysis route: {error}")
            });
        let ticket = first_ticket(&response.into_inner(), "markdown analysis route");

        assert_eq!(ticket, ANALYSIS_MARKDOWN_ROUTE);
    }

    #[cfg(feature = "julia")]
    #[tokio::test]
    async fn build_studio_flight_service_for_roots_accepts_code_ast_analysis_routes() {
        let temp_dir = tempdir_or_panic("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        create_dir_all_or_panic(
            project_root.join("repo/src"),
            "project repo dir should build",
        );
        init_git_repo_or_panic(
            project_root.join("repo"),
            "analysis repo fixture should initialize",
        );
        write_file_or_panic(
            project_root.join("repo/Project.toml"),
            "name = \"Demo\"\nuuid = \"00000000-0000-0000-0000-000000000001\"\n",
            "Project.toml should write",
        );
        write_file_or_panic(
            project_root.join("repo/src/lib.jl"),
            "module Demo\nexport solve\nsolve(x) = x + 1\nend\n",
            "source fixture should write",
        );
        write_file_or_panic(
            project_root.join("wendao.toml"),
            r#"
[link_graph.projects.kernel]
root = "."
dirs = ["docs"]

[link_graph.projects.demo]
root = "repo"
plugins = ["julia"]
"#,
            "wendao.toml should write",
        );

        let search_plane = Arc::new(SearchPlaneService::with_paths(
            project_root.clone(),
            storage_root,
            SearchManifestKeyspace::new("xiuxian:test:flight-studio-service-roots-code-ast"),
            SearchMaintenancePolicy::default(),
        ));
        let flight_service = build_studio_flight_service_for_roots(
            search_plane,
            project_root.clone(),
            project_root.clone(),
            "v2",
            3,
        )
        .unwrap_or_else(|error| panic!("studio flight service should build from roots: {error}"));
        let descriptor = FlightDescriptor::new_path(
            flight_descriptor_path(ANALYSIS_CODE_AST_ROUTE)
                .unwrap_or_else(|error| panic!("descriptor path: {error}")),
        );
        let mut request = Request::new(descriptor);
        populate_code_ast_analysis_headers(request.metadata_mut(), "src/lib.jl", "demo", Some(3));

        let response = flight_service
            .get_flight_info(request)
            .await
            .unwrap_or_else(|error| {
                panic!("studio flight service should resolve code AST analysis route: {error}")
            });
        let ticket = first_ticket(&response.into_inner(), "code AST analysis route");

        assert_eq!(ticket, ANALYSIS_CODE_AST_ROUTE);
    }

    #[tokio::test]
    async fn studio_repo_search_flight_provider_applies_language_filters() {
        let temp_dir = tempdir_or_panic("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        create_dir_all_or_panic(&project_root, "project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-repo-search-provider-filters"),
            SearchMaintenancePolicy::default(),
        ));
        bootstrap_sample_repo_search_content(service.as_ref(), "alpha/repo")
            .await
            .unwrap_or_else(|error| {
                panic!("sample bootstrap should publish repo content: {error}")
            });

        let provider = StudioRepoSearchFlightRouteProvider::new(Arc::clone(&service));
        let batch = repo_search_batch_or_panic(
            &provider,
            &repo_search_request(
                "alpha/repo",
                "alpha",
                10,
                RepoSearchRequestFilters {
                    language_filters: HashSet::from(["markdown".to_string()]),
                    ..RepoSearchRequestFilters::default()
                },
            ),
            "provider should materialize one markdown-filtered search batch",
        )
        .await;

        let paths = string_column(&batch, "path");
        let languages = string_column(&batch, "language");

        assert_eq!(batch.num_rows(), 1);
        assert_eq!(paths.value(0), "README.md");
        assert_eq!(languages.value(0), "markdown");
    }

    #[tokio::test]
    async fn studio_repo_search_flight_provider_applies_path_prefix_filters() {
        let temp_dir = tempdir_or_panic("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        create_dir_all_or_panic(&project_root, "project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-repo-search-provider-prefixes"),
            SearchMaintenancePolicy::default(),
        ));
        bootstrap_sample_repo_search_content(service.as_ref(), "alpha/repo")
            .await
            .unwrap_or_else(|error| {
                panic!("sample bootstrap should publish repo content: {error}")
            });

        let provider = StudioRepoSearchFlightRouteProvider::new(Arc::clone(&service));
        let batch = repo_search_batch_or_panic(
            &provider,
            &repo_search_request(
                "alpha/repo",
                "flightbridgetoken",
                10,
                RepoSearchRequestFilters {
                    path_prefixes: HashSet::from(["src/flight".to_string()]),
                    ..RepoSearchRequestFilters::default()
                },
            ),
            "provider should materialize one path-filtered search batch",
        )
        .await;

        let paths = string_column(&batch, "path");

        assert_eq!(batch.num_rows(), 1);
        assert!(paths.value(0).starts_with("src/flight"));
    }

    #[tokio::test]
    async fn studio_repo_search_flight_provider_applies_title_filters() {
        let temp_dir = tempdir_or_panic("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        create_dir_all_or_panic(&project_root, "project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-repo-search-provider-titles"),
            SearchMaintenancePolicy::default(),
        ));
        bootstrap_sample_repo_search_content(service.as_ref(), "alpha/repo")
            .await
            .unwrap_or_else(|error| {
                panic!("sample bootstrap should publish repo content: {error}")
            });

        let provider = StudioRepoSearchFlightRouteProvider::new(Arc::clone(&service));
        let batch = repo_search_batch_or_panic(
            &provider,
            &repo_search_request(
                "alpha/repo",
                "alpha",
                10,
                RepoSearchRequestFilters {
                    title_filters: HashSet::from(["readme".to_string()]),
                    ..RepoSearchRequestFilters::default()
                },
            ),
            "provider should materialize one title-filtered search batch",
        )
        .await;

        let paths = string_column(&batch, "path");
        let titles = string_column(&batch, "title");

        assert_eq!(batch.num_rows(), 1);
        assert_eq!(paths.value(0), "README.md");
        assert_eq!(titles.value(0), "README.md");
    }

    #[tokio::test]
    async fn studio_repo_search_flight_provider_applies_tag_filters() {
        let temp_dir = tempdir_or_panic("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        create_dir_all_or_panic(&project_root, "project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-repo-search-provider-tags"),
            SearchMaintenancePolicy::default(),
        ));
        bootstrap_sample_repo_search_content(service.as_ref(), "alpha/repo")
            .await
            .unwrap_or_else(|error| {
                panic!("sample bootstrap should publish repo content: {error}")
            });

        let provider = StudioRepoSearchFlightRouteProvider::new(Arc::clone(&service));
        let batch = repo_search_batch_or_panic(
            &provider,
            &repo_search_request(
                "alpha/repo",
                "alpha",
                10,
                RepoSearchRequestFilters {
                    tag_filters: HashSet::from(["lang:markdown".to_string()]),
                    ..RepoSearchRequestFilters::default()
                },
            ),
            "provider should materialize one tag-filtered search batch",
        )
        .await;

        let paths = string_column(&batch, "path");
        let languages = string_column(&batch, "language");

        assert_eq!(batch.num_rows(), 1);
        assert_eq!(paths.value(0), "README.md");
        assert_eq!(languages.value(0), "markdown");
    }

    #[tokio::test]
    async fn studio_repo_search_flight_provider_exposes_exact_match_tag() {
        let temp_dir = tempdir_or_panic("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        create_dir_all_or_panic(&project_root, "project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-repo-search-provider-exact-tag"),
            SearchMaintenancePolicy::default(),
        ));
        bootstrap_sample_repo_search_content(service.as_ref(), "alpha/repo")
            .await
            .unwrap_or_else(|error| {
                panic!("sample bootstrap should publish repo content: {error}")
            });

        let provider = StudioRepoSearchFlightRouteProvider::new(Arc::clone(&service));
        let batch = repo_search_batch_or_panic(
            &provider,
            &repo_search_request(
                "alpha/repo",
                "searchonlytoken",
                10,
                RepoSearchRequestFilters {
                    tag_filters: HashSet::from(["match:exact".to_string()]),
                    ..RepoSearchRequestFilters::default()
                },
            ),
            "provider should materialize one exact-match-tagged search batch",
        )
        .await;

        let paths = string_column(&batch, "path");

        assert_eq!(batch.num_rows(), 1);
        assert_eq!(paths.value(0), "src/search.rs");
    }

    #[tokio::test]
    async fn studio_repo_search_flight_provider_prefers_exact_case_match_over_folded_match() {
        let temp_dir = tempdir_or_panic("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        create_dir_all_or_panic(&project_root, "project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-repo-search-provider-exact-rank"),
            SearchMaintenancePolicy::default(),
        ));
        bootstrap_sample_repo_search_content(service.as_ref(), "alpha/repo")
            .await
            .unwrap_or_else(|error| {
                panic!("sample bootstrap should publish repo content: {error}")
            });

        let provider = StudioRepoSearchFlightRouteProvider::new(Arc::clone(&service));
        let batch = repo_search_batch_or_panic(
            &provider,
            &repo_search_request(
                "alpha/repo",
                "CamelBridgeToken",
                2,
                RepoSearchRequestFilters::default(),
            ),
            "provider should materialize one exact-ranked search batch",
        )
        .await;

        let paths = string_column(&batch, "path");
        let scores = float_column(&batch, "score");

        assert_eq!(batch.num_rows(), 2);
        assert_eq!(paths.value(0), "docs/CamelBridge.md");
        assert_eq!(paths.value(1), "src/camelbridge.rs");
        assert!(scores.value(0) > scores.value(1));
    }

    #[tokio::test]
    async fn studio_repo_search_flight_provider_applies_filename_filters() {
        let temp_dir = tempdir_or_panic("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        create_dir_all_or_panic(&project_root, "project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-repo-search-provider-filenames"),
            SearchMaintenancePolicy::default(),
        ));
        bootstrap_sample_repo_search_content(service.as_ref(), "alpha/repo")
            .await
            .unwrap_or_else(|error| {
                panic!("sample bootstrap should publish repo content: {error}")
            });

        let provider = StudioRepoSearchFlightRouteProvider::new(Arc::clone(&service));
        let batch = repo_search_batch_or_panic(
            &provider,
            &repo_search_request(
                "alpha/repo",
                "alpha",
                10,
                RepoSearchRequestFilters {
                    filename_filters: HashSet::from(["readme.md".to_string()]),
                    ..RepoSearchRequestFilters::default()
                },
            ),
            "provider should materialize one filename-filtered search batch",
        )
        .await;

        let paths = string_column(&batch, "path");
        let doc_ids = string_column(&batch, "doc_id");

        assert_eq!(batch.num_rows(), 1);
        assert_eq!(paths.value(0), "README.md");
        assert_eq!(doc_ids.value(0), "README.md");
    }

    #[tokio::test]
    async fn studio_repo_search_flight_provider_rejects_blank_repo_id() {
        let temp_dir = tempdir_or_panic("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        create_dir_all_or_panic(&project_root, "project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-repo-search-provider-blank"),
            SearchMaintenancePolicy::default(),
        ));
        let provider = StudioRepoSearchFlightRouteProvider::new(service);
        let Err(error) = provider
            .repo_search_batch(&repo_search_request(
                "   ",
                "alpha",
                5,
                RepoSearchRequestFilters::default(),
            ))
            .await
        else {
            panic!("blank repo id should fail");
        };
        assert_eq!(
            error,
            "repo-search Flight request repo_id must not be blank"
        );
    }

    #[test]
    fn build_repo_search_flight_service_accepts_runtime_repo_search_provider() {
        let temp_dir = tempdir_or_panic("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        create_dir_all_or_panic(&project_root, "project root should build");

        let service = Arc::new(SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-repo-search-service"),
            SearchMaintenancePolicy::default(),
        ));
        let flight_service = build_repo_search_flight_service(service, "v2", 3)
            .unwrap_or_else(|error| panic!("flight service should build: {error}"));

        let _ = flight_service;
    }

    #[tokio::test]
    async fn bootstrap_sample_repo_search_content_publishes_queryable_rows() {
        let temp_dir = tempdir_or_panic("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        create_dir_all_or_panic(&project_root, "project root should build");

        let service = SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-repo-search-bootstrap"),
            SearchMaintenancePolicy::default(),
        );
        bootstrap_sample_repo_search_content(&service, "alpha/repo")
            .await
            .unwrap_or_else(|error| {
                panic!("sample bootstrap should publish repo content: {error}")
            });

        let hits = service
            .search_repo_content_chunks("alpha/repo", "flight", &HashSet::new(), 5)
            .await
            .unwrap_or_else(|error| panic!("bootstrapped repo should be searchable: {error}"));

        assert!(!hits.is_empty());
        assert!(hits.iter().any(|hit| hit.path == "src/flight.rs"));
        assert!(hits.iter().any(|hit| hit.path == "src/flight_search.rs"));
    }

    #[tokio::test]
    async fn bootstrap_sample_repo_search_content_respects_query_and_limit() {
        let temp_dir = tempdir_or_panic("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        create_dir_all_or_panic(&project_root, "project root should build");

        let service = SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-repo-search-bootstrap-query-limit"),
            SearchMaintenancePolicy::default(),
        );
        bootstrap_sample_repo_search_content(&service, "alpha/repo")
            .await
            .unwrap_or_else(|error| {
                panic!("sample bootstrap should publish repo content: {error}")
            });

        let search_hits = service
            .search_repo_content_chunks("alpha/repo", "searchonlytoken", &HashSet::new(), 1)
            .await
            .unwrap_or_else(|error| {
                panic!("bootstrapped repo should be searchable by search keyword: {error}")
            });
        let flight_hits = service
            .search_repo_content_chunks("alpha/repo", "flightbridgetoken", &HashSet::new(), 5)
            .await
            .unwrap_or_else(|error| {
                panic!("bootstrapped repo should be searchable by combined keywords: {error}")
            });

        assert_eq!(search_hits.len(), 1);
        assert_eq!(search_hits[0].path, "src/search.rs");
        assert!(
            flight_hits
                .iter()
                .any(|hit| hit.path == "src/flight_search.rs")
        );
    }

    #[tokio::test]
    async fn bootstrap_sample_repo_search_content_uses_path_order_for_exact_match_ties() {
        let temp_dir = tempdir_or_panic("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        create_dir_all_or_panic(&project_root, "project root should build");

        let service = SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-repo-search-bootstrap-rank-tie"),
            SearchMaintenancePolicy::default(),
        );
        bootstrap_sample_repo_search_content(&service, "alpha/repo")
            .await
            .unwrap_or_else(|error| {
                panic!("sample bootstrap should publish repo content: {error}")
            });

        let hits = service
            .search_repo_content_chunks("alpha/repo", "ranktieexacttoken", &HashSet::new(), 1)
            .await
            .unwrap_or_else(|error| panic!("bootstrapped repo should expose deterministic exact-match tie ordering: {error}"));

        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, "src/a_rank.rs");
    }

    #[tokio::test]
    async fn bootstrap_sample_repo_search_content_persists_across_service_restart() {
        let temp_dir = tempdir_or_panic("temp dir should build");
        let project_root = temp_dir.path().join("project");
        let storage_root = temp_dir.path().join("storage");
        create_dir_all_or_panic(&project_root, "project root should build");

        let writer = SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-repo-search-bootstrap-persist"),
            SearchMaintenancePolicy::default(),
        );
        bootstrap_sample_repo_search_content(&writer, "alpha/repo")
            .await
            .unwrap_or_else(|error| {
                panic!("sample bootstrap should publish repo content: {error}")
            });
        drop(writer);

        let reader = SearchPlaneService::with_paths(
            PathBuf::from(&project_root),
            PathBuf::from(&storage_root),
            SearchManifestKeyspace::new("xiuxian:test:flight-repo-search-bootstrap-persist"),
            SearchMaintenancePolicy::default(),
        );
        let hits = reader
            .search_repo_content_chunks("alpha/repo", "alpha", &HashSet::new(), 5)
            .await
            .unwrap_or_else(|error| {
                panic!("bootstrapped repo should remain searchable after restart: {error}")
            });

        assert!(!hits.is_empty());
        assert!(hits.iter().any(|hit| hit.path == "src/lib.rs"));
    }
}
