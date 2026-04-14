"""Unit tests for scripts/sync_wendao_julia_locks.py."""

from __future__ import annotations

import subprocess
import sys
from functools import lru_cache
from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
from textwrap import dedent
from typing import TYPE_CHECKING

import pytest
from xiuxian_foundation.config.prj import get_project_root

if TYPE_CHECKING:
    from types import ModuleType


@lru_cache(maxsize=1)
def _load_script_module() -> ModuleType:
    script_path = get_project_root() / "scripts" / "sync_wendao_julia_locks.py"
    spec = spec_from_file_location("sync_wendao_julia_locks_script", script_path)
    assert spec is not None
    assert spec.loader is not None
    module = module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def _git(cwd: Path, *args: str, check: bool = True) -> str:
    result = subprocess.run(
        ["git", *args],
        cwd=cwd,
        check=check,
        text=True,
        capture_output=True,
    )
    return result.stdout.strip()


def _configure_git(repo_path: Path) -> None:
    _git(repo_path, "config", "user.name", "Codex Test")
    _git(repo_path, "config", "user.email", "codex-test@example.com")


def _init_repo(repo_path: Path) -> None:
    repo_path.mkdir(parents=True, exist_ok=True)
    _git(repo_path, "init", "--initial-branch=main")
    _configure_git(repo_path)


def _commit_file(repo_path: Path, relative_path: str, content: str, message: str) -> str:
    file_path = repo_path / relative_path
    file_path.parent.mkdir(parents=True, exist_ok=True)
    file_path.write_text(content, encoding="utf-8")
    _git(repo_path, "add", relative_path)
    _git(repo_path, "commit", "-m", message)
    return _git(repo_path, "rev-parse", "HEAD")


