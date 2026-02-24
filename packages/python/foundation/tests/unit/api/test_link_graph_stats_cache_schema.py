"""Tests for omni.foundation.api.link_graph_stats_cache_schema."""

from __future__ import annotations

import pytest

import omni.foundation.api.link_graph_stats_cache_schema as stats_schema
from omni.foundation.api.link_graph_stats_cache_schema import get_schema_id, validate


def _payload() -> dict:
    return {
        "schema": "omni.link_graph.stats.cache.v1",
        "source_key": "/tmp/notebook|include=docs|exclude=.git",
        "updated_at_unix": 1739980800.0,
        "stats": {
            "total_notes": 42,
            "orphans": 9,
            "links_in_graph": 1337,
            "nodes_in_graph": 42,
        },
    }


def test_get_schema_id() -> None:
    schema_id = get_schema_id()
    assert schema_id.endswith("/omni.link_graph.stats.cache.v1.schema.json")


def test_validate_accepts_payload() -> None:
    validate(_payload())


def test_validate_rejects_invalid_schema_version() -> None:
    payload = _payload()
    payload["schema"] = "omni.link_graph.stats.cache.v0"
    with pytest.raises(ValueError, match="schema"):
        validate(payload)


def test_validate_rejects_negative_stats() -> None:
    payload = _payload()
    payload["stats"]["total_notes"] = -1
    with pytest.raises(ValueError, match=r"stats\.total_notes"):
        validate(payload)


def test_get_validator_raises_when_rust_schema_backend_unavailable(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    stats_schema.get_validator.cache_clear()
    monkeypatch.setattr(
        stats_schema,
        "get_schema",
        lambda _name: (_ for _ in ()).throw(ImportError("Rust schema backend unavailable")),
    )
    with pytest.raises(ImportError, match="Rust schema backend unavailable"):
        stats_schema.get_validator()
    stats_schema.get_validator.cache_clear()
