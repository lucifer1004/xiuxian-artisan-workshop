use xiuxian_wendao_core::repo_intelligence::{RegisteredRepository, RepositoryPluginConfig};

use super::julia_parser_summary_allows_safe_incremental_file_for_repository;
use crate::julia_plugin_test_support::common::ensure_linked_julia_parser_summary_service;

fn parser_summary_repository() -> RegisteredRepository {
    RegisteredRepository {
        id: "repo-julia".to_string(),
        plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
        ..RegisteredRepository::default()
    }
}

#[tokio::test]
#[serial_test::serial(julia_live)]
async fn safe_incremental_live_service_distinguishes_leaf_and_root_files()
-> Result<(), Box<dyn std::error::Error>> {
    ensure_linked_julia_parser_summary_service()?;
    let repository = parser_summary_repository();
    let leaf_repository = repository.clone();
    let leaf_is_safe = tokio::task::spawn_blocking(move || {
        julia_parser_summary_allows_safe_incremental_file_for_repository(
            &leaf_repository,
            "src/leaf.jl",
            "alpha() = 2\nbeta() = 3\n",
        )
    })
    .await
    .expect("blocking task should complete")
    .unwrap_or_else(|error| panic!("safe incremental leaf file should decode: {error}"));
    let root_is_safe = tokio::task::spawn_blocking(move || {
        julia_parser_summary_allows_safe_incremental_file_for_repository(
            &repository,
            "src/FixturePkg.jl",
            "module FixturePkg\ninclude(\"leaf.jl\")\nend\n",
        )
    })
    .await
    .expect("blocking task should complete")
    .unwrap_or_else(|error| panic!("root summary should decode: {error}"));

    assert!(
        leaf_is_safe,
        "leaf-only Julia file should stay incremental-safe"
    );
    assert!(
        !root_is_safe,
        "root Julia file should not stay incremental-safe"
    );
    Ok(())
}
