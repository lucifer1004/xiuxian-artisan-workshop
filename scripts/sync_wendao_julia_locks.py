#!/usr/bin/env python3
"""Synchronize Julia package source revs across Wendao package projects."""

from __future__ import annotations

import pathlib
import re
import sys


def replace_once(path: pathlib.Path, pattern: str, replacement: str) -> None:
    text = path.read_text()
    updated, count = re.subn(pattern, replacement, text, count=1, flags=re.MULTILINE)
    if count != 1:
        raise SystemExit(f"failed to update {path}: {pattern}")
    path.write_text(updated)


def replace_manifest_dep_block(
    path: pathlib.Path,
    dep_name: str,
    repo_rev: str,
    git_tree_sha1: str,
) -> None:
    text = path.read_text()
    start_pattern = re.compile(rf"(?m)^\[\[deps\.{re.escape(dep_name)}\]\]$")
    start_match = start_pattern.search(text)
    if start_match is None:
        raise SystemExit(f"failed to update {path}: missing manifest block for {dep_name}")

    next_match = re.search(r"(?m)^\[\[deps\.", text[start_match.end() :])
    end_index = start_match.end() + next_match.start() if next_match is not None else len(text)
    block = text[start_match.start() : end_index]
    updated_block = re.sub(
        r'git-tree-sha1 = "[0-9a-f]{40}"',
        f'git-tree-sha1 = "{git_tree_sha1}"',
        block,
        count=1,
    )
    updated_block = re.sub(
        r'repo-rev = "[0-9a-f]{40}"',
        f'repo-rev = "{repo_rev}"',
        updated_block,
        count=1,
    )
    path.write_text(text[: start_match.start()] + updated_block + text[end_index:])


def main(argv: list[str]) -> int:
    if len(argv) != 8:
        raise SystemExit(
            "usage: sync_wendao_julia_locks.py DATA_HOME ARROW_REV GRPCSERVER_REV "
            "WENDAOARROW_REV WENDAOCODEPARSER_REV OMPARSER_REV OMPARSER_TREE"
        )

    data_home = pathlib.Path(argv[1])
    (
        arrow_rev,
        grpcserver_rev,
        wendaoarrow_rev,
        wendaocodeparser_rev,
        omparser_rev,
        omparser_tree,
    ) = argv[2:]

    wendaoarrow_project = data_home / "WendaoArrow.jl" / "Project.toml"
    replace_once(
        wendaoarrow_project,
        r'^Arrow = \{ url = "https://github\.com/JuliaCN/arrow-julia\.git", rev = "[0-9a-f]{40}" \}$',
        f'Arrow = {{ url = "https://github.com/JuliaCN/arrow-julia.git", rev = "{arrow_rev}" }}',
    )
    replace_once(
        wendaoarrow_project,
        r'^ArrowTypes = \{ url = "https://github\.com/JuliaCN/arrow-julia\.git", subdir = "src/ArrowTypes", rev = "[0-9a-f]{40}" \}$',
        f'ArrowTypes = {{ url = "https://github.com/JuliaCN/arrow-julia.git", subdir = "src/ArrowTypes", rev = "{arrow_rev}" }}',
    )
    replace_once(
        wendaoarrow_project,
        r'^gRPCServer = \{ url = "https://github\.com/tao3k/gRPCServer\.jl\.git", rev = "[0-9a-f]{40}" \}$',
        f'gRPCServer = {{ url = "https://github.com/tao3k/gRPCServer.jl.git", rev = "{grpcserver_rev}" }}',
    )

    wendaocodeparser_project = data_home / "WendaoCodeParser.jl" / "Project.toml"
    replace_once(
        wendaocodeparser_project,
        r'^Arrow = \{rev = "[0-9a-f]{40}", url = "https://github\.com/JuliaCN/arrow-julia\.git"\}$',
        f'Arrow = {{rev = "{arrow_rev}", url = "https://github.com/JuliaCN/arrow-julia.git"}}',
    )
    replace_once(
        wendaocodeparser_project,
        r'^ArrowTypes = \{rev = "[0-9a-f]{40}", subdir = "src/ArrowTypes", url = "https://github\.com/JuliaCN/arrow-julia\.git"\}$',
        f'ArrowTypes = {{rev = "{arrow_rev}", subdir = "src/ArrowTypes", url = "https://github.com/JuliaCN/arrow-julia.git"}}',
    )
    replace_once(
        wendaocodeparser_project,
        r'^WendaoArrow = \{rev = "[0-9a-f]{40}", url = "https://github\.com/tao3k/WendaoArrow\.jl\.git"\}$',
        f'WendaoArrow = {{rev = "{wendaoarrow_rev}", url = "https://github.com/tao3k/WendaoArrow.jl.git"}}',
    )
    replace_once(
        wendaocodeparser_project,
        r'^OMParser = \{rev = "[0-9a-f]{40}", url = "https://github\.com/tao3k/OMParser\.jl"\}$',
        f'OMParser = {{rev = "{omparser_rev}", url = "https://github.com/tao3k/OMParser.jl"}}',
    )
    replace_manifest_dep_block(
        data_home / "WendaoCodeParser.jl" / "Manifest.toml",
        "OMParser",
        omparser_rev,
        omparser_tree,
    )

    wendaosearch_project = data_home / "WendaoSearch.jl" / "Project.toml"
    replace_once(
        wendaosearch_project,
        r'^Arrow = \{rev = "[0-9a-f]{40}", url = "https://github\.com/JuliaCN/arrow-julia\.git"\}$',
        f'Arrow = {{rev = "{arrow_rev}", url = "https://github.com/JuliaCN/arrow-julia.git"}}',
    )
    replace_once(
        wendaosearch_project,
        r'^ArrowTypes = \{rev = "[0-9a-f]{40}", subdir = "src/ArrowTypes", url = "https://github\.com/JuliaCN/arrow-julia\.git"\}$',
        f'ArrowTypes = {{rev = "{arrow_rev}", subdir = "src/ArrowTypes", url = "https://github.com/JuliaCN/arrow-julia.git"}}',
    )
    replace_once(
        wendaosearch_project,
        r'^WendaoArrow = \{rev = "[0-9a-f]{40}", url = "https://github\.com/tao3k/WendaoArrow\.jl\.git"\}$',
        f'WendaoArrow = {{rev = "{wendaoarrow_rev}", url = "https://github.com/tao3k/WendaoArrow.jl.git"}}',
    )
    replace_once(
        wendaosearch_project,
        r'^WendaoCodeParser = \{rev = "[0-9a-f]{40}", url = "https://github\.com/tao3k/WendaoCodeParser\.jl\.git"\}$',
        f'WendaoCodeParser = {{rev = "{wendaocodeparser_rev}", url = "https://github.com/tao3k/WendaoCodeParser.jl.git"}}',
    )
    replace_once(
        wendaosearch_project,
        r'^gRPCServer = \{rev = "[0-9a-f]{40}", url = "https://github\.com/tao3k/gRPCServer\.jl\.git"\}$',
        f'gRPCServer = {{rev = "{grpcserver_rev}", url = "https://github.com/tao3k/gRPCServer.jl.git"}}',
    )
    replace_once(
        wendaosearch_project,
        r'^OMParser = \{rev = "[0-9a-f]{40}", url = "https://github\.com/tao3k/OMParser\.jl"\}$',
        f'OMParser = {{rev = "{omparser_rev}", url = "https://github.com/tao3k/OMParser.jl"}}',
    )
    replace_manifest_dep_block(
        data_home / "WendaoSearch.jl" / "Manifest.toml",
        "OMParser",
        omparser_rev,
        omparser_tree,
    )

    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
