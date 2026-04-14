#!/usr/bin/env python3
"""Synchronize Julia search lock metadata for prod/dev repo workflows."""

from __future__ import annotations

import argparse
import os
import pathlib
import re
import subprocess
import sys
import tomllib
from dataclasses import dataclass


@dataclass(frozen=True)
class RepoSpec:
    """One managed nested repository."""

    repo_dir: str
    branch: str


@dataclass(frozen=True)
class RepoState:
    """Resolved revision and tree metadata for one repository ref."""

    rev: str
    tree: str


@dataclass(frozen=True)
class SourceLineStyle:
    """Formatting rules for one Project.toml source entry."""

    url: str
    compact: bool
    subdir: str | None = None


MODE_DEV = "dev"
MODE_PROD = "prod"

ARROW_REPO_KEY = "arrow-julia"
WENDAO_ARROW_REPO_KEY = "WendaoArrow.jl"
WENDAO_CODE_PARSER_REPO_KEY = "WendaoCodeParser.jl"
WENDAO_SEARCH_REPO_KEY = "WendaoSearch.jl"

MANAGED_REPOS: dict[str, RepoSpec] = {
    ARROW_REPO_KEY: RepoSpec(repo_dir="arrow-julia", branch="main"),
    WENDAO_ARROW_REPO_KEY: RepoSpec(repo_dir="WendaoArrow.jl", branch="main"),
    WENDAO_CODE_PARSER_REPO_KEY: RepoSpec(repo_dir="WendaoCodeParser.jl", branch="main"),
    WENDAO_SEARCH_REPO_KEY: RepoSpec(repo_dir="WendaoSearch.jl", branch="main"),
}

PROJECT_SOURCE_STYLES: dict[str, dict[str, SourceLineStyle]] = {
    "WendaoArrow.jl": {
        "Arrow": SourceLineStyle(
            url="https://github.com/JuliaCN/arrow-julia.git",
            compact=False,
        ),
        "ArrowTypes": SourceLineStyle(
            url="https://github.com/JuliaCN/arrow-julia.git",
            subdir="src/ArrowTypes",
            compact=False,
        ),
    },
    "WendaoCodeParser.jl": {
        "Arrow": SourceLineStyle(
            url="https://github.com/JuliaCN/arrow-julia.git",
            compact=True,
        ),
        "ArrowTypes": SourceLineStyle(
            url="https://github.com/JuliaCN/arrow-julia.git",
            subdir="src/ArrowTypes",
            compact=True,
        ),
        "WendaoArrow": SourceLineStyle(
            url="https://github.com/tao3k/WendaoArrow.jl.git",
            compact=True,
        ),
    },
    "WendaoSearch.jl": {
        "Arrow": SourceLineStyle(
            url="https://github.com/JuliaCN/arrow-julia.git",
            compact=True,
        ),
        "ArrowTypes": SourceLineStyle(
            url="https://github.com/JuliaCN/arrow-julia.git",
            subdir="src/ArrowTypes",
            compact=True,
        ),
        "WendaoArrow": SourceLineStyle(
            url="https://github.com/tao3k/WendaoArrow.jl.git",
            compact=True,
        ),
        "WendaoCodeParser": SourceLineStyle(
            url="https://github.com/tao3k/WendaoCodeParser.jl.git",
            compact=True,
        ),
    },
}

PACKAGE_TO_REPO_KEY: dict[str, str] = {
    "Arrow": ARROW_REPO_KEY,
    "ArrowTypes": ARROW_REPO_KEY,
    "WendaoArrow": WENDAO_ARROW_REPO_KEY,
    "WendaoCodeParser": WENDAO_CODE_PARSER_REPO_KEY,
}


def git_command(repo_path: pathlib.Path, *args: str, capture: bool = True) -> str:
    """Run one git command in the target repository."""

    result = subprocess.run(
        ["git", "-C", str(repo_path), *args],
        check=True,
        text=True,
        capture_output=capture,
    )
    return result.stdout.strip() if capture else ""


def git_has_object(repo_path: pathlib.Path, rev: str) -> bool:
    """Return whether the repository already has the target commit object."""

    result = subprocess.run(
        ["git", "-C", str(repo_path), "cat-file", "-e", f"{rev}^{{commit}}"],
        check=False,
        text=True,
        capture_output=True,
    )
    return result.returncode == 0


def repo_is_clean(repo_path: pathlib.Path) -> bool:
    """Return whether the working tree has no tracked or untracked changes."""

    return git_command(repo_path, "status", "--short") == ""


