"""Architecture guardrails for LinkGraph search API contracts."""

from __future__ import annotations

import json
import re
from typing import TYPE_CHECKING

import pytest

pytestmark = pytest.mark.architecture

if TYPE_CHECKING:
    from pathlib import Path


def _read(project_root: Path, relative_path: str) -> str:
    return (project_root / relative_path).read_text(encoding="utf-8")


def _read_any(project_root: Path, relative_paths: tuple[str, ...]) -> str:
    for relative_path in relative_paths:
        path = project_root / relative_path
        if path.exists():
            return path.read_text(encoding="utf-8")
    joined = ", ".join(relative_paths)
    raise AssertionError(f"none of expected source files exist: {joined}")


def _read_tree(project_root: Path, relative_dir: str) -> str:
    root = project_root / relative_dir
    if not root.exists():
        raise AssertionError(f"expected source directory not found: {relative_dir}")
    parts: list[str] = []
    for path in sorted(root.rglob("*.rs")):
        parts.append(path.read_text(encoding="utf-8"))
    return "\n".join(parts)


def test_link_graph_python_backend_host_surface_is_deleted(project_root: Path) -> None:
    """Python must not carry a local link-graph backend host surface."""
    path = project_root / "packages/python/foundation/src/xiuxian_rag/link_graph/backend.py"
    assert path.exists() is False


def test_link_graph_rust_py_binding_surface_is_deleted(project_root: Path) -> None:
    """Rust must not carry the deleted Python binding facade for link-graph search."""
    paths = (
        project_root / "packages/rust/crates/xiuxian-wendao/src/link_graph_py.rs",
        project_root / "packages/rust/crates/xiuxian-wendao/src/link_graph_py/engine/mod.rs",
    )
    assert all(path.exists() is False for path in paths)


def test_link_graph_rust_index_contract_is_planned_only(project_root: Path) -> None:
    """Rust index public API must stay under the modularized index tree only."""
    flat_index = project_root / "packages/rust/crates/xiuxian-wendao/src/link_graph/index.rs"
    assert flat_index.exists() is False
    source = _read_tree(project_root, "packages/rust/crates/xiuxian-wendao/src/link_graph/index")
    assert "pub fn search_planned(" in source
    assert re.search(r"(?m)^\s*pub\s+fn\s+search\s*\(", source) is None
    assert re.search(r"(?m)^\s*pub\s+fn\s+search_with_query\s*\(", source) is None
    assert re.search(r"(?m)^\s*pub\s+fn\s+search_with_options\s*\(", source) is None


def test_link_graph_reason_vocab_contract_matches_schema(project_root: Path) -> None:
    """Rust reason constants and schema enum must stay aligned."""
    rust_source = _read(
        project_root,
        "packages/rust/crates/xiuxian-wendao/src/link_graph/models/records/retrieval_plan.rs",
    )
    schema = json.loads(
        _read(
            project_root,
            "packages/rust/crates/xiuxian-wendao/resources/xiuxian_wendao.link_graph.retrieval_plan.v1.schema.json",
        )
    )
    rust_reasons = sorted(
        set(re.findall(r'LINK_GRAPH_REASON_[A-Z_]+:\s*&str\s*=\s*"([^"]+)"', rust_source))
    )
    schema_reasons = sorted(schema["properties"]["reason"]["enum"])
    assert rust_reasons == schema_reasons