def _write_text(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(dedent(content).lstrip(), encoding="utf-8")


def _write_package_lock_fixtures(
    data_home: Path, arrow_rev: str, wendao_arrow_rev: str, wcp_rev: str
) -> None:
    _write_text(
        data_home / "WendaoArrow.jl" / "Project.toml",
        f"""
        [sources]
        Arrow = {{ url = "https://github.com/JuliaCN/arrow-julia.git", rev = "{arrow_rev}" }}
        ArrowTypes = {{ url = "https://github.com/JuliaCN/arrow-julia.git", subdir = "src/ArrowTypes", rev = "{arrow_rev}" }}
        """,
    )
    _write_text(
        data_home / "WendaoArrow.jl" / "Manifest.toml",
        """
        [[deps.Arrow]]
        git-tree-sha1 = "old-arrow-tree"
        repo-rev = "old-arrow-rev"

        [[deps.ArrowTypes]]
        git-tree-sha1 = "old-arrowtypes-tree"
        repo-rev = "old-arrowtypes-rev"
        """,
    )

    _write_text(
        data_home / "WendaoCodeParser.jl" / "Project.toml",
        f"""
        [sources]
        Arrow = {{rev = "{arrow_rev}", url = "https://github.com/JuliaCN/arrow-julia.git"}}
        ArrowTypes = {{rev = "{arrow_rev}", subdir = "src/ArrowTypes", url = "https://github.com/JuliaCN/arrow-julia.git"}}
        WendaoArrow = {{rev = "{wendao_arrow_rev}", url = "https://github.com/tao3k/WendaoArrow.jl.git"}}
        """,
    )
    _write_text(
        data_home / "WendaoCodeParser.jl" / "Manifest.toml",
        """
        [[deps.Arrow]]
        git-tree-sha1 = "old-arrow-tree"
        repo-rev = "old-arrow-rev"

        [[deps.ArrowTypes]]
        git-tree-sha1 = "old-arrowtypes-tree"
        repo-rev = "old-arrowtypes-rev"

        [[deps.WendaoArrow]]
        git-tree-sha1 = "old-wendaoarrow-tree"
        repo-rev = "old-wendaoarrow-rev"
        """,
    )

    _write_text(
        data_home / "WendaoSearch.jl" / "Project.toml",
        f"""
        [sources]
        Arrow = {{rev = "{arrow_rev}", url = "https://github.com/JuliaCN/arrow-julia.git"}}
        ArrowTypes = {{rev = "{arrow_rev}", subdir = "src/ArrowTypes", url = "https://github.com/JuliaCN/arrow-julia.git"}}
        WendaoArrow = {{rev = "{wendao_arrow_rev}", url = "https://github.com/tao3k/WendaoArrow.jl.git"}}
        WendaoCodeParser = {{rev = "{wcp_rev}", url = "https://github.com/tao3k/WendaoCodeParser.jl.git"}}
        """,
    )
    _write_text(
        data_home / "WendaoSearch.jl" / "Manifest.toml",
        """
        [[deps.Arrow]]
        git-tree-sha1 = "old-arrow-tree"
        repo-rev = "old-arrow-rev"

        [[deps.ArrowTypes]]
        git-tree-sha1 = "old-arrowtypes-tree"
        repo-rev = "old-arrowtypes-rev"

        [[deps.WendaoArrow]]
        git-tree-sha1 = "old-wendaoarrow-tree"
        repo-rev = "old-wendaoarrow-rev"

        [[deps.WendaoCodeParser]]
        git-tree-sha1 = "old-wendaocodeparser-tree"
        repo-rev = "old-wendaocodeparser-rev"
        """,
    )


def _init_remote_clone(tmp_path: Path, name: str) -> tuple[Path, Path, Path]:
    remote_path = tmp_path / f"{name}-remote.git"
    subprocess.run(
        ["git", "init", "--bare", "--initial-branch=main", str(remote_path)],
        check=True,
        text=True,
        capture_output=True,
    )

    seed_path = tmp_path / f"{name}-seed"
    subprocess.run(
        ["git", "clone", str(remote_path), str(seed_path)],
        check=True,
        text=True,
        capture_output=True,
    )
    _configure_git(seed_path)
    _commit_file(seed_path, "tracked.txt", "base\n", "seed base")
    _git(seed_path, "push", "origin", "main")

    local_path = tmp_path / f"{name}-local"
    subprocess.run(
        ["git", "clone", str(remote_path), str(local_path)],
        check=True,
        text=True,
        capture_output=True,
    )
    _configure_git(local_path)

    return remote_path, seed_path, local_path


def test_sync_manifest_blocks_from_project_uses_project_revs(tmp_path: Path) -> None:
    module = _load_script_module()
    data_home = tmp_path / ".data"

    arrow_repo = data_home / "arrow-julia"
    wendao_arrow_repo = data_home / "WendaoArrow.jl"
    wcp_repo = data_home / "WendaoCodeParser.jl"
    wsearch_repo = data_home / "WendaoSearch.jl"
    for repo_path in (arrow_repo, wendao_arrow_repo, wcp_repo, wsearch_repo):
        _init_repo(repo_path)

    arrow_rev = _commit_file(arrow_repo, "src/Arrow.jl", "module Arrow end\n", "arrow rev")
    wendao_arrow_rev = _commit_file(
        wendao_arrow_repo,
        "src/WendaoArrow.jl",
        "module WendaoArrow end\n",
        "wendao arrow rev",
    )
    wcp_rev = _commit_file(
        wcp_repo,
        "src/WendaoCodeParser.jl",
        "module WendaoCodeParser end\n",
        "wendao code parser rev",
    )
    _commit_file(
        wsearch_repo,
        "src/WendaoSearch.jl",
        "module WendaoSearch end\n",
        "wendao search rev",
    )

    _write_package_lock_fixtures(data_home, arrow_rev, wendao_arrow_rev, wcp_rev)

    module.sync_manifest_blocks_from_project(data_home)

    arrow_tree = _git(arrow_repo, "rev-parse", f"{arrow_rev}^{{tree}}")
    wendao_arrow_tree = _git(wendao_arrow_repo, "rev-parse", f"{wendao_arrow_rev}^{{tree}}")
    wcp_tree = _git(wcp_repo, "rev-parse", f"{wcp_rev}^{{tree}}")

    wendao_arrow_manifest = (data_home / "WendaoArrow.jl" / "Manifest.toml").read_text(
        encoding="utf-8"
    )
    assert f'repo-rev = "{arrow_rev}"' in wendao_arrow_manifest
    assert f'git-tree-sha1 = "{arrow_tree}"' in wendao_arrow_manifest

    wcp_manifest = (data_home / "WendaoCodeParser.jl" / "Manifest.toml").read_text(encoding="utf-8")
    assert f'repo-rev = "{arrow_rev}"' in wcp_manifest
    assert f'git-tree-sha1 = "{arrow_tree}"' in wcp_manifest
    assert f'repo-rev = "{wendao_arrow_rev}"' in wcp_manifest
    assert f'git-tree-sha1 = "{wendao_arrow_tree}"' in wcp_manifest

    wsearch_manifest = (data_home / "WendaoSearch.jl" / "Manifest.toml").read_text(encoding="utf-8")
    assert f'repo-rev = "{arrow_rev}"' in wsearch_manifest
    assert f'git-tree-sha1 = "{arrow_tree}"' in wsearch_manifest
    assert f'repo-rev = "{wendao_arrow_rev}"' in wsearch_manifest
    assert f'git-tree-sha1 = "{wendao_arrow_tree}"' in wsearch_manifest
    assert f'repo-rev = "{wcp_rev}"' in wsearch_manifest
    assert f'git-tree-sha1 = "{wcp_tree}"' in wsearch_manifest


def test_update_project_sources_for_prod_rewrites_source_revs(tmp_path: Path) -> None:
    module = _load_script_module()
    data_home = tmp_path / ".data"
    _write_package_lock_fixtures(data_home, "old-arrow", "old-wendao-arrow", "old-wcp")

    prod_states = {
        module.ARROW_REPO_KEY: module.RepoState(rev="a" * 40, tree="1" * 40),
        module.WENDAO_ARROW_REPO_KEY: module.RepoState(rev="b" * 40, tree="2" * 40),
        module.WENDAO_CODE_PARSER_REPO_KEY: module.RepoState(rev="c" * 40, tree="3" * 40),
        module.WENDAO_SEARCH_REPO_KEY: module.RepoState(rev="d" * 40, tree="4" * 40),
    }

    module.update_project_sources_for_prod(data_home, prod_states)

    wendao_arrow_project = (data_home / "WendaoArrow.jl" / "Project.toml").read_text(
        encoding="utf-8"
    )
    assert (
        'Arrow = { url = "https://github.com/JuliaCN/arrow-julia.git", rev = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }'
        in wendao_arrow_project
    )
    assert (
        'ArrowTypes = { url = "https://github.com/JuliaCN/arrow-julia.git", subdir = "src/ArrowTypes", rev = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }'
        in wendao_arrow_project
    )

    wcp_project = (data_home / "WendaoCodeParser.jl" / "Project.toml").read_text(encoding="utf-8")
    assert (
        'Arrow = {rev = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", url = "https://github.com/JuliaCN/arrow-julia.git"}'
        in wcp_project
    )
    assert (
        'WendaoArrow = {rev = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", url = "https://github.com/tao3k/WendaoArrow.jl.git"}'
        in wcp_project
    )

    wsearch_project = (data_home / "WendaoSearch.jl" / "Project.toml").read_text(encoding="utf-8")
    assert (
        'WendaoCodeParser = {rev = "cccccccccccccccccccccccccccccccccccccccc", url = "https://github.com/tao3k/WendaoCodeParser.jl.git"}'
        in wsearch_project
    )


def test_sync_repo_to_remote_branch_switches_to_main_and_fast_forwards(tmp_path: Path) -> None:
    module = _load_script_module()
    _remote_path, seed_path, local_path = _init_remote_clone(tmp_path, "arrow-julia")

    _git(local_path, "switch", "-c", "feature/dev")
    expected_rev = _commit_file(seed_path, "tracked.txt", "remote update\n", "remote update")
    _git(seed_path, "push", "origin", "main")

    repo_state = module.sync_repo_to_remote_branch(local_path, "main")

    assert _git(local_path, "branch", "--show-current") == "main"
    assert _git(local_path, "rev-parse", "HEAD") == expected_rev
    assert repo_state.rev == expected_rev
    assert repo_state.tree == _git(local_path, "rev-parse", "HEAD^{tree}")


def test_sync_repo_to_remote_branch_rejects_dirty_worktree(tmp_path: Path) -> None:
    module = _load_script_module()
    _remote_path, _seed_path, local_path = _init_remote_clone(tmp_path, "wendao-search")

    _git(local_path, "switch", "-c", "feature/dev")
    (local_path / "dirty.txt").write_text("dirty\n", encoding="utf-8")

    with pytest.raises(SystemExit, match="clean worktree"):
        module.sync_repo_to_remote_branch(local_path, "main")
