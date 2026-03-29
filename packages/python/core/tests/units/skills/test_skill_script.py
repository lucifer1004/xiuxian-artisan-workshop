"""Removal guards for deleted Python command-decorator surfaces."""

from __future__ import annotations

import importlib

import pytest

from xiuxian_foundation.api.types import CommandResult


def test_decorators_module_no_longer_exports_command_decorator() -> None:
    decorators = importlib.import_module("xiuxian_foundation.api.decorators")

    assert not hasattr(decorators, "tool_command")
    assert not hasattr(decorators, "get_command_metadata")
    assert not hasattr(decorators, "get_tool_annotations")


class TestCommandResultGeneric:
    """Tests for the retained CommandResult Generic[T] type."""

    def test_command_result_typed_dict(self) -> None:
        result = CommandResult(success=True, data={"key": "value"})
        assert isinstance(result.data, dict)
        assert result.data["key"] == "value"

    def test_command_result_typed_str(self) -> None:
        result = CommandResult(success=True, data="hello")
        assert isinstance(result.data, str)
        assert result.data == "hello"

    def test_command_result_computed_fields(self) -> None:
        result = CommandResult(
            success=False,
            data={},
            error="connection refused",
            metadata={"retry_count": 2, "duration_ms": 150.0},
        )

        assert result.is_retryable is True
        assert result.retry_count == 2
        assert result.duration_ms == 150.0

        serialized = result.model_dump()
        assert "is_retryable" in serialized
        assert "retry_count" in serialized
        assert "duration_ms" in serialized

    def test_command_result_frozen(self) -> None:
        result = CommandResult(success=True, data="test")
        with pytest.raises(Exception):
            result.success = False
