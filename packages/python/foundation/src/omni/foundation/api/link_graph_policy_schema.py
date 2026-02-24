"""
LinkGraph retrieval plan schema API for policy contract validation.

This module freezes the policy payload crossing common retrieval layers.
"""

from __future__ import annotations

from functools import lru_cache
from typing import Any, Literal

from jsonschema import Draft202012Validator

from .schema_provider import get_schema

SCHEMA_NAME = "omni.link_graph.retrieval_plan.v1.schema.json"
SCHEMA_VERSION = "omni.link_graph.retrieval_plan.v1"
RetrievalMode = Literal["graph_only", "hybrid", "vector_only"]
ConfidenceLevel = Literal["none", "low", "medium", "high"]


@lru_cache(maxsize=1)
def get_validator() -> Draft202012Validator:
    """Cached validator for LinkGraph retrieval plan schema."""
    return Draft202012Validator(get_schema(SCHEMA_VERSION))


@lru_cache(maxsize=1)
def get_schema_id() -> str:
    """Return JSON schema `$id` from LinkGraph retrieval plan schema."""
    schema = get_schema(SCHEMA_VERSION)
    schema_id = str(schema.get("$id", "")).strip()
    if not schema_id:
        raise ValueError("LinkGraph retrieval plan schema missing $id")
    return schema_id


@lru_cache(maxsize=1)
def get_reason_enum() -> tuple[str, ...]:
    """Return canonical policy-reason enum from retrieval plan schema."""
    schema = get_schema(SCHEMA_VERSION)
    reason = schema.get("properties", {}).get("reason", {})
    values = reason.get("enum") if isinstance(reason, dict) else None
    if not isinstance(values, list) or not values:
        raise ValueError("LinkGraph retrieval plan schema missing reason enum")
    out = tuple(str(item).strip() for item in values if str(item).strip())
    if not out:
        raise ValueError("LinkGraph retrieval plan schema has empty reason enum")
    return out


def validate(payload: dict[str, Any]) -> None:
    """Raise ValueError if retrieval plan payload violates schema."""
    errs = sorted(get_validator().iter_errors(payload), key=lambda e: list(e.path))
    if not errs:
        return
    first = errs[0]
    loc = ".".join(str(p) for p in first.path) or "<root>"
    raise ValueError(f"link_graph retrieval plan schema violation at {loc}: {first.message}")


def build_plan_record(
    *,
    requested_mode: RetrievalMode,
    selected_mode: RetrievalMode,
    reason: str,
    backend_name: str,
    graph_hit_count: int,
    source_hint_count: int,
    graph_confidence_score: float,
    graph_confidence_level: ConfidenceLevel,
    budget_candidate_limit: int,
    budget_max_sources: int,
    budget_rows_per_source: int,
) -> dict[str, Any]:
    """Build and validate canonical LinkGraph retrieval plan payload."""
    payload = {
        "schema": SCHEMA_VERSION,
        "requested_mode": str(requested_mode),
        "selected_mode": str(selected_mode),
        "reason": str(reason),
        "backend_name": str(backend_name),
        "graph_hit_count": int(graph_hit_count),
        "source_hint_count": int(source_hint_count),
        "graph_confidence_score": float(graph_confidence_score),
        "graph_confidence_level": str(graph_confidence_level),
        "budget": {
            "candidate_limit": int(budget_candidate_limit),
            "max_sources": int(budget_max_sources),
            "rows_per_source": int(budget_rows_per_source),
        },
    }
    validate(payload)
    return payload


__all__ = [
    "SCHEMA_NAME",
    "SCHEMA_VERSION",
    "ConfidenceLevel",
    "RetrievalMode",
    "build_plan_record",
    "get_reason_enum",
    "get_schema_id",
    "get_validator",
    "validate",
]
