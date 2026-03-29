"""Tests for xiuxian_foundation.api.link_graph_schema."""

from __future__ import annotations

import pytest

import xiuxian_foundation.api.link_graph_schema as link_graph_schema
from xiuxian_foundation.api.link_graph_schema import (
    build_record,
    get_schema_id,
    validate,
    validate_records,
)


def test_build_record_hit_roundtrip() -> None:
    payload = build_record(
        kind="hit",
        stem="knowledge-recall",
        title="Knowledge Recall",
        path="assets/skills/knowledge/scripts/recall.py",
        score=0.9,
        best_section="Architecture / Recall",
        match_reason="path_fuzzy+section_heading_contains",
    )
    validate(payload)
    assert payload["best_section"] == "Architecture / Recall"
    assert payload["match_reason"] == "path_fuzzy+section_heading_contains"


def test_get_schema_id() -> None:
    schema_id = get_schema_id()
    assert schema_id.endswith("/xiuxian_wendao.link_graph.record.v1.schema.json")


def test_build_record_neighbor_requires_direction() -> None:
    with pytest.raises(ValueError, match="direction"):
        validate({"schema": "xiuxian_wendao.link_graph.record.v1", "kind": "neighbor", "stem": "x"})


def test_validate_records_rejects_invalid_item() -> None:
    good = build_record(kind="metadata", stem="a", tags=["tag1"])
    bad = {"schema": "xiuxian_wendao.link_graph.record.v1", "kind": "hit", "stem": "", "score": 0.1}
    with pytest.raises(ValueError, match="stem"):
        validate_records([good, bad])


def test_get_validator_raises_when_rust_schema_backend_unavailable(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    link_graph_schema.get_validator.cache_clear()
    monkeypatch.setattr(
        link_graph_schema,
        "get_schema",
        lambda _name: (_ for _ in ()).throw(ImportError("Rust schema backend unavailable")),
    )
    with pytest.raises(ImportError, match="Rust schema backend unavailable"):
        link_graph_schema.get_validator()
    link_graph_schema.get_validator.cache_clear()
