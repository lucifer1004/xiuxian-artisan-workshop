use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::Duration;

use gix::Repository;
use gix::bstr::ByteSlice;
use gix::remote::Direction;

use crate::spec::{RepoRefreshPolicy, RevisionSelector};
use crate::sync::SyncMode;

const MANAGED_REMOTE_RETRY_ATTEMPTS: usize = 3;
const MANAGED_GIT_OPEN_RETRY_ATTEMPTS: usize = 5;
const MANAGED_GIT_OPEN_RETRY_DELAY: Duration = Duration::from_millis(100);

pub(crate) type RepositoryHandle = Repository;

#[derive(Debug, Clone)]
pub(crate) struct BackendError {
    message: String,
}

impl BackendError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for BackendError {}

pub(crate) fn should_fetch(refresh: RepoRefreshPolicy, mode: SyncMode) -> bool {
    matches!(mode, SyncMode::Refresh)
        || (matches!(mode, SyncMode::Ensure) && matches!(refresh, RepoRefreshPolicy::Fetch))
}

pub(crate) fn clone_bare_with_retry(
    upstream_url: &str,
    mirror_root: &Path,
) -> Result<RepositoryHandle, BackendError> {
    retry_remote_operation(|| {
        let args = vec![
            "clone".to_string(),
            "--mirror".to_string(),
            "--quiet".to_string(),
            upstream_url.to_string(),
            mirror_root.display().to_string(),
        ];
        run_git_command(None, &args)?;
        open_bare_with_retry(mirror_root)
    })
}

pub(crate) fn clone_checkout_from_mirror(
    mirror_origin: &str,
    checkout_root: &Path,
) -> Result<RepositoryHandle, BackendError> {
    let args = vec![
        "clone".to_string(),
        "--quiet".to_string(),
        mirror_origin.to_string(),
        checkout_root.display().to_string(),
    ];
    run_git_command(None, &args)?;
    open_checkout_with_retry(checkout_root)
}

pub(crate) fn fetch_origin_with_retry(repository: &RepositoryHandle) -> Result<(), BackendError> {
    retry_remote_operation(|| fetch_origin_once(repository))
}

fn fetch_origin_once(repository: &RepositoryHandle) -> Result<(), BackendError> {
    let args = vec![
        "fetch".to_string(),
        "--quiet".to_string(),
        "--tags".to_string(),
        "origin".to_string(),
    ];
    run_git_command(Some(git_command_directory(repository).as_path()), &args)
}

pub(crate) fn probe_remote_target_revision_with_retry(
    repository: &RepositoryHandle,
    revision: Option<&RevisionSelector>,
) -> Result<Option<String>, BackendError> {
    retry_remote_operation(|| probe_remote_target_revision_once(repository, revision))
}

fn probe_remote_target_revision_once(
    repository: &RepositoryHandle,
    revision: Option<&RevisionSelector>,
) -> Result<Option<String>, BackendError> {
    if let Some(RevisionSelector::Commit(sha)) = revision {
        return Ok(Some(sha.clone()));
    }

    let remote = repository.find_remote("origin").map_err(error_message)?;
    let connection = remote.connect(Direction::Fetch).map_err(error_message)?;
    let (ref_map, _handshake) = connection
        .ref_map(gix::progress::Discard, remote_probe_options(revision)?)
        .map_err(error_message)?;

    Ok(match revision {
        Some(RevisionSelector::Branch(branch)) => remote_ref_target_revision(
            &ref_map.remote_refs,
            format!("refs/heads/{branch}").as_str(),
        ),
        Some(RevisionSelector::Tag(tag)) => {
            remote_ref_target_revision(&ref_map.remote_refs, format!("refs/tags/{tag}").as_str())
        }
        None => default_remote_head_revision(&ref_map.remote_refs),
        Some(RevisionSelector::Commit(_)) => unreachable!("commit revisions return early"),
    })
}

fn remote_probe_options(
    revision: Option<&RevisionSelector>,
) -> Result<gix::remote::ref_map::Options, BackendError> {
    let probe_refspec = match revision {
        Some(RevisionSelector::Branch(branch)) => Some(format!("refs/heads/{branch}")),
        Some(RevisionSelector::Tag(tag)) => Some(format!("refs/tags/{tag}")),
        Some(RevisionSelector::Commit(_)) => None,
        None => Some("HEAD".to_string()),
    };

    let mut options = gix::remote::ref_map::Options {
        prefix_from_spec_as_filter_on_remote: false,
        ..Default::default()
    };
    if let Some(probe_refspec) = probe_refspec {
        options.extra_refspecs.push(
            gix::refspec::parse(
                probe_refspec.as_str().into(),
                gix::refspec::parse::Operation::Fetch,
            )
            .map_err(error_message)?
            .to_owned(),
        );
    }
    Ok(options)
}