def ensure_local_branch(repo_path: pathlib.Path, branch: str) -> None:
    """Switch to a local tracking branch, creating it if needed."""

    branch_ref = f"refs/heads/{branch}"
    has_local_branch = (
        subprocess.run(
            ["git", "-C", str(repo_path), "show-ref", "--verify", "--quiet", branch_ref],
            check=False,
            capture_output=True,
            text=True,
        ).returncode
        == 0
    )
    if has_local_branch:
        git_command(repo_path, "switch", branch, capture=False)
        return
    git_command(repo_path, "switch", "--track", "-c", branch, f"origin/{branch}", capture=False)


def sync_repo_to_remote_branch(repo_path: pathlib.Path, branch: str) -> RepoState:
    """Fast-forward one clean repo to the latest remote branch head."""

    if not repo_is_clean(repo_path):
        raise SystemExit(
            f"prod mode requires a clean worktree before switching branches: {repo_path}"
        )
    git_command(repo_path, "fetch", "origin", branch, capture=False)
    ensure_local_branch(repo_path, branch)
    git_command(repo_path, "merge", "--ff-only", f"origin/{branch}", capture=False)
    remote_head = git_command(repo_path, "rev-parse", f"origin/{branch}")
    local_head = git_command(repo_path, "rev-parse", "HEAD")
    if local_head != remote_head:
        raise SystemExit(
            f"prod mode requires {repo_path} to match origin/{branch} exactly; "
            f"local={local_head} remote={remote_head}"
        )
    return RepoState(rev=local_head, tree=git_command(repo_path, "rev-parse", "HEAD^{tree}"))


def resolve_repo_state_for_rev(repo_path: pathlib.Path, rev: str) -> RepoState:
    """Resolve one revision/tree pair without switching branches."""

    if not git_has_object(repo_path, rev):
        git_command(repo_path, "fetch", "origin", capture=False)
    resolved_rev = git_command(repo_path, "rev-parse", rev)
    tree = git_command(repo_path, "rev-parse", f"{resolved_rev}^{{tree}}")
    return RepoState(rev=resolved_rev, tree=tree)


def format_source_line(source_name: str, style: SourceLineStyle, rev: str) -> str:
    """Render one inline-table source line with repo-specific formatting."""

    if style.compact:
        fields = [f'rev = "{rev}"']
        if style.subdir is not None:
            fields.append(f'subdir = "{style.subdir}"')
        fields.append(f'url = "{style.url}"')
        return f"{source_name} = {{{', '.join(fields)}}}"

    fields = [f'url = "{style.url}"']
    if style.subdir is not None:
        fields.append(f'subdir = "{style.subdir}"')
    fields.append(f'rev = "{rev}"')
    return f"{source_name} = {{ {', '.join(fields)} }}"


def replace_section_line(path: pathlib.Path, section_name: str, key: str, new_line: str) -> None:
    """Replace one key inside one TOML section."""

    lines = path.read_text(encoding="utf-8").splitlines()
    in_section = False
    updated = False
    section_header = f"[{section_name}]"
    for index, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("[") and stripped.endswith("]"):
            in_section = stripped == section_header
            continue
        if not in_section:
            continue
        if re.match(rf"^{re.escape(key)}\s*=", line):
            lines[index] = new_line
            updated = True
            break
    if not updated:
        raise SystemExit(f"failed to update {path}: missing [{section_name}] entry for {key}")
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def replace_manifest_field(block: str, field_name: str, value: str) -> str:
    """Replace one scalar field inside one manifest dependency block."""

    updated, count = re.subn(
        rf'^{re.escape(field_name)} = "[^"]+"$',
        f'{field_name} = "{value}"',
        block,
        count=1,
        flags=re.MULTILINE,
    )
    if count != 1:
        raise SystemExit(f"failed to update manifest block field {field_name}")
    return updated


def replace_manifest_dep_block(
    path: pathlib.Path,
    dep_name: str,
    repo_state: RepoState,
) -> None:
    """Update repo-rev and git-tree-sha1 for one dependency block."""

    text = path.read_text(encoding="utf-8")
    start_pattern = re.compile(rf"(?m)^\[\[deps\.{re.escape(dep_name)}\]\]$")
    start_match = start_pattern.search(text)
    if start_match is None:
        raise SystemExit(f"failed to update {path}: missing manifest block for {dep_name}")

    next_match = re.search(r"(?m)^\[\[deps\.", text[start_match.end() :])
    end_index = start_match.end() + next_match.start() if next_match is not None else len(text)
    block = text[start_match.start() : end_index]
    block = replace_manifest_field(block, "repo-rev", repo_state.rev)
    block = replace_manifest_field(block, "git-tree-sha1", repo_state.tree)
    path.write_text(text[: start_match.start()] + block + text[end_index:], encoding="utf-8")


