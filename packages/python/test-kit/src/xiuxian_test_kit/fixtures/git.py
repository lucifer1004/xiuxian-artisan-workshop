"""Git-related test fixtures."""

import subprocess

import pytest
from xiuxian_wendao_py.compat.config import PRJ_DIRS, reset_config_paths_cache
from xiuxian_wendao_py.compat.runtime import clear_project_root_cache


def _reset_git_root_caches() -> None:
    """Reset project-root related caches for isolated git test repos."""
    clear_project_root_cache()
    PRJ_DIRS.clear_cache()
    reset_config_paths_cache()


@pytest.fixture
def temp_git_repo(tmp_path, monkeypatch):
    """Create a temporary git repository for testing."""
    monkeypatch.delenv("PRJ_ROOT", raising=False)
    _reset_git_root_caches()

    subprocess.run(["git", "init"], cwd=tmp_path, capture_output=True, check=True)
    subprocess.run(["git", "config", "user.email", "test@example.com"], cwd=tmp_path, check=True)
    subprocess.run(["git", "config", "user.name", "Test User"], cwd=tmp_path, check=True)
    (tmp_path / "README.md").write_text("# Test Repo")
    subprocess.run(["git", "add", "."], cwd=tmp_path, check=True)
    subprocess.run(["git", "commit", "-m", "Initial commit"], cwd=tmp_path, check=True)
    try:
        yield tmp_path
    finally:
        # Avoid leaking temporary git root into cached project-root resolution.
        _reset_git_root_caches()


@pytest.fixture
def git_repo(temp_git_repo):
    """Alias for temp_git_repo."""
    return temp_git_repo


@pytest.fixture
def git_test_env(temp_git_repo, monkeypatch):
    """
    Set up a git test environment.

    Changes CWD to the temp repo and clears global caches to ensure
    ConfigPaths and other singletons pick up the new root.
    """
    monkeypatch.chdir(temp_git_repo)
    monkeypatch.delenv("PRJ_ROOT", raising=False)

    # Reset caches to pick up new CWD as project root
    _reset_git_root_caches()

    return temp_git_repo


@pytest.fixture
def gitops_verifier(git_test_env):
    """Fixture to verify GitOps states in the git test environment."""
    from xiuxian_test_kit.gitops import GitOpsVerifier

    return GitOpsVerifier(git_test_env)
