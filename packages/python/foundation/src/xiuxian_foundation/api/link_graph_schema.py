"""
Link-graph record schema API.

This module is the single validation entrypoint for link-graph payloads crossing
the Python/Rust boundary.
"""

from __future__ import annotations

from functools import lru_cache
from typing import Any, Literal

from jsonschema import Draft202012Validator

from .schema_provider import get_schema

SCHEMA_NAME = "xiuxian_wendao.link_graph.record.v1.schema.json"
SCHEMA_VERSION = "xiuxian_wendao.link_graph.record.v1"
RecordKind = Literal["hit", "neighbor", "metadata"]
Direction = Literal["incoming", "outgoing", "both"]


@lru_cache(maxsize=1)
def get_validator() -> Draft202012Validator:
    """Cached JSON Schema validator for link-graph records."""
    return Draft202012Validator(get_schema(SCHEMA_VERSION))


@lru_cache(maxsize=1)
def get_schema_id() -> str:
    """Return JSON schema `$id` from shared link-graph schema."""
    schema = get_schema(SCHEMA_VERSION)
    schema_id = str(schema.get("$id", "")).strip()
    if not schema_id:
        raise ValueError("Link graph schema missing $id")
    return schema_id


def validate(record: dict[str, Any]) -> None:
    """Raise ValueError if a record violates the shared schema."""
    errs = sorted(get_validator().iter_errors(record), key=lambda e: list(e.path))
    if not errs:
        return
    first = errs[0]
    loc = ".".join(str(p) for p in first.path) or "<root>"
    raise ValueError(f"link_graph schema violation at {loc}: {first.message}")


def build_record(
    *,
    kind: RecordKind,
    stem: str,
    title: str = "",
    path: str = "",
    score: float | None = None,
    best_section: str | None = None,
    match_reason: str | None = None,
    direction: Direction | None = None,
    distance: int | None = None,
    tags: list[str] | None = None,
) -> dict[str, Any]:
    """Build and validate a canonical link-graph record."""
    payload: dict[str, Any] = {
        "schema": SCHEMA_VERSION,
        "kind": kind,
        "stem": stem,
        "title": title,
        "path": path,
    }
    if score is not None or kind == "hit":
        payload["score"] = score
    if best_section is not None:
        payload["best_section"] = str(best_section)
    if match_reason is not None:
        payload["match_reason"] = str(match_reason)
    if direction is not None or kind == "neighbor":
        payload["direction"] = direction
    if distance is not None:
        payload["distance"] = distance
    if tags is not None or kind == "metadata":
        payload["tags"] = tags or []
    validate(payload)
    return payload


def validate_records(records: list[dict[str, Any]]) -> None:
    """Validate every record in a list."""
    for record in records:
        validate(record)


__all__ = [
    "SCHEMA_NAME",
    "SCHEMA_VERSION",
    "Direction",
    "RecordKind",
    "build_record",
    "get_schema_id",
    "get_validator",
    "validate",
    "validate_records",
]
