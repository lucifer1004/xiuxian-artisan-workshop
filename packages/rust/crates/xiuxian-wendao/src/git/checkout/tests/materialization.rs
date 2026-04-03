use std::fs;
use std::path::Path;

use uuid::Uuid;
use xiuxian_io::PrjDirs;

use crate::analyzers::config::{RegisteredRepository, RepositoryRef, RepositoryRefreshPolicy};
use crate::git::checkout::{
    RepositoryLifecycleState, RepositorySyncMode, ResolvedRepositorySourceKind,
    discover_checkout_metadata, discover_managed_remote_probe_state, resolve_repository_source,
};

use super::helpers::{append_repo_file_and_commit, init_test_repository};

#[test]
fn resolve_repository_source_materializes_remote_checkout_under_prj_data_home() {
    let source = tempfile::tempdir().expect("tempdir");
    init_test_repository(source.path());
    let repo_id = format!("checkout-test-{}", Uuid::new_v4());

    let repository = RegisteredRepository {
        id: repo_id.clone(),
        path: None,
        url: Some(source.path().display().to_string()),
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Manual,
        plugins: Vec::new(),
    };
    let target_root = crate::git::checkout::namespace::managed_checkout_root_for(&repository);
    if target_root.exists() {
        fs::remove_dir_all(&target_root).expect("cleanup stale checkout");
    }

    let resolved = resolve_repository_source(
        &repository,
        Path::new("/Users/guangtao/projects/xiuxian-artisan-workshop"),
        RepositorySyncMode::Ensure,
    )
    .expect("resolve managed checkout");

    assert!(resolved.checkout_root.starts_with(PrjDirs::data_home()));
    assert!(resolved.checkout_root.is_dir());
    assert_eq!(
        resolved.source_kind,
        ResolvedRepositorySourceKind::ManagedRemote
    );
    assert!(resolved.tracking_revision.is_some());
    let metadata =
        discover_checkout_metadata(&resolved.checkout_root).expect("discover checkout metadata");
    assert_eq!(
        metadata.remote_url.as_deref(),
        Some(
            std::fs::canonicalize(
                resolved
                    .mirror_root
                    .as_ref()
                    .expect("managed checkout should expose mirror root"),
            )
            .unwrap_or_else(|_| {
                resolved
                    .mirror_root
                    .clone()
                    .expect("managed checkout should expose mirror root")
            })
            .display()
            .to_string()
            .as_str(),
        )
    );
    let mirror_metadata = discover_checkout_metadata(
        resolved
            .mirror_root
            .as_deref()
            .expect("managed checkout should expose mirror root"),
    )
    .expect("discover managed mirror metadata");
    assert_eq!(
        mirror_metadata.remote_url.as_deref(),
        Some(source.path().display().to_string().as_str())
    );

    fs::remove_dir_all(&resolved.checkout_root).expect("cleanup managed checkout");
}

#[test]
fn resolve_repository_source_reuses_fetch_policy_checkout_when_revision_is_unchanged() {
    let source = tempfile::tempdir().expect("tempdir");
    init_test_repository(source.path());
    let repo_id = format!("checkout-reuse-test-{}", Uuid::new_v4());
    let repository = RegisteredRepository {
        id: repo_id,
        path: None,
        url: Some(source.path().display().to_string()),
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: Vec::new(),
    };

    let first = resolve_repository_source(
        &repository,
        Path::new("/Users/guangtao/projects/xiuxian-artisan-workshop"),
        RepositorySyncMode::Ensure,
    )
    .expect("resolve first ensure");
    let first_revision = discover_checkout_metadata(&first.checkout_root)
        .expect("discover first checkout metadata")
        .revision;
    let second = resolve_repository_source(
        &repository,
        Path::new("/Users/guangtao/projects/xiuxian-artisan-workshop"),
        RepositorySyncMode::Ensure,
    )
    .expect("resolve second ensure");
    let second_metadata = discover_checkout_metadata(&second.checkout_root)
        .expect("discover second checkout metadata");

    assert_eq!(first_revision, second_metadata.revision);
    assert_eq!(second.mirror_state, RepositoryLifecycleState::Reused);
    assert_eq!(second.checkout_state, RepositoryLifecycleState::Reused);
    assert_eq!(
        discover_managed_remote_probe_state(second.mirror_root.as_deref().expect("mirror root"))
            .and_then(|state| state.target_revision),
        second_metadata.revision
    );

    fs::remove_dir_all(second.mirror_root.as_ref().expect("mirror root"))
        .expect("cleanup managed mirror");
    fs::remove_dir_all(&second.checkout_root).expect("cleanup managed checkout");
}

