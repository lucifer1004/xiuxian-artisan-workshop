#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::time::Duration;

use git2::{Repository, Signature};
use uuid::Uuid;
use xiuxian_io::PrjDirs;

use super::*;

fn init_test_repository(root: &Path) {
    let repository = Repository::init(root).expect("init repository");
    fs::write(root.join("Project.toml"), "name = \"CheckoutTest\"\n").expect("write file");

    let mut index = repository.index().expect("open index");
    index
        .add_path(Path::new("Project.toml"))
        .expect("stage project file");
    let tree_id = index.write_tree().expect("write tree");
    let tree = repository.find_tree(tree_id).expect("find tree");
    let signature =
        Signature::now("checkout-test", "checkout-test@example.com").expect("signature");
    repository
        .commit(Some("HEAD"), &signature, &signature, "init", &tree, &[])
        .expect("commit");
}

#[test]
fn resolve_repository_source_materializes_remote_checkout_under_prj_data_home() {
    let source = tempfile::tempdir().expect("tempdir");
    init_test_repository(source.path());
    let repo_id = format!("checkout-test-{}", Uuid::new_v4());

    let repository = crate::analyzers::config::RegisteredRepository {
        id: repo_id.clone(),
        path: None,
        url: Some(source.path().display().to_string()),
        git_ref: None,
        refresh: crate::analyzers::config::RepositoryRefreshPolicy::Manual,
        plugins: Vec::new(),
    };
    let target_root = super::namespace::managed_checkout_root_for(&repository);
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
fn managed_repo_paths_follow_ghq_layout_for_remote_urls() {
    let repository = crate::analyzers::config::RegisteredRepository {
        id: "sciml".to_string(),
        path: None,
        url: Some("https://github.com/SciML/BaseModelica.jl.git".to_string()),
        git_ref: None,
        refresh: crate::analyzers::config::RepositoryRefreshPolicy::Manual,
        plugins: Vec::new(),
    };

    assert_eq!(
        super::namespace::managed_checkout_root_for(&repository),
        PrjDirs::data_home()
            .join("xiuxian-wendao")
            .join("repo-intelligence")
            .join("repos")
            .join("github.com")
            .join("SciML")
            .join("BaseModelica.jl")
    );
    assert_eq!(
        super::namespace::managed_mirror_root_for(&repository),
        PrjDirs::data_home()
            .join("xiuxian-wendao")
            .join("repo-intelligence")
            .join("mirrors")
            .join("github.com")
            .join("SciML")
            .join("BaseModelica.jl.git")
    );
}

#[test]
fn managed_repo_paths_support_scp_style_remote_urls() {
    let repository = crate::analyzers::config::RegisteredRepository {
        id: "sciml".to_string(),
        path: None,
        url: Some("git@github.com:SciML/BaseModelica.jl.git".to_string()),
        git_ref: None,
        refresh: crate::analyzers::config::RepositoryRefreshPolicy::Manual,
        plugins: Vec::new(),
    };

    assert_eq!(
        super::namespace::managed_checkout_root_for(&repository),
        PrjDirs::data_home()
            .join("xiuxian-wendao")
            .join("repo-intelligence")
            .join("repos")
            .join("github.com")
            .join("SciML")
            .join("BaseModelica.jl")
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn resolve_repository_source_overrides_managed_remote_url_from_config() {
    let source_a = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(source_a.path().join("src")).expect("create src dir");
    init_test_repository(source_a.path());
    fs::write(
        source_a.path().join("src").join("source_a.jl"),
        "const SOURCE = :a\n",
    )
    .expect("write source a marker");
    let repository_a = Repository::open(source_a.path()).expect("open source a repository");
    let mut index_a = repository_a.index().expect("open source a index");
    index_a
        .add_path(Path::new("src/source_a.jl"))
        .expect("stage source a marker");
    let tree_id_a = index_a.write_tree().expect("write source a tree");
    let tree_a = repository_a
        .find_tree(tree_id_a)
        .expect("find source a tree");
    let signature =
        Signature::now("checkout-test", "checkout-test@example.com").expect("signature");
    let head_a = repository_a
        .head()
        .expect("source a head")
        .peel_to_commit()
        .expect("source a commit");
    repository_a
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "add source a marker",
            &tree_a,
            &[&head_a],
        )
        .expect("commit source a marker");

    let source_b = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(source_b.path().join("src")).expect("create src dir");
    init_test_repository(source_b.path());
    fs::write(
        source_b.path().join("src").join("source_b.jl"),
        "const SOURCE = :b\n",
    )
    .expect("write source b marker");
    let repository_b = Repository::open(source_b.path()).expect("open source b repository");
    let mut index_b = repository_b.index().expect("open source b index");
    index_b
        .add_path(Path::new("src/source_b.jl"))
        .expect("stage source b marker");
    let tree_id_b = index_b.write_tree().expect("write source b tree");
    let tree_b = repository_b
        .find_tree(tree_id_b)
        .expect("find source b tree");
    let head_b = repository_b
        .head()
        .expect("source b head")
        .peel_to_commit()
        .expect("source b commit");
    repository_b
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            "add source b marker",
            &tree_b,
            &[&head_b],
        )
        .expect("commit source b marker");

    let repo_id = format!("managed-url-override-{}", Uuid::new_v4());
    let repository = crate::analyzers::config::RegisteredRepository {
        id: repo_id.clone(),
        path: None,
        url: Some(source_a.path().display().to_string()),
        git_ref: None,
        refresh: crate::analyzers::config::RepositoryRefreshPolicy::Manual,
        plugins: Vec::new(),
    };
    let mirror_root = super::namespace::managed_mirror_root_for(&repository);
    let checkout_root = super::namespace::managed_checkout_root_for(&repository);
    if mirror_root.exists() {
        fs::remove_dir_all(&mirror_root).expect("cleanup stale mirror");
    }
    if checkout_root.exists() {
        fs::remove_dir_all(&checkout_root).expect("cleanup stale checkout");
    }

    let resolved_a = resolve_repository_source(
        &repository,
        Path::new("/Users/guangtao/projects/xiuxian-artisan-workshop"),
        RepositorySyncMode::Ensure,
    )
    .expect("resolve managed checkout from source a");
    let mirror_repository_a =
        Repository::open_bare(resolved_a.mirror_root.as_ref().expect("mirror root"))
            .expect("open mirror repository a");
    assert_eq!(
        super::managed::current_remote_url(&mirror_repository_a, "origin").as_deref(),
        Some(source_a.path().display().to_string().as_str())
    );
    assert!(resolved_a.checkout_root.join("src/source_a.jl").exists());

    let repository = crate::analyzers::config::RegisteredRepository {
        id: repo_id,
        path: None,
        url: Some(source_b.path().display().to_string()),
        git_ref: None,
        refresh: crate::analyzers::config::RepositoryRefreshPolicy::Manual,
        plugins: Vec::new(),
    };
    let resolved_b = resolve_repository_source(
        &repository,
        Path::new("/Users/guangtao/projects/xiuxian-artisan-workshop"),
        RepositorySyncMode::Ensure,
    )
    .expect("resolve managed checkout from source b");
    let mirror_repository_b =
        Repository::open_bare(resolved_b.mirror_root.as_ref().expect("mirror root"))
            .expect("open mirror repository b");
    assert_eq!(
        super::managed::current_remote_url(&mirror_repository_b, "origin").as_deref(),
        Some(source_b.path().display().to_string().as_str())
    );
    assert!(resolved_b.checkout_root.join("src/source_b.jl").exists());

    fs::remove_dir_all(
        resolved_b
            .mirror_root
            .as_ref()
            .expect("mirror root should exist"),
    )
    .expect("cleanup managed mirror");
    fs::remove_dir_all(&resolved_b.checkout_root).expect("cleanup managed checkout");
}

#[test]
fn managed_checkout_lock_reclaims_stale_lockfiles() {
    let repository = crate::analyzers::config::RegisteredRepository {
        id: format!("managed-lock-{}", Uuid::new_v4()),
        path: None,
        url: Some(format!("https://example.com/org/{}.git", Uuid::new_v4())),
        git_ref: None,
        refresh: crate::analyzers::config::RepositoryRefreshPolicy::Manual,
        plugins: Vec::new(),
    };
    let lock_path = super::lock::managed_lock_path_for(&repository);
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent).expect("create lock dir");
    }
    fs::write(&lock_path, "stale").expect("write stale lock");

    let guard = super::lock::acquire_managed_checkout_lock_with_policy(
        lock_path.clone(),
        Duration::from_millis(1),
        Duration::from_millis(5),
        Duration::ZERO,
    )
    .expect("reclaim stale lock");

    assert!(lock_path.exists());
    drop(guard);
    assert!(!lock_path.exists());
}

#[test]
fn managed_checkout_lock_times_out_for_active_lockfiles() {
    let repository = crate::analyzers::config::RegisteredRepository {
        id: format!("managed-lock-busy-{}", Uuid::new_v4()),
        path: None,
        url: Some(format!("https://example.com/org/{}.git", Uuid::new_v4())),
        git_ref: None,
        refresh: crate::analyzers::config::RepositoryRefreshPolicy::Manual,
        plugins: Vec::new(),
    };
    let lock_path = super::lock::managed_lock_path_for(&repository);
    if let Some(parent) = lock_path.parent() {
        fs::create_dir_all(parent).expect("create lock dir");
    }
    fs::write(&lock_path, "busy").expect("write active lock");

    let error = super::lock::acquire_managed_checkout_lock_with_policy(
        lock_path.clone(),
        Duration::from_millis(1),
        Duration::from_millis(5),
        Duration::from_secs(60),
    )
    .expect_err("active lock should time out");

    match error {
        crate::analyzers::errors::RepoIntelligenceError::AnalysisFailed { message } => {
            assert!(message.contains("timed out waiting for managed checkout lock"));
        }
        other => panic!("unexpected error: {other:?}"),
    }

    fs::remove_file(&lock_path).expect("cleanup active lock");
}
