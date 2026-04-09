use std::fs;
use std::path::Path;
use std::process::Command;

const TEST_AUTHOR_NAME: &str = "checkout-test";
const TEST_AUTHOR_EMAIL: &str = "checkout-test@example.com";

pub fn must<T, E: std::fmt::Display>(result: Result<T, E>, context: &str) -> T {
    result.unwrap_or_else(|error| panic!("{context}: {error}"))
}

pub fn must_some<T>(value: Option<T>, context: &str) -> T {
    value.unwrap_or_else(|| panic!("{context}"))
}

pub fn temp_dir() -> tempfile::TempDir {
    must(tempfile::tempdir(), "create tempdir")
}

pub fn init_test_repository(root: &Path) {
    run_git(None, &["init", root.display().to_string().as_str()]);
    must(
        fs::write(root.join("Project.toml"), "name = \"CheckoutTest\"\n"),
        "write file",
    );
    run_git(Some(root), &["add", "Project.toml"]);
    run_git(Some(root), &["commit", "-m", "init"]);
}

pub fn append_repo_file_and_commit(
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

pub fn create_branch_and_commit(
    root: &Path,
    branch: &str,
    relative_path: &str,
    contents: &str,
    message: &str,
) {
    run_git(Some(root), &["checkout", "-b", branch]);
    append_repo_file_and_commit(root, relative_path, contents, message);
}

pub fn create_annotated_tag(root: &Path, tag: &str, message: &str) {
    run_git(Some(root), &["tag", "-a", tag, "-m", message]);
}

pub fn head_revision(root: &Path) -> String {
    git_stdout(root, &["rev-parse", "HEAD"]).trim().to_string()
}

pub fn remove_repo_file_and_commit(root: &Path, relative_path: &str, message: &str) {
    let path = root.join(relative_path);
    if path.is_file() {
        must(fs::remove_file(&path), "remove file");
    }
    run_git(
        Some(root),
        &["rm", "--cached", "--ignore-unmatch", relative_path],
    );
    run_git(Some(root), &["add", "-A"]);
    run_git(Some(root), &["commit", "-m", message]);
}

pub fn rename_repo_file_and_commit(root: &Path, from: &str, to: &str, message: &str) {
    let from_path = root.join(from);
    let to_path = root.join(to);
    if let Some(parent) = to_path.parent() {
        must(fs::create_dir_all(parent), "create parent dir");
    }
    must(fs::rename(&from_path, &to_path), "rename file");
    run_git(Some(root), &["add", "-A"]);
    run_git(Some(root), &["commit", "-m", message]);
}

pub fn set_repo_remote_url(root: &Path, remote: &str, url: &str) {
    run_git(Some(root), &["remote", "set-url", remote, url]);
}

fn git_stdout(cwd: &Path, args: &[&str]) -> String {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(cwd)
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
    must(String::from_utf8(output.stdout), "utf8 git stdout")
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