#[test]
fn resolve_repository_source_refreshes_fetch_policy_checkout_when_remote_revision_advances() {
    let source = tempfile::tempdir().expect("tempdir");
    init_test_repository(source.path());
    let repo_id = format!("checkout-refresh-test-{}", Uuid::new_v4());
    let repository = RegisteredRepository {
        id: repo_id,
        path: None,
        url: Some(source.path().display().to_string()),
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: Vec::new(),
    };

    let first = resolve_repository_source(
        &repository,
        Path::new("/Users/guangtao/projects/xiuxian-artisan-workshop"),
        RepositorySyncMode::Ensure,
    )
    .expect("resolve first ensure");
    let first_revision = discover_checkout_metadata(&first.checkout_root)
        .expect("discover first checkout metadata")
        .revision;
    append_repo_file_and_commit(
        source.path(),
        "src/advanced.jl",
        "const ADVANCED = true\n",
        "advance source",
    );

    let second = resolve_repository_source(
        &repository,
        Path::new("/Users/guangtao/projects/xiuxian-artisan-workshop"),
        RepositorySyncMode::Ensure,
    )
    .expect("resolve second ensure after advance");
    let second_metadata = discover_checkout_metadata(&second.checkout_root)
        .expect("discover second checkout metadata");

    assert_ne!(first_revision, second_metadata.revision);
    assert_eq!(second.mirror_state, RepositoryLifecycleState::Refreshed);
    assert_eq!(second.checkout_state, RepositoryLifecycleState::Refreshed);

    fs::remove_dir_all(second.mirror_root.as_ref().expect("mirror root"))
        .expect("cleanup managed mirror");
    fs::remove_dir_all(&second.checkout_root).expect("cleanup managed checkout");
}

#[test]
fn resolve_repository_source_reuses_commit_pinned_fetch_policy_remote() {
    let source = tempfile::tempdir().expect("tempdir");
    init_test_repository(source.path());
    let initial_revision = discover_checkout_metadata(source.path())
        .expect("discover source metadata")
        .revision
        .expect("source revision");
    let repo_id = format!("checkout-commit-pin-test-{}", Uuid::new_v4());
    let repository = RegisteredRepository {
        id: repo_id,
        path: None,
        url: Some(source.path().display().to_string()),
        git_ref: Some(RepositoryRef::Commit(initial_revision.clone())),
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: Vec::new(),
    };

    let first = resolve_repository_source(
        &repository,
        Path::new("/Users/guangtao/projects/xiuxian-artisan-workshop"),
        RepositorySyncMode::Ensure,
    )
    .expect("resolve first ensure");
    append_repo_file_and_commit(
        source.path(),
        "src/future.jl",
        "const FUTURE = true\n",
        "advance source branch without changing pinned commit",
    );

    let second = resolve_repository_source(
        &repository,
        Path::new("/Users/guangtao/projects/xiuxian-artisan-workshop"),
        RepositorySyncMode::Ensure,
    )
    .expect("resolve second ensure for pinned commit");
    let second_metadata = discover_checkout_metadata(&second.checkout_root)
        .expect("discover second checkout metadata");

    assert_eq!(
        second_metadata.revision.as_deref(),
        Some(initial_revision.as_str())
    );
    assert_eq!(second.mirror_state, RepositoryLifecycleState::Reused);
    assert_eq!(second.checkout_state, RepositoryLifecycleState::Reused);

    fs::remove_dir_all(second.mirror_root.as_ref().expect("mirror root"))
        .expect("cleanup managed mirror");
    fs::remove_dir_all(&second.checkout_root).expect("cleanup managed checkout");
    fs::remove_dir_all(&first.checkout_root).ok();
}
