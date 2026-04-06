use std::fs;
use std::path::Path;
use std::process::Command;

use gix::protocol::handshake::Ref;
use tempfile::tempdir;

use crate::spec::RevisionSelector;

use super::checkout::checkout_detached_to_revision;
use super::clone::{clone_bare_with_retry, clone_checkout_from_mirror};
use super::fetch::fetch_origin_with_retry;
use super::probe::{
    default_remote_head_revision, probe_remote_target_revision_with_retry, remote_probe_options,
    remote_ref_target_revision,
};
use super::retry::{is_retryable_remote_error_message, retry_delay_for_attempt};
use super::types::RepositoryHandle;

const TEST_AUTHOR_NAME: &str = "backend-test";
const TEST_AUTHOR_EMAIL: &str = "backend-test@example.com";

fn must<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) -> T {
    result.unwrap_or_else(|error| panic!("{context}: {error}"))
}

fn object_id(hex: &[u8]) -> gix::hash::ObjectId {
    must(gix::hash::ObjectId::from_hex(hex), "parse object id")
}

fn temp_dir() -> tempfile::TempDir {
    must(tempdir(), "create tempdir")
}

#[test]
fn retryable_remote_error_message_matches_transient_transport_failures() {
    assert!(is_retryable_remote_error_message(
        "failed to connect to github.com: Can't assign requested address; class=Os (2)"
    ));
    assert!(is_retryable_remote_error_message(
        "connection reset by peer while fetching packfile"
    ));
    assert!(is_retryable_remote_error_message(
        "operation timed out after 30 seconds"
    ));
}

#[test]
fn retryable_remote_error_message_rejects_non_transient_failures() {
    assert!(!is_retryable_remote_error_message(
        "authentication required but no callback set"
    ));
    assert!(!is_retryable_remote_error_message("reference not found"));
}

#[test]
fn retry_delay_for_attempt_caps_backoff_growth() {
    assert_eq!(retry_delay_for_attempt(1).as_millis(), 250);
    assert_eq!(retry_delay_for_attempt(2).as_millis(), 500);
    assert_eq!(retry_delay_for_attempt(3).as_millis(), 1000);
    assert_eq!(retry_delay_for_attempt(9).as_millis(), 1000);
}

#[test]
fn default_remote_head_revision_uses_symbolic_head_object() {
    let remote_refs = vec![
        Ref::Symbolic {
            full_ref_name: "HEAD".into(),
            target: "refs/heads/main".into(),
            tag: None,
            object: object_id(b"0123456789012345678901234567890123456789"),
        },
        Ref::Direct {
            full_ref_name: "refs/heads/main".into(),
            object: object_id(b"0123456789012345678901234567890123456789"),
        },
    ];

    assert_eq!(
        default_remote_head_revision(&remote_refs).as_deref(),
        Some("0123456789012345678901234567890123456789")
    );
}

