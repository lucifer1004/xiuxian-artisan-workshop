"""Tests for schema provider binding-only loading."""

from __future__ import annotations

import json
import sys
import types

import pytest

import omni.foundation.api.schema_provider as schema_provider


@pytest.fixture(autouse=True)
def _clear_schema_cache() -> None:
    schema_provider.get_schema.cache_clear()
    yield
    schema_provider.get_schema.cache_clear()


def test_get_schema_loads_from_xiuxian_binding(monkeypatch):
    """Schema id should resolve from _xiuxian_wendao binding when available."""
    module = types.ModuleType("_xiuxian_wendao")
    module.get_schema = lambda name: json.dumps(
        {"$id": f"https://schemas.test/{name}.json", "type": "object"}
    )
    monkeypatch.setitem(sys.modules, "_xiuxian_wendao", module)

    # Use an ID that does not exist in bundled Rust resources so binding path is exercised.
    schema = schema_provider.get_schema("custom.schema.v1")

    assert schema["type"] == "object"
    assert schema["$id"].endswith("custom.schema.v1.json")


def test_get_schema_loads_mapped_schema_from_omni_core_rs(monkeypatch):
    """Mapped schema ids should load from omni_core_rs type registry."""
    module = types.ModuleType("omni_core_rs")
    module.py_get_schema_json = lambda type_name: json.dumps(
        {"$id": f"https://schemas.test/{type_name}.json", "type": "object"}
    )
    monkeypatch.setitem(sys.modules, "_xiuxian_wendao", types.ModuleType("_xiuxian_wendao"))
    monkeypatch.setitem(sys.modules, "omni_core_rs", module)

    vector_schema = schema_provider.get_schema("omni.vector.search.v1")
    hybrid_schema = schema_provider.get_schema("omni.vector.hybrid.v1")
    tool_schema = schema_provider.get_schema("omni.vector.tool_search.v1")

    assert vector_schema["type"] == "object"
    assert vector_schema["$id"].endswith("VectorSearchResult.json")
    assert hybrid_schema["$id"].endswith("HybridSearchResult.json")
    assert tool_schema["$id"].endswith("ToolSearchResult.json")


def test_get_schema_loads_named_schema_from_omni_core_rs(monkeypatch):
    """Canonical schema ids should resolve via omni_core_rs named-schema API."""
    module = types.ModuleType("omni_core_rs")
    module.py_get_named_schema_json = lambda schema_id: json.dumps(
        {"$id": f"https://schemas.test/{schema_id}.json", "type": "object"}
    )
    monkeypatch.setitem(sys.modules, "_xiuxian_wendao", types.ModuleType("_xiuxian_wendao"))
    monkeypatch.setitem(sys.modules, "omni_core_rs", module)

    schema = schema_provider.get_schema("omni.agent.server_info.v1")
    assert schema["type"] == "object"
    assert schema["$id"].endswith("omni.agent.server_info.v1.json")


def test_get_schema_falls_back_when_xiuxian_unknown(monkeypatch):
    """When xiuxian binding reports unknown schema, provider should fall back to omni_core_rs."""
    xiuxian = types.ModuleType("_xiuxian_wendao")
    xiuxian.get_schema = lambda _name: (_ for _ in ()).throw(ValueError("unknown"))
    core = types.ModuleType("omni_core_rs")
    core.py_get_named_schema_json = lambda schema_id: json.dumps(
        {"$id": f"https://schemas.test/{schema_id}.json", "type": "object"}
    )
    monkeypatch.setitem(sys.modules, "_xiuxian_wendao", xiuxian)
    monkeypatch.setitem(sys.modules, "omni_core_rs", core)

    schema = schema_provider.get_schema("omni.agent.server_info.v1")
    assert schema["type"] == "object"
    assert schema["$id"].endswith("omni.agent.server_info.v1.json")


def test_get_schema_raises_for_unmapped_schema_without_xiuxian(monkeypatch):
    """Unmapped schema ids should fail when xiuxian binding is unavailable."""
    monkeypatch.setitem(sys.modules, "_xiuxian_wendao", types.ModuleType("_xiuxian_wendao"))
    monkeypatch.setitem(sys.modules, "omni_core_rs", types.ModuleType("omni_core_rs"))

    with pytest.raises(ImportError):
        schema_provider.get_schema("omni.vector.unknown.v1")