fn default_remote_head_revision(remote_refs: &[gix::protocol::handshake::Ref]) -> Option<String> {
    remote_ref_target_revision(remote_refs, "HEAD")
}

fn remote_ref_target_revision(
    remote_refs: &[gix::protocol::handshake::Ref],
    full_ref_name: &str,
) -> Option<String> {
    remote_refs
        .iter()
        .find(|reference| remote_ref_name(reference) == full_ref_name.as_bytes().as_bstr())
        .and_then(remote_ref_revision)
}

fn remote_ref_name(reference: &gix::protocol::handshake::Ref) -> &gix::bstr::BStr {
    match reference {
        gix::protocol::handshake::Ref::Peeled { full_ref_name, .. }
        | gix::protocol::handshake::Ref::Direct { full_ref_name, .. }
        | gix::protocol::handshake::Ref::Symbolic { full_ref_name, .. }
        | gix::protocol::handshake::Ref::Unborn { full_ref_name, .. } => full_ref_name.as_bstr(),
    }
}

fn remote_ref_revision(reference: &gix::protocol::handshake::Ref) -> Option<String> {
    match reference {
        gix::protocol::handshake::Ref::Peeled { object, .. }
        | gix::protocol::handshake::Ref::Direct { object, .. }
        | gix::protocol::handshake::Ref::Symbolic { object, .. } => Some(object.to_string()),
        gix::protocol::handshake::Ref::Unborn { .. } => None,
    }
}

pub(crate) fn open_bare_with_retry(path: &Path) -> Result<RepositoryHandle, BackendError> {
    retry_git_open_operation(|| {
        let repository = gix::open(path.to_path_buf()).map_err(error_message)?;
        if repository.is_bare() {
            Ok(repository)
        } else {
            Err(BackendError::new(format!(
                "repository `{}` is not bare",
                path.display()
            )))
        }
    })
}

pub(crate) fn open_checkout_with_retry(path: &Path) -> Result<RepositoryHandle, BackendError> {
    retry_git_open_operation(|| {
        let repository = gix::open(path.to_path_buf()).map_err(error_message)?;
        if repository.is_bare() {
            Err(BackendError::new(format!(
                "repository `{}` is bare and cannot serve as checkout",
                path.display()
            )))
        } else {
            Ok(repository)
        }
    })
}

fn retry_git_open_operation<T>(
    mut operation: impl FnMut() -> Result<T, BackendError>,
) -> Result<T, BackendError> {
    let mut attempts = 0usize;
    loop {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error)
                if attempts + 1 < MANAGED_GIT_OPEN_RETRY_ATTEMPTS
                    && retryable_git_open_error_message(error.message()) =>
            {
                attempts += 1;
                thread::sleep(MANAGED_GIT_OPEN_RETRY_DELAY);
            }
            Err(error) => return Err(error),
        }
    }
}

pub(crate) fn retryable_git_open_error_message(message: &str) -> bool {
    message.to_ascii_lowercase().contains("too many open files")
}

fn retry_remote_operation<T>(
    mut operation: impl FnMut() -> Result<T, BackendError>,
) -> Result<T, BackendError> {
    let mut attempt = 1;
    loop {
        match operation() {
            Ok(value) => return Ok(value),
            Err(error) => {
                if attempt >= MANAGED_REMOTE_RETRY_ATTEMPTS
                    || !is_retryable_remote_error_message(error.message())
                {
                    return Err(error);
                }
                thread::sleep(retry_delay_for_attempt(attempt));
                attempt += 1;
            }
        }
    }
}