#[test]
fn remote_ref_target_revision_prefers_peeled_target_object_for_tags() {
    let remote_refs = vec![Ref::Peeled {
        full_ref_name: "refs/tags/v1.0.0".into(),
        tag: object_id(b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
        object: object_id(b"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
    }];

    assert_eq!(
        remote_ref_target_revision(&remote_refs, "refs/tags/v1.0.0").as_deref(),
        Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")
    );
}

#[test]
fn probe_remote_target_revision_resolves_default_head_for_local_mirror() {
    let source = temp_dir();
    init_test_repository(source.path());
    let mirror = temp_dir();
    let repository = must(
        clone_bare_with_retry(source.path().display().to_string().as_str(), mirror.path()),
        "clone bare mirror",
    );
    let expected = head_revision(source.path());

    let probed = must(
        probe_remote_target_revision_with_retry(&repository, None),
        "probe default head",
    );

    assert_eq!(
        probed.as_deref(),
        Some(expected.as_str()),
        "remote refs: {}",
        describe_remote_refs(&repository)
    );
}

#[test]
fn probe_remote_target_revision_resolves_branch_for_local_mirror() {
    let source = temp_dir();
    init_test_repository(source.path());
    create_branch_and_commit(
        source.path(),
        "release",
        "src/release.jl",
        "const RELEASE = true\n",
        "release branch commit",
    );
    let mirror = temp_dir();
    let repository = must(
        clone_bare_with_retry(source.path().display().to_string().as_str(), mirror.path()),
        "clone bare mirror",
    );
    let expected = rev_parse(source.path(), "release");

    let probed = must(
        probe_remote_target_revision_with_retry(
            &repository,
            Some(&RevisionSelector::Branch("release".to_string())),
        ),
        "probe branch",
    );

    assert_eq!(
        probed.as_deref(),
        Some(expected.as_str()),
        "remote refs: {}",
        describe_remote_refs(&repository)
    );
}

#[test]
fn probe_remote_target_revision_resolves_annotated_tag_for_local_mirror() {
    let source = temp_dir();
    init_test_repository(source.path());
    create_annotated_tag(source.path(), "v1.0.0", "release tag");
    let mirror = temp_dir();
    let repository = must(
        clone_bare_with_retry(source.path().display().to_string().as_str(), mirror.path()),
        "clone bare mirror",
    );
    let expected = rev_parse(source.path(), "refs/tags/v1.0.0^{}");

    let probed = must(
        probe_remote_target_revision_with_retry(
            &repository,
            Some(&RevisionSelector::Tag("v1.0.0".to_string())),
        ),
        "probe tag",
    );

    assert_eq!(
        probed.as_deref(),
        Some(expected.as_str()),
        "remote refs: {}",
        describe_remote_refs(&repository)
    );
}

#[test]
fn clone_bare_with_retry_preserves_mirror_branch_and_tag_refs() {
    let source = temp_dir();
    init_test_repository(source.path());
    create_branch_and_commit(
        source.path(),
        "release",
        "src/release.jl",
        "const RELEASE = true\n",
        "release branch commit",
    );
    create_annotated_tag(source.path(), "v1.0.0", "release tag");
    let mirror = temp_dir();
    let _repository = must(
        clone_bare_with_retry(source.path().display().to_string().as_str(), mirror.path()),
        "clone bare mirror",
    );

    assert_eq!(
        rev_parse(mirror.path(), "refs/heads/release"),
        rev_parse(source.path(), "release")
    );
    assert_eq!(
        rev_parse(mirror.path(), "refs/tags/v1.0.0^{}"),
        rev_parse(source.path(), "refs/tags/v1.0.0^{}")
    );
}

#[test]
fn clone_checkout_from_mirror_materializes_worktree_head() {
    let source = temp_dir();
    init_test_repository(source.path());
    let mirror = temp_dir();
    let _mirror_repository = must(
        clone_bare_with_retry(source.path().display().to_string().as_str(), mirror.path()),
        "clone bare mirror",
    );
    let checkout = temp_dir();
    let checkout_root = checkout.path().join("checkout");
    let repository = must(
        clone_checkout_from_mirror(mirror.path().display().to_string().as_str(), &checkout_root),
        "clone checkout from mirror",
    );

    assert_eq!(
        must(repository.head_id(), "checkout head").to_string(),
        head_revision(source.path())
    );
    assert!(checkout_root.join("Project.toml").is_file());
}

#[test]
fn fetch_origin_with_retry_updates_existing_mirror_refs() {
    let source = temp_dir();
    init_test_repository(source.path());
    let mirror = temp_dir();
    let repository = must(
        clone_bare_with_retry(source.path().display().to_string().as_str(), mirror.path()),
        "clone bare mirror",
    );
    let branch = current_branch_name(source.path());
    append_commit_on_current_branch(
        source.path(),
        "src/advanced.jl",
        "const ADVANCED = true\n",
        "advance source",
    );

    must(fetch_origin_with_retry(&repository), "fetch origin");

    assert_eq!(
        rev_parse(mirror.path(), format!("refs/heads/{branch}").as_str()),
        head_revision(source.path())
    );
}

#[test]
fn checkout_detached_to_revision_peels_tag_and_removes_stale_paths() {
    let source = temp_dir();
    init_test_repository(source.path());
    create_annotated_tag(source.path(), "v1.0.0", "release tag");
    append_commit_on_current_branch(
        source.path(),
        "src/advanced.jl",
        "const ADVANCED = true\n",
        "advance source",
    );
    let mirror = temp_dir();
    let _mirror_repository = must(
        clone_bare_with_retry(source.path().display().to_string().as_str(), mirror.path()),
        "clone bare mirror",
    );
    let checkout = temp_dir();
    let checkout_root = checkout.path().join("checkout");
    let mut repository = must(
        clone_checkout_from_mirror(mirror.path().display().to_string().as_str(), &checkout_root),
        "clone checkout from mirror",
    );

    must(
        checkout_detached_to_revision(&mut repository, "refs/tags/v1.0.0"),
        "checkout detached tag",
    );

    assert_eq!(
        must(repository.head_id(), "detached head").to_string(),
        rev_parse(source.path(), "refs/tags/v1.0.0^{}")
    );
    assert_eq!(must(repository.head_name(), "detached head name"), None);
    assert!(!checkout_root.join("src/advanced.jl").exists());
}

#[test]
fn checkout_detached_to_revision_refuses_recursive_directory_removal() {
    let source = temp_dir();
    init_test_repository(source.path());
    create_annotated_tag(source.path(), "v1.0.0", "release tag");
    append_commit_on_current_branch(
        source.path(),
        "src/advanced.jl",
        "const ADVANCED = true\n",
        "advance source",
    );
    let expected_head_before_failure = head_revision(source.path());
    let mirror = temp_dir();
    let _mirror_repository = must(
        clone_bare_with_retry(source.path().display().to_string().as_str(), mirror.path()),
        "clone bare mirror",
    );
    let checkout = temp_dir();
    let checkout_root = checkout.path().join("checkout");
    let mut repository = must(
        clone_checkout_from_mirror(mirror.path().display().to_string().as_str(), &checkout_root),
        "clone checkout from mirror",
    );
    let collided_path = checkout_root.join("src/advanced.jl");
    must(
        fs::remove_file(&collided_path),
        "remove tracked file before collision setup",
    );
    must(
        fs::create_dir_all(&collided_path),
        "create untracked directory collision",
    );
    let preserved_path = collided_path.join("keep.txt");
    must(
        fs::write(&preserved_path, "preserve me\n"),
        "write preserved untracked content",
    );

    let error = match checkout_detached_to_revision(&mut repository, "refs/tags/v1.0.0") {
        Ok(()) => panic!("directory collision should fail"),
        Err(error) => error,
    };

    assert!(
        error.message().contains("refusing recursive removal"),
        "unexpected error: {error}"
    );
    assert!(preserved_path.is_file());
    assert_eq!(
        must(repository.head_id(), "head after failed checkout").to_string(),
        expected_head_before_failure
    );
}

fn init_test_repository(root: &Path) {
    run_git(None, &["init", root.display().to_string().as_str()]);
    must(
        fs::write(root.join("Project.toml"), "name = \"BackendTest\"\n"),
        "write file",
    );
    run_git(Some(root), &["add", "Project.toml"]);
    run_git(Some(root), &["commit", "-m", "init"]);
}

fn create_branch_and_commit(
    root: &Path,
    branch: &str,
    relative_path: &str,
    contents: &str,
    message: &str,
) {
    run_git(Some(root), &["checkout", "-b", branch]);
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        must(fs::create_dir_all(parent), "create parent dir");
    }
    must(fs::write(&path, contents), "write file");
    run_git(Some(root), &["add", relative_path]);
    run_git(Some(root), &["commit", "-m", message]);
}

fn create_annotated_tag(root: &Path, tag: &str, message: &str) {
    run_git(Some(root), &["tag", "-a", tag, "-m", message]);
}

fn append_commit_on_current_branch(
    root: &Path,
    relative_path: &str,
    contents: &str,
    message: &str,
) {
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        must(fs::create_dir_all(parent), "create parent dir");
    }
    must(fs::write(&path, contents), "write file");
    run_git(Some(root), &["add", relative_path]);
    run_git(Some(root), &["commit", "-m", message]);
}

