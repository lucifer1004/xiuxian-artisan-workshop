"""
git/scripts/commit.py - Git commit operations

Write commands (commit, amend, revert) are exposed as plain CLI-callable functions.
Read commands (get_last_commit, get_last_commit_msg) are simple wrappers.
"""

import subprocess
from pathlib import Path


def _run(cmd: list[str], cwd: Path | None = None) -> tuple[str, str, int]:
    """Run command and return stdout, stderr, returncode."""
    result = subprocess.run(cmd, capture_output=True, text=True, cwd=cwd)
    return result.stdout.strip(), result.stderr.strip(), result.returncode


def commit(message: str, project_root: Path | None = None) -> str:
    _run(["git", "add", "-u"], cwd=project_root)

    stdout, stderr, rc = _run(["git", "commit", "-m", message], cwd=project_root)
    if rc == 0:
        return "Commit created successfully"
    raise RuntimeError(f"Commit failed: {stdout} {stderr}")


def commit_with_amend(message: str, project_root: Path | None = None) -> str:
    stdout, stderr, rc = _run(["git", "commit", "--amend", "-m", message], cwd=project_root)
    if rc == 0:
        return "Commit amended successfully"
    raise RuntimeError(f"Amend failed: {stdout} {stderr}")


def commit_no_verify(message: str, project_root: Path | None = None) -> str:
    stdout, stderr, rc = _run(["git", "commit", "--no-verify", "-m", message], cwd=project_root)
    if rc == 0:
        return "Commit created (no-verify)"
    raise RuntimeError(f"Commit failed: {stdout} {stderr}")


def get_last_commit(project_root: Path | None = None) -> str:
    """Retrieves the hash of the most recent commit."""
    stdout, _, rc = _run(["git", "rev-parse", "HEAD"], cwd=project_root)
    return stdout if rc == 0 else ""


def get_last_commit_msg(project_root: Path | None = None) -> str:
    """Retrieves the message of the most recent commit."""
    stdout, _, _ = _run(["git", "log", "-1", "--pretty=%B"], cwd=project_root)
    return stdout


def revert(commit: str, no_commit: bool = False, project_root: Path | None = None) -> str:
    cmd = ["git", "revert"]
    if no_commit:
        cmd.append("--no-commit")
    cmd.append(commit)
    stdout, stderr, rc = _run(cmd, cwd=project_root)
    if rc == 0:
        return "Revert initiated successfully"
    raise RuntimeError(f"Revert failed: {stdout} {stderr}")
