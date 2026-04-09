use std::fs;
use std::path::Path;
use std::process::Command;

use gix::protocol::handshake::Ref;
use tempfile::tempdir;

use crate::spec::RevisionSelector;

use super::checkout::checkout_detached_to_revision;
use super::clone::{clone_bare_with_retry, clone_checkout_from_mirror};
use super::fetch::fetch_origin_with_retry;
use super::open::{open_bare_with_retry, open_checkout_with_retry};
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
    let refs_output = must(
        Command::new("git")
            .arg("-C")
            .arg(mirror.path())
            .arg("show-ref")
            .output(),
        "list mirror refs",
    );
    let refs = String::from_utf8_lossy(&refs_output.stdout);

    assert!(refs.contains("refs/heads/release"), "refs: {refs}");
    assert!(refs.contains("refs/tags/v1.0.0"), "refs: {refs}");
}

#[test]
fn fetch_origin_with_retry_refreshes_existing_mirror() {
    let source = temp_dir();
    init_test_repository(source.path());
    let mirror = temp_dir();
    let repository = must(
        clone_bare_with_retry(source.path().display().to_string().as_str(), mirror.path()),
        "clone bare mirror",
    );

    create_branch_and_commit(
        source.path(),
        "release",
        "src/release.jl",
        "const RELEASE = true\n",
        "release branch commit",
    );

    must(fetch_origin_with_retry(&repository), "fetch mirror");

    let refs_output = must(
        Command::new("git")
            .arg("-C")
            .arg(mirror.path())
            .arg("show-ref")
            .output(),
        "list mirror refs",
    );
    let refs = String::from_utf8_lossy(&refs_output.stdout);

    assert!(refs.contains("refs/heads/release"), "refs: {refs}");
}

#[test]
fn clone_checkout_from_mirror_materializes_requested_revision() {
    let source = temp_dir();
    init_test_repository(source.path());
    let mirror = temp_dir();
    let _mirror_repository = must(
        clone_bare_with_retry(source.path().display().to_string().as_str(), mirror.path()),
        "clone bare mirror",
    );
    let checkout = temp_dir();
    let expected = head_revision(source.path());
    let mirror_origin = mirror.path().display().to_string();

    let materialized = must(
        clone_checkout_from_mirror(mirror_origin.as_str(), checkout.path()),
        "materialize checkout",
    );

    assert_eq!(head_revision(checkout.path()), expected);
    assert!(materialized.workdir().is_some());
    assert!(checkout.path().join(".git").exists());
}

#[test]
fn checkout_detached_to_revision_resets_existing_checkout() {
    let source = temp_dir();
    init_test_repository(source.path());
    let mirror = temp_dir();
    let mirror_repository = must(
        clone_bare_with_retry(source.path().display().to_string().as_str(), mirror.path()),
        "clone bare mirror",
    );
    let checkout = temp_dir();
    let initial = head_revision(source.path());
    let mirror_origin = mirror.path().display().to_string();
    let mut checkout_repository = must(
        clone_checkout_from_mirror(mirror_origin.as_str(), checkout.path()),
        "materialize checkout",
    );
    assert_eq!(head_revision(checkout.path()), initial);

    create_branch_and_commit(
        source.path(),
        "main",
        "src/runtime.jl",
        "const UPDATED = true\n",
        "update main",
    );
    must(
        fetch_origin_with_retry(&mirror_repository),
        "refresh mirror",
    );
    must(
        fetch_origin_with_retry(&checkout_repository),
        "refresh checkout from mirror",
    );
    let updated = head_revision(source.path());

    must(
        checkout_detached_to_revision(&mut checkout_repository, &updated),
        "reset checkout",
    );

    assert_eq!(head_revision(checkout.path()), updated);
}

#[test]
fn remote_probe_options_include_expected_refspecs() {
    let default = must(remote_probe_options(None), "build default probe options");
    assert_eq!(default.extra_refspecs.len(), 1);
    assert!(format!("{:?}", default.extra_refspecs[0]).contains("HEAD"));

    let branch = must(
        remote_probe_options(Some(&RevisionSelector::Branch("main".to_string()))),
        "build branch probe options",
    );
    assert_eq!(branch.extra_refspecs.len(), 1);
    assert!(format!("{:?}", branch.extra_refspecs[0]).contains("refs/heads/main"));

    let tag = must(
        remote_probe_options(Some(&RevisionSelector::Tag("v1.0.0".to_string()))),
        "build tag probe options",
    );
    assert_eq!(tag.extra_refspecs.len(), 1);
    assert!(format!("{:?}", tag.extra_refspecs[0]).contains("refs/tags/v1.0.0"));
}

#[test]
fn repository_handle_tracks_working_tree_presence() {
    let bare_dir = temp_dir();
    must(
        Command::new("git")
            .arg("init")
            .arg("--bare")
            .arg(bare_dir.path())
            .status(),
        "initialize bare repository",
    );
    let bare = must(
        open_bare_with_retry(bare_dir.path()),
        "open bare repository handle",
    );
    assert!(bare.workdir().is_none());

    let checkout_dir = temp_dir();
    init_test_repository(checkout_dir.path());
    let checkout = must(
        open_checkout_with_retry(checkout_dir.path()),
        "open checkout repository handle",
    );
    assert!(checkout.workdir().is_some());
}

fn init_test_repository(path: &Path) {
    must(
        Command::new("git").arg("init").arg(path).status(),
        "initialize repository",
    );
    configure_identity(path);
    must(
        fs::write(path.join("README.md"), "# fixture\n"),
        "write initial file",
    );
    must(
        Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("add")
            .arg(".")
            .status(),
        "stage initial commit",
    );
    must(
        Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("commit")
            .arg("-m")
            .arg("initial commit")
            .status(),
        "create initial commit",
    );
}

fn configure_identity(path: &Path) {
    for (key, value) in [
        ("user.name", TEST_AUTHOR_NAME),
        ("user.email", TEST_AUTHOR_EMAIL),
    ] {
        must(
            Command::new("git")
                .arg("-C")
                .arg(path)
                .arg("config")
                .arg(key)
                .arg(value)
                .status(),
            "configure repository identity",
        );
    }
}

fn create_branch_and_commit(path: &Path, branch: &str, file: &str, content: &str, message: &str) {
    must(
        Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("checkout")
            .arg("-B")
            .arg(branch)
            .status(),
        "create branch",
    );
    let file_path = path.join(file);
    if let Some(parent) = file_path.parent() {
        must(fs::create_dir_all(parent), "create branch file parent");
    }
    must(fs::write(&file_path, content), "write branch file");
    must(
        Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("add")
            .arg(file)
            .status(),
        "stage branch file",
    );
    must(
        Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("commit")
            .arg("-m")
            .arg(message)
            .status(),
        "commit branch change",
    );
}

fn create_annotated_tag(path: &Path, tag: &str, message: &str) {
    must(
        Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("tag")
            .arg("-a")
            .arg(tag)
            .arg("-m")
            .arg(message)
            .status(),
        "create annotated tag",
    );
}

fn rev_parse(path: &Path, rev: &str) -> String {
    let output = must(
        Command::new("git")
            .arg("-C")
            .arg(path)
            .arg("rev-parse")
            .arg(rev)
            .output(),
        "rev-parse revision",
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn head_revision(path: &Path) -> String {
    rev_parse(path, "HEAD")
}

fn describe_remote_refs(repository: &RepositoryHandle) -> String {
    format!("git_dir={}", repository.git_dir().display())
}
