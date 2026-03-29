"""Tests for retained API helper surfaces in `decorators.py`."""

from __future__ import annotations

import importlib
from pathlib import Path

from xiuxian_foundation.api.decorators import (
    _generate_tool_schema,
    is_tool_result,
    normalize_tool_result,
)
from xiuxian_foundation.config.settings import Settings


def test_decorators_module_no_longer_exports_command_registration_api() -> None:
    decorators = importlib.import_module("xiuxian_foundation.api.decorators")

    assert not hasattr(decorators, "tool_command")
    assert not hasattr(decorators, "get_command_metadata")
    assert not hasattr(decorators, "get_tool_annotations")


def test_generate_tool_schema_basic_types() -> None:
    def echo(message: str, count: int = 1) -> dict:
        return {"message": message * count}

    schema = _generate_tool_schema(echo)
    assert schema["type"] == "object"
    assert "message" in schema["properties"]
    assert "count" in schema["properties"]
    assert "message" in schema.get("required", [])


def test_generate_tool_schema_excludes_injected_types() -> None:
    def func_with_injected_types(
        message: str,
        settings: Settings | None = None,
        project_root: Path | None = None,
    ) -> dict:
        return {"message": message}

    schema = _generate_tool_schema(
        func_with_injected_types,
        exclude_params={"settings", "project_root"},
    )
    assert "message" in schema["properties"]
    assert "settings" not in schema["properties"]
    assert "project_root" not in schema["properties"]


def test_decorators_module_no_longer_exports_di_helpers() -> None:
    decorators = importlib.import_module("xiuxian_foundation.api.decorators")

    assert not hasattr(decorators, "inject_resources")
    assert not hasattr(decorators, "_DIContainer")
    assert not hasattr(decorators, "_get_settings")


def test_normalize_tool_result_wraps_plain_text() -> None:
    result = normalize_tool_result("hello")
    assert is_tool_result(result)
    assert result["content"][0]["text"] == "hello"
    assert result["isError"] is False


def test_normalize_tool_result_keeps_canonical_shape() -> None:
    payload = {"content": [{"type": "text", "text": '{"status":"ok"}'}], "isError": False}
    result = normalize_tool_result(payload)
    assert result == payload
