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
    let repository = Repository::init(root).expect("init repository");
    fs::write(root.join("Project.toml"), "name = \"RepoIndexWarmStart\"\n")
        .expect("write project file");

    let mut index = repository.index().expect("open index");
    index
        .add_path(std::path::Path::new("Project.toml"))
        .expect("stage project file");
    let tree_id = index.write_tree().expect("write tree");
    let tree = repository.find_tree(tree_id).expect("find tree");
    let signature =
        Signature::now("repo-index-test", "repo-index-test@example.com").expect("signature");
    repository
        .commit(Some("HEAD"), &signature, &signature, "init", &tree, &[])
        .expect("commit");
}
