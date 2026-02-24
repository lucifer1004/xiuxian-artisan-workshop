"""Shared helpers for strict machine JSON output in CLI paths."""

from __future__ import annotations

from typing import Any

from omni.foundation.utils import json_codec as json


def unwrap_canonical_json_payload(value: Any) -> Any:
    """Unwrap canonical MCP payloads for CLI --json output.

    Canonical shape:
      {"content":[{"type":"text","text":"<json-or-text>"}],"isError":false}
    """
    if isinstance(value, str):
        stripped = value.strip()
        if not stripped:
            return value
        try:
            parsed = json.loads(stripped)
        except json.JSONDecodeError:
            return value
        return unwrap_canonical_json_payload(parsed)

    if not isinstance(value, dict):
        return value

    content = value.get("content")
    if (
        isinstance(content, list)
        and content
        and isinstance(content[0], dict)
        and isinstance(content[0].get("text"), str)
        and "isError" in value
    ):
        text = content[0]["text"]
        stripped = text.strip()
        if not stripped:
            return ""
        try:
            return json.loads(stripped)
        except json.JSONDecodeError:
            return text

    return value


def normalize_result_for_json_output(result: Any) -> str:
    """Normalize tool result into strict JSON-mode stdout payload."""
    if hasattr(result, "model_dump_json"):
        try:
            model_json = result.model_dump_json(indent=2)
        except TypeError:
            model_json = result.model_dump_json()
        if isinstance(model_json, bytes):
            model_json = model_json.decode("utf-8", errors="replace")
        normalized = unwrap_canonical_json_payload(model_json)
    elif hasattr(result, "model_dump"):
        try:
            model_payload = result.model_dump(mode="json")
        except TypeError:
            model_payload = result.model_dump()
        normalized = unwrap_canonical_json_payload(model_payload)
    elif isinstance(result, (dict, list, str)):
        normalized = unwrap_canonical_json_payload(result)
    elif hasattr(result, "data"):
        data = getattr(result, "data", None)
        normalized = unwrap_canonical_json_payload(data)
        if data is None:
            normalized = ""
    else:
        normalized = str(result)

    if isinstance(normalized, (dict, list)):
        return json.dumps(normalized, indent=2, ensure_ascii=False)
    if normalized is None:
        return ""
    if isinstance(normalized, str):
        return normalized
    return str(normalized)


__all__ = ["normalize_result_for_json_output", "unwrap_canonical_json_payload"]