pub(crate) fn is_retryable_remote_error_message(message: &str) -> bool {
    let lower = message.to_ascii_lowercase();
    [
        "can't assign requested address",
        "failed to connect",
        "could not connect",
        "timed out",
        "timeout",
        "temporary failure",
        "connection reset",
        "connection refused",
        "connection aborted",
        "network is unreachable",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

fn retry_delay_for_attempt(attempt: usize) -> Duration {
    match attempt {
        0 | 1 => Duration::from_millis(250),
        2 => Duration::from_millis(500),
        _ => Duration::from_secs(1),
    }
}

pub(crate) fn ensure_remote_url(
    repository: &mut RepositoryHandle,
    remote_name: &str,
    expected_url: &str,
) -> Result<bool, BackendError> {
    let config_path = local_config_path(repository);
    let mut config = load_local_config(&config_path)?;

    match current_remote_url(repository, remote_name) {
        Some(current) if current == expected_url => Ok(false),
        Some(_) => {
            repository
                .find_remote(remote_name)
                .map_err(error_message)?
                .with_url_without_url_rewrite(expected_url)
                .map_err(error_message)?
                .save_to(&mut config)
                .map_err(error_message)?;
            write_local_config(&config, &config_path)?;
            repository.reload().map_err(error_message)?;
            Ok(true)
        }
        None => {
            let fetch_refspec = format!("+refs/heads/*:refs/remotes/{remote_name}/*");
            let mut remote = repository
                .remote_at_without_url_rewrite(expected_url)
                .map_err(error_message)?
                .with_refspecs(Some(fetch_refspec.as_str()), Direction::Fetch)
                .map_err(error_message)?;
            remote
                .save_as_to(remote_name, &mut config)
                .map_err(error_message)?;
            write_local_config(&config, &config_path)?;
            repository.reload().map_err(error_message)?;
            Ok(true)
        }
    }
}

fn local_config_path(repository: &RepositoryHandle) -> PathBuf {
    repository
        .config_snapshot()
        .plumbing()
        .meta()
        .path
        .clone()
        .unwrap_or_else(|| repository.git_dir().join("config"))
}

fn load_local_config(path: &Path) -> Result<gix::config::File<'static>, BackendError> {
    gix::config::File::from_path_no_includes(path.to_path_buf(), gix::config::Source::Local)
        .map_err(|error| {
            BackendError::new(format!(
                "failed to load local git config `{}`: {error}",
                path.display()
            ))
        })
}

fn write_local_config(
    config: &gix::config::File<'static>,
    path: &Path,
) -> Result<(), BackendError> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .map_err(|error| {
            BackendError::new(format!(
                "failed to open local git config `{}` for write: {error}",
                path.display()
            ))
        })?;
    config.write_to(&mut file).map_err(|error| {
        BackendError::new(format!(
            "failed to write local git config `{}`: {error}",
            path.display()
        ))
    })
}

pub(crate) fn checkout_detached_to_revision(
    repository: &RepositoryHandle,
    revision: &str,
) -> Result<(), BackendError> {
    let args = vec![
        "checkout".to_string(),
        "--detach".to_string(),
        "--quiet".to_string(),
        revision.to_string(),
    ];
    run_git_command(Some(git_command_directory(repository).as_path()), &args)
}

fn current_remote_url(repository: &RepositoryHandle, remote_name: &str) -> Option<String> {
    repository
        .find_remote(remote_name)
        .ok()
        .and_then(|remote| remote.url(Direction::Fetch).map(display_remote_url))
}

fn display_remote_url(url: &gix::Url) -> String {
    if url.scheme == gix::url::Scheme::File {
        return gix::path::from_bstr(url.path.as_bstr())
            .into_owned()
            .display()
            .to_string();
    }
    url.to_bstring().to_string()
}

fn git_command_directory(repository: &RepositoryHandle) -> PathBuf {
    repository
        .workdir()
        .unwrap_or_else(|| repository.git_dir())
        .to_path_buf()
}

fn run_git_command(cwd: Option<&Path>, args: &[String]) -> Result<(), BackendError> {
    let output = run_git_command_capture(cwd, args)?;
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if stderr.is_empty() {
        stdout
    } else if stdout.is_empty() {
        stderr
    } else {
        format!("{stderr}; stdout: {stdout}")
    };
    Err(BackendError::new(format!(
        "git {} failed: {}",
        args.join(" "),
        if detail.is_empty() {
            "unknown error".to_string()
        } else {
            detail
        }
    )))
}

fn run_git_command_capture(
    cwd: Option<&Path>,
    args: &[String],
) -> Result<std::process::Output, BackendError> {
    let mut command = Command::new("git");
    if let Some(cwd) = cwd {
        command.arg("-C").arg(cwd);
    }
    command.args(args);

    command.output().map_err(|error| {
        BackendError::new(format!(
            "failed to spawn git with args `{}`: {error}",
            args.join(" ")
        ))
    })
}

fn error_message(error: impl std::fmt::Display) -> BackendError {
    BackendError::new(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::process::Command;

    use gix::protocol::handshake::Ref;
    use tempfile::tempdir;

    use super::{
        RevisionSelector, clone_bare_with_retry, default_remote_head_revision,
        is_retryable_remote_error_message, probe_remote_target_revision_with_retry,
        remote_ref_target_revision, retry_delay_for_attempt,
    };

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

    fn describe_remote_refs(repository: &super::RepositoryHandle) -> String {
        let remote = must(repository.find_remote("origin"), "find remote");
        let connection = must(remote.connect(super::Direction::Fetch), "connect");
        let (ref_map, _handshake) = must(
            connection.ref_map(
                gix::progress::Discard,
                must(super::remote_probe_options(None), "probe options"),
            ),
            "ref map",
        );
        format!("{:?}", ref_map.remote_refs)
    }
}
