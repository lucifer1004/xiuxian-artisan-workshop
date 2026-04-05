use std::fs;
use std::sync::Arc;

use async_trait::async_trait;
use git2::Repository;
use tempfile::{TempDir, tempdir};
use xiuxian_vector::{
    LanceDataType, LanceField, LanceFloat64Array, LanceRecordBatch, LanceSchema, LanceStringArray,
};
use xiuxian_wendao_runtime::transport::{
    RepoSearchFlightRouteProvider, RerankScoreWeights, WendaoFlightService,
};

use super::build_studio_search_flight_service_with_repo_provider;
use crate::gateway::studio::router::{GatewayState, StudioState};
use crate::gateway::studio::search::build_symbol_index;
use crate::gateway::studio::search::handlers::tests::test_studio_state;
use crate::gateway::studio::types::{UiConfig, UiProjectConfig, UiRepoProjectConfig};

pub(super) struct GatewayStateFixture {
    _temp_dir: TempDir,
    pub(super) state: Arc<GatewayState>,
}

pub(super) fn make_gateway_state_with_docs(docs: &[(&str, &str)]) -> GatewayStateFixture {
    let temp_dir = tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    for (path, contents) in docs {
        let full_path = temp_dir.path().join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("create fixture dirs for {path}: {error}"));
        }
        fs::write(&full_path, contents)
            .unwrap_or_else(|error| panic!("write fixture doc {path}: {error}"));
    }

    let mut studio = test_studio_state();
    studio.project_root = temp_dir.path().to_path_buf();
    studio.config_root = temp_dir.path().to_path_buf();
    studio.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string(), "packages".to_string()],
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

    GatewayStateFixture {
        _temp_dir: temp_dir,
        state: Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(studio),
        }),
    }
}

pub(super) async fn make_gateway_state_with_search_routes() -> GatewayStateFixture {
    let temp_dir = tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let docs = [
        (
            "docs/alpha.md",
            "# Alpha\n\nIntent keyword: alpha.\n\n![Topology](assets/topology.png)\n",
        ),
        (
            "packages/rust/crates/demo/src/lib.rs",
            "pub struct AlphaService;\npub fn alpha_handler() {}\n",
        ),
    ];
    for (path, contents) in docs {
        let full_path = temp_dir.path().join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("create fixture dirs for {path}: {error}"));
        }
        fs::write(&full_path, contents)
            .unwrap_or_else(|error| panic!("write fixture doc {path}: {error}"));
    }

    let mut studio = test_studio_state();
    studio.project_root = temp_dir.path().to_path_buf();
    studio.config_root = temp_dir.path().to_path_buf();
    studio.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string(), "packages".to_string()],
        }],
        repo_projects: Vec::new(),
    });

    let configured_projects = studio.configured_projects();
    let warmed_index = build_symbol_index(
        studio.project_root.as_path(),
        studio.config_root.as_path(),
        configured_projects.as_slice(),
    );
    studio.symbol_index_coordinator.set_ready_index_for_test(
        configured_projects.as_slice(),
        Arc::clone(&studio.symbol_index),
        warmed_index,
    );

    let fingerprint_seed = format!(
        "{}:{}:{}",
        studio.project_root.display(),
        studio.config_root.display(),
        configured_projects.len()
    );
    let knowledge_fingerprint = format!(
        "test:knowledge:{}",
        blake3::hash(fingerprint_seed.as_bytes()).to_hex()
    );
    studio
        .search_plane
        .publish_knowledge_sections_from_projects(
            studio.project_root.as_path(),
            studio.config_root.as_path(),
            &configured_projects,
            knowledge_fingerprint.as_str(),
        )
        .await
        .unwrap_or_else(|error| panic!("publish knowledge sections: {error}"));

    let reference_fingerprint = format!(
        "test:reference:{}",
        blake3::hash(fingerprint_seed.as_bytes()).to_hex()
    );
    studio
        .search_plane
        .publish_reference_occurrences_from_projects(
            studio.project_root.as_path(),
            studio.config_root.as_path(),
            &configured_projects,
            reference_fingerprint.as_str(),
        )
        .await
        .unwrap_or_else(|error| panic!("publish reference occurrences: {error}"));

    let attachment_fingerprint = format!(
        "test:attachment:{}",
        blake3::hash(fingerprint_seed.as_bytes()).to_hex()
    );
    studio
        .search_plane
        .publish_attachments_from_projects(
            studio.project_root.as_path(),
            studio.config_root.as_path(),
            &configured_projects,
            attachment_fingerprint.as_str(),
        )
        .await
        .unwrap_or_else(|error| panic!("publish attachments: {error}"));

    GatewayStateFixture {
        _temp_dir: temp_dir,
        state: Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(studio),
        }),
    }
}

