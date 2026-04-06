use std::path::Path;
use std::process::Command;

use serde::Serialize;

const TEST_GIT_AUTHOR_NAME: &str = "Xiuxian Test";
const TEST_GIT_AUTHOR_EMAIL: &str = "test@example.com";
const TEST_GIT_COMMIT_TIME: &str = "1700000000 +0000";

pub(crate) fn assert_studio_json_snapshot(name: &str, value: impl Serialize) {
    insta::with_settings!({
        snapshot_path => "../../../tests/snapshots/gateway/studio",
        prepend_module_to_snapshot => false,
        sort_maps => true,
    }, {
        insta::assert_json_snapshot!(name, value);
    });
}

pub(crate) fn assert_wendao_json_snapshot(name: &str, value: impl Serialize) {
    insta::with_settings!({
        snapshot_path => "../../../tests/snapshots/wendao",
        prepend_module_to_snapshot => false,
        sort_maps => true,
    }, {
        insta::assert_json_snapshot!(name, value);
    });
}

pub(crate) fn round_f64(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

pub(crate) fn init_git_repository(path: impl AsRef<Path>) {
    let path = path.as_ref();
    let path_arg = path.display().to_string();
    run_git(
        None,
        &["init", "--quiet", path_arg.as_str()],
        "init git repository",
    );
}

pub(crate) fn add_git_remote(path: impl AsRef<Path>, remote_name: &str, remote_url: &str) {
    run_git(
        Some(path.as_ref()),
        &["remote", "add", remote_name, remote_url],
        "add git remote",
    );
}

pub(crate) fn commit_all(path: impl AsRef<Path>, message: &str) {
    let path = path.as_ref();
    run_git(Some(path), &["add", "--all"], "stage git fixture contents");
    run_git(
        Some(path),
        &["commit", "--quiet", "-m", message],
        "commit git fixture contents",
    );
    run_git(
        Some(path),
        &["branch", "-M", "main"],
        "rename branch to main",
    );
}

fn run_git(cwd: Option<&Path>, args: &[&str], context: &str) {
    let mut command = Command::new("git");
    if let Some(cwd) = cwd {
        command.arg("-C").arg(cwd);
    }
    let output = command
        .args(args)
        .env("GIT_AUTHOR_NAME", TEST_GIT_AUTHOR_NAME)
        .env("GIT_AUTHOR_EMAIL", TEST_GIT_AUTHOR_EMAIL)
        .env("GIT_COMMITTER_NAME", TEST_GIT_AUTHOR_NAME)
        .env("GIT_COMMITTER_EMAIL", TEST_GIT_AUTHOR_EMAIL)
        .env("GIT_AUTHOR_DATE", TEST_GIT_COMMIT_TIME)
        .env("GIT_COMMITTER_DATE", TEST_GIT_COMMIT_TIME)
        .output()
        .unwrap_or_else(|error| panic!("{context}: {error}"));
    if output.status.success() {
        return;
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = match (stderr.is_empty(), stdout.is_empty()) {
        (false, true) => stderr,
        (true, false) => stdout,
        (false, false) => format!("{stderr}; stdout: {stdout}"),
        (true, true) => "unknown git error".to_string(),
    };
    panic!("{context}: git {} failed: {detail}", args.join(" "));
}
