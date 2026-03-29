"""
decorators.py - Retained Python helper exports.

This module keeps:
- tool result normalization
- execution helpers
- schema generation helpers
- resource registration
"""

from __future__ import annotations

import json
from collections.abc import Callable
from typing import Any

from .execution import (
    TimingContext,
    cached,
    measure_time,
    retry,
    trace_execution,
)
from .schema import _generate_tool_schema
from .types import CommandResult

# Re-export for convenience
__all__ = [
    # Tool result contract
    "normalize_tool_result",
    "is_tool_result",
    "TOOL_RESULT_SCHEMA_V1",
    "TOOL_RESULT_CONTENT_KEY",
    "TOOL_RESULT_IS_ERROR_KEY",
    # Execution
    "trace_execution",
    "measure_time",
    "retry",
    "cached",
    "TimingContext",
    # Schema
    "_generate_tool_schema",
    "CommandResult",
]


# =============================================================================
# Tool result normalization
# =============================================================================

TOOL_RESULT_CONTENT_KEY = "content"
TOOL_RESULT_IS_ERROR_KEY = "isError"
TOOL_RESULT_SCHEMA_V1 = "xiuxian.runtime.tool_result.v1"


def _build_tool_result(text: str, is_error: bool = False) -> dict[str, Any]:
    return {
        TOOL_RESULT_CONTENT_KEY: [{"type": "text", "text": text}],
        TOOL_RESULT_IS_ERROR_KEY: bool(is_error),
    }


def is_tool_result(value: Any) -> bool:
    if not isinstance(value, dict):
        return False
    content = value.get(TOOL_RESULT_CONTENT_KEY)
    if not isinstance(content, list) or not content:
        return False
    first = content[0]
    return (
        isinstance(first, dict)
        and first.get("type") == "text"
        and isinstance(first.get("text"), str)
        and TOOL_RESULT_IS_ERROR_KEY in value
    )


def _enforce_tool_result_shape(payload: dict[str, Any]) -> dict[str, Any]:
    return {
        TOOL_RESULT_CONTENT_KEY: payload[TOOL_RESULT_CONTENT_KEY],
        TOOL_RESULT_IS_ERROR_KEY: bool(payload[TOOL_RESULT_IS_ERROR_KEY]),
    }


def _parse_tool_result_payload(value: Any) -> dict[str, Any]:
    if isinstance(value, dict):
        data = value
    elif isinstance(value, str):
        data = json.loads(value)
        if not isinstance(data, dict):
            raise TypeError(f"Expected JSON object payload, got: {type(data).__name__}")
    else:
        raise TypeError(f"Unsupported payload type: {type(value).__name__}")

    if is_tool_result(data):
        content = data.get(TOOL_RESULT_CONTENT_KEY) or []
        first = content[0] if content else {}
        text = first.get("text") if isinstance(first, dict) else None
        if isinstance(text, str) and text.strip():
            try:
                parsed = json.loads(text)
                if isinstance(parsed, dict):
                    return parsed
            except json.JSONDecodeError:
                return data
    return data


def _validate_tool_result(payload: dict[str, Any]) -> None:
    if not is_tool_result(payload):
        raise ValueError("Tool result must contain canonical content/isError fields")


def _text_from_raw(value: Any) -> str:
    """Serialize a raw return value to display text."""
    if value is None:
        return ""
    if isinstance(value, str):
        return value
    return json.dumps(value, ensure_ascii=False)


def normalize_tool_result(return_value: Any) -> dict[str, Any]:
    """Normalize any tool return value to the retained canonical result shape.

    - Canonical dicts are stripped to content + isError via enforce_result_shape.
    - All other values are wrapped with build_result(text).
    - Final result is always validated against the local content/isError contract.
    """
    if hasattr(return_value, "success") and hasattr(return_value, "data"):
        if return_value.success and return_value.data is not None:
            return normalize_tool_result(return_value.data)
        result = _build_tool_result(
            getattr(return_value, "error", None) or str(return_value),
            is_error=True,
        )
    elif is_tool_result(return_value):
        result = _enforce_tool_result_shape(return_value)
    else:
        result = _build_tool_result(_text_from_raw(return_value), is_error=False)

    _validate_tool_result(result)
    return result


# =============================================================================
# Prompt Decorator (Prompt Template Registration)
# =============================================================================


def prompt(
    name: str | None = None,
    description: str | None = None,
):
    """Decorator to mark a function as a prompt template.

    The function receives prompt arguments (e.g. from runtime prompt lookup) and
    returns the prompt content (string or list of messages).

    Example::

        @prompt(
            name="analyze_code",
            description="Code analysis template",
        )
        def analyze_code(file_path: str) -> str:
            return f'''
        请分析 {file_path}:
        1. 代码结构
        2. 潜在问题
        '''

    Args:
        name: Prompt name (defaults to function name).
        description: Human-readable description for list_prompts.
    """

    def decorator(func: Callable) -> Callable:
        prompt_name = name or func.__name__
        prompt_desc = description or (func.__doc__ or "").strip().split("\n")[0]

        func._is_prompt = True  # type: ignore[attr-defined]
        func._prompt_config = {  # type: ignore[attr-defined]
            "name": prompt_name,
            "description": prompt_desc,
        }
        return func

    if callable(name):
        func = name
        name = None
        return decorator(func)

    return decorator


def is_prompt(func: Callable) -> bool:
    """Check if a function is marked with @prompt."""
    return getattr(func, "_is_prompt", False)


def get_prompt_config(func: Callable) -> dict | None:
    """Get the prompt config attached to a function (for @prompt)."""
    return getattr(func, "_prompt_config", None)