fn run_git(cwd: Option<&Path>, args: &[&str]) {
    let mut command = Command::new("git");
    if let Some(cwd) = cwd {
        command.arg("-C").arg(cwd);
    }
    command
        .args(args)
        .env("GIT_AUTHOR_NAME", TEST_AUTHOR_NAME)
        .env("GIT_AUTHOR_EMAIL", TEST_AUTHOR_EMAIL)
        .env("GIT_COMMITTER_NAME", TEST_AUTHOR_NAME)
        .env("GIT_COMMITTER_EMAIL", TEST_AUTHOR_EMAIL);

    let output = must(command.output(), "run git");
    assert!(
        output.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
}

fn head_revision(root: &Path) -> String {
    rev_parse(root, "HEAD")
}

fn current_branch_name(root: &Path) -> String {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(root)
        .arg("symbolic-ref")
        .arg("--short")
        .arg("HEAD");
    let output = must(command.output(), "run git symbolic-ref");
    assert!(
        output.status.success(),
        "git symbolic-ref HEAD failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn rev_parse(root: &Path, revision: &str) -> String {
    let mut command = Command::new("git");
    command.arg("-C").arg(root).arg("rev-parse").arg(revision);
    let output = must(command.output(), "run git rev-parse");
    assert!(
        output.status.success(),
        "git rev-parse {revision:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn describe_remote_refs(repository: &RepositoryHandle) -> String {
    let remote = must(repository.find_remote("origin"), "find remote");
    let connection = must(remote.connect(gix::remote::Direction::Fetch), "connect");
    let (ref_map, _handshake) = must(
        connection.ref_map(
            gix::progress::Discard,
            must(remote_probe_options(None), "probe options"),
        ),
        "ref map",
    );
    format!("{:?}", ref_map.remote_refs)
}