def load_project_sources(project_path: pathlib.Path) -> dict[str, dict[str, str]]:
    """Load the [sources] table from one Project.toml file."""

    with project_path.open("rb") as project_file:
        parsed = tomllib.load(project_file)
    sources = parsed.get("sources")
    if not isinstance(sources, dict):
        raise SystemExit(f"missing [sources] in {project_path}")
    result: dict[str, dict[str, str]] = {}
    for key, value in sources.items():
        if isinstance(value, dict):
            normalized: dict[str, str] = {}
            for subkey, subvalue in value.items():
                if isinstance(subvalue, str):
                    normalized[subkey] = subvalue
            result[str(key)] = normalized
    return result


def managed_repo_path(data_home: pathlib.Path, repo_key: str) -> pathlib.Path:
    """Return the local checkout path for one managed repo key."""

    spec = MANAGED_REPOS[repo_key]
    return data_home / spec.repo_dir


def update_project_sources_for_prod(
    data_home: pathlib.Path, prod_states: dict[str, RepoState]
) -> None:
    """Rewrite Project.toml source revs to the latest prod heads."""

    for package_dir_name, source_styles in PROJECT_SOURCE_STYLES.items():
        project_path = data_home / package_dir_name / "Project.toml"
        for source_name, style in source_styles.items():
            repo_state = prod_states[PACKAGE_TO_REPO_KEY[source_name]]
            replace_section_line(
                project_path,
                "sources",
                source_name,
                format_source_line(source_name, style, repo_state.rev),
            )


def sync_manifest_blocks_from_project(data_home: pathlib.Path) -> None:
    """Update repo-rev/git-tree-sha1 from the revs already pinned in Project.toml."""

    for package_dir_name, source_styles in PROJECT_SOURCE_STYLES.items():
        project_path = data_home / package_dir_name / "Project.toml"
        manifest_path = data_home / package_dir_name / "Manifest.toml"
        sources = load_project_sources(project_path)
        for source_name in source_styles:
            source_metadata = sources.get(source_name)
            if source_metadata is None or "rev" not in source_metadata:
                raise SystemExit(f"missing source rev for {source_name} in {project_path}")
            repo_key = PACKAGE_TO_REPO_KEY[source_name]
            repo_state = resolve_repo_state_for_rev(
                managed_repo_path(data_home, repo_key),
                source_metadata["rev"],
            )
            replace_manifest_dep_block(manifest_path, source_name, repo_state)


def collect_prod_repo_states(data_home: pathlib.Path) -> dict[str, RepoState]:
    """Move each managed repo to latest main and collect the resulting metadata."""

    states: dict[str, RepoState] = {}
    for repo_key, spec in MANAGED_REPOS.items():
        repo_path = data_home / spec.repo_dir
        states[repo_key] = sync_repo_to_remote_branch(repo_path, spec.branch)
    return states


def parse_args(argv: list[str]) -> argparse.Namespace:
    """Parse command-line arguments."""

    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--mode",
        choices=[MODE_DEV, MODE_PROD],
        default=MODE_DEV,
        help="dev keeps Project.toml revs and refreshes manifest tree hashes; "
        "prod switches managed repos to latest origin/main and rewrites project revs first.",
    )
    parser.add_argument(
        "--root",
        type=pathlib.Path,
        default=pathlib.Path(os_environ().get("PRJ_ROOT", pathlib.Path.cwd())),
        help="project root (defaults to PRJ_ROOT or current directory)",
    )
    parser.add_argument(
        "--data-home",
        type=pathlib.Path,
        default=None,
        help="path to PRJ_DATA_HOME (defaults to PRJ_DATA_HOME or ROOT/.data)",
    )
    return parser.parse_args(argv[1:])


def os_environ() -> dict[str, str]:
    """Small wrapper to make environment lookup test-friendly."""

    return dict(os.environ)


def main(argv: list[str]) -> int:
    """Run the requested sync mode."""

    args = parse_args(argv)
    root = args.root.resolve()
    data_home = (
        args.data_home.resolve()
        if args.data_home is not None
        else pathlib.Path(os_environ().get("PRJ_DATA_HOME", root / ".data")).resolve()
    )

    if args.mode == MODE_PROD:
        prod_states = collect_prod_repo_states(data_home)
        update_project_sources_for_prod(data_home, prod_states)

    sync_manifest_blocks_from_project(data_home)

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
