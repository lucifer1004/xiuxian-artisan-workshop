use std::fs;
use std::sync::Arc;

use tempfile::tempdir;
use xiuxian_wendao_runtime::transport::AstSearchFlightRouteProvider;

use super::provider::StudioAstSearchFlightRouteProvider;
use crate::gateway::studio::router::GatewayState;
use crate::gateway::studio::search::build_symbol_index;
use crate::gateway::studio::search::handlers::tests::test_studio_state;
use crate::gateway::studio::types::{UiConfig, UiProjectConfig};

#[tokio::test]
async fn studio_ast_flight_provider_materializes_ast_batches() {
    let temp_dir = match tempdir() {
        Ok(temp_dir) => temp_dir,
        Err(error) => panic!("AST provider tempdir should build: {error}"),
    };
    let source_dir = temp_dir.path().join("packages/rust/crates/demo/src");
    if let Err(error) = fs::create_dir_all(&source_dir) {
        panic!("AST provider source dir should build: {error}");
    }
    fs::write(
        source_dir.join("lib.rs"),
        "pub struct AlphaService;\npub fn alpha_handler() {}\n",
    )
    .unwrap_or_else(|error| panic!("AST provider source fixture should write: {error}"));

    let mut studio = test_studio_state();
    studio.project_root = temp_dir.path().to_path_buf();
    studio.config_root = temp_dir.path().to_path_buf();
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

    let provider = StudioAstSearchFlightRouteProvider::new(Arc::new(GatewayState {
        index: None,
        signal_tx: None,
        webhook_url: None,
        studio: Arc::new(studio),
    }));

    let batch = provider
        .ast_search_batch("alpha", 5)
        .await
        .unwrap_or_else(|error| {
            panic!("dedicated AST provider should accept AST requests: {error}")
        });

    assert!(batch.num_rows() >= 1);
}