pub(super) fn make_gateway_state_with_repo(repo_files: &[(&str, &str)]) -> GatewayStateFixture {
    let temp_dir = tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    Repository::init(temp_dir.path().join("repo"))
        .unwrap_or_else(|error| panic!("init repo fixture: {error}"));
    for (path, contents) in repo_files {
        let full_path = temp_dir.path().join("repo").join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("create repo fixture dirs for {path}: {error}"));
        }
        fs::write(&full_path, contents)
            .unwrap_or_else(|error| panic!("write repo fixture {path}: {error}"));
    }

    let mut studio = test_studio_state();
    studio.project_root = temp_dir.path().to_path_buf();
    studio.config_root = temp_dir.path().to_path_buf();
    studio.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string()],
        }],
        repo_projects: vec![UiRepoProjectConfig {
            id: "demo".to_string(),
            root: Some("repo".to_string()),
            url: None,
            git_ref: None,
            refresh: None,
            plugins: vec!["julia".to_string()],
        }],
    });

    GatewayStateFixture {
        _temp_dir: temp_dir,
        state: Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(studio),
        }),
    }
}

pub(super) async fn make_gateway_state_with_attachments() -> GatewayStateFixture {
    let temp_dir = tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
    fs::create_dir_all(temp_dir.path().join("docs/assets"))
        .unwrap_or_else(|error| panic!("create docs/assets: {error}"));
    fs::write(
        temp_dir.path().join("docs/alpha.md"),
        "# Alpha\n\n![Topology](assets/topology.png)\n",
    )
    .unwrap_or_else(|error| panic!("write alpha.md: {error}"));

    let mut studio = test_studio_state();
    studio.project_root = temp_dir.path().to_path_buf();
    studio.config_root = temp_dir.path().to_path_buf();
    studio.set_ui_config(UiConfig {
        projects: vec![UiProjectConfig {
            name: "kernel".to_string(),
            root: ".".to_string(),
            dirs: vec!["docs".to_string()],
        }],
        repo_projects: Vec::new(),
    });

    let fingerprint = format!(
        "test:attachment:{}",
        blake3::hash(
            format!(
                "{}:{}:{}",
                studio.project_root.display(),
                studio.config_root.display(),
                studio.configured_projects().len()
            )
            .as_bytes()
        )
        .to_hex()
    );
    studio
        .search_plane
        .publish_attachments_from_projects(
            studio.project_root.as_path(),
            studio.config_root.as_path(),
            &studio.configured_projects(),
            fingerprint.as_str(),
        )
        .await
        .unwrap_or_else(|error| panic!("publish attachments: {error}"));

    GatewayStateFixture {
        _temp_dir: temp_dir,
        state: Arc::new(GatewayState {
            index: None,
            signal_tx: None,
            studio: Arc::new(studio),
        }),
    }
}

#[derive(Debug)]
struct RecordingRepoSearchProvider;

#[async_trait]
impl RepoSearchFlightRouteProvider for RecordingRepoSearchProvider {
    async fn repo_search_batch(
        &self,
        request: &xiuxian_wendao_runtime::transport::RepoSearchFlightRequest,
    ) -> Result<LanceRecordBatch, String> {
        LanceRecordBatch::try_new(
            Arc::new(LanceSchema::new(vec![
                LanceField::new("doc_id", LanceDataType::Utf8, false),
                LanceField::new("score", LanceDataType::Float64, false),
            ])),
            vec![
                Arc::new(LanceStringArray::from(vec![format!(
                    "repo:{}:{}",
                    request.query_text, request.limit
                )])) as _,
                Arc::new(LanceFloat64Array::from(vec![0.99_f64])) as _,
            ],
        )
        .map_err(|error| error.to_string())
    }
}

pub(super) fn build_service(state: Arc<GatewayState>) -> WendaoFlightService {
    build_studio_search_flight_service_with_repo_provider(
        "v2",
        Arc::new(RecordingRepoSearchProvider),
        state,
        3,
        RerankScoreWeights::default(),
    )
    .unwrap_or_else(|error| panic!("build studio flight service: {error}"))
}

#[allow(dead_code)]
pub(super) fn bare_gateway_state() -> Arc<GatewayState> {
    Arc::new(GatewayState {
        index: None,
        signal_tx: None,
        studio: Arc::new(StudioState::new_with_bootstrap_ui_config(Arc::new(
            crate::analyzers::bootstrap_builtin_registry()
                .unwrap_or_else(|error| panic!("bootstrap registry: {error}")),
        ))),
    })
}
