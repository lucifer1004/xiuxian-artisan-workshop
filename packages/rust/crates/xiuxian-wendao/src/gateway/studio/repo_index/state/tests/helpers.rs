use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use git2::{Repository, Signature};

use crate::analyzers::registry::PluginRegistry;
use crate::analyzers::{RegisteredRepository, RepositoryPluginConfig, RepositoryRefreshPolicy};
use crate::gateway::studio::repo_index::state::coordinator::RepoIndexCoordinator;

pub(crate) fn repo(id: &str, path: &str) -> RegisteredRepository {
    RegisteredRepository {
        id: id.to_string(),
        path: Some(PathBuf::from(path)),
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
    }
}

pub(crate) fn remote_repo(id: &str, url: &str) -> RegisteredRepository {
    RegisteredRepository {
        id: id.to_string(),
        path: None,
        url: Some(url.to_string()),
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Id("julia".to_string())],
    }
}

pub(crate) fn new_coordinator(
    search_plane: crate::search_plane::SearchPlaneService,
) -> RepoIndexCoordinator {
    RepoIndexCoordinator::new(
        PathBuf::from("."),
        Arc::new(PluginRegistry::new()),
        search_plane,
    )
}

pub(crate) fn init_test_repository(root: &std::path::Path) {
    let repository =
        Repository::init(root).unwrap_or_else(|error| panic!("init repository: {error}"));
    fs::write(root.join("Project.toml"), "name = \"RepoIndexWarmStart\"\n")
        .unwrap_or_else(|error| panic!("write project file: {error}"));

    let mut index = repository
        .index()
        .unwrap_or_else(|error| panic!("open index: {error}"));
    index
        .add_path(std::path::Path::new("Project.toml"))
        .unwrap_or_else(|error| panic!("stage project file: {error}"));
    let tree_id = index
        .write_tree()
        .unwrap_or_else(|error| panic!("write tree: {error}"));
    let tree = repository
        .find_tree(tree_id)
        .unwrap_or_else(|error| panic!("find tree: {error}"));
    let signature = Signature::now("repo-index-test", "repo-index-test@example.com")
        .unwrap_or_else(|error| panic!("signature: {error}"));
    repository
        .commit(Some("HEAD"), &signature, &signature, "init", &tree, &[])
        .unwrap_or_else(|error| panic!("commit: {error}"));
}
