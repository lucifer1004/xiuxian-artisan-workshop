"""
LinkGraph Valkey cache snapshot schema API.

This module is the single validation entrypoint for the shared contract.
"""

from __future__ import annotations

from functools import lru_cache
from typing import Any

from jsonschema import Draft202012Validator

from .schema_provider import get_schema

SCHEMA_NAME = "omni.link_graph.valkey_cache_snapshot.v1.schema.json"
SCHEMA_VERSION = "omni.link_graph.valkey_cache_snapshot.v1"


@lru_cache(maxsize=1)
def get_validator() -> Draft202012Validator:
    """Cached validator for LinkGraph Valkey cache snapshot schema."""
    return Draft202012Validator(get_schema(SCHEMA_VERSION))


@lru_cache(maxsize=1)
def get_schema_id() -> str:
    """Return JSON schema `$id` from LinkGraph Valkey cache schema."""
    schema = get_schema(SCHEMA_VERSION)
    schema_id = str(schema.get("$id", "")).strip()
    if not schema_id:
        raise ValueError("LinkGraph Valkey cache schema missing $id")
    return schema_id


def validate(payload: dict[str, Any]) -> None:
    """Raise ValueError if payload violates LinkGraph Valkey cache schema."""
    errs = sorted(get_validator().iter_errors(payload), key=lambda e: list(e.path))
    if not errs:
        return
    first = errs[0]
    loc = ".".join(str(p) for p in first.path) or "<root>"
    raise ValueError(f"link_graph valkey cache schema violation at {loc}: {first.message}")


__all__ = [
    "SCHEMA_NAME",
    "SCHEMA_VERSION",
    "get_schema_id",
    "get_validator",
    "validate",
]
