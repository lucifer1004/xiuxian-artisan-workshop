"""Retrieval response payload helpers for Rust-owned recall flows."""

from __future__ import annotations

from typing import Any


def build_status_message_response(
    *,
    status: str,
    message: str,
    extra: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Build a normalized status payload with an optional metadata overlay."""
    payload: dict[str, Any] = {"status": status, "message": message}
    if extra:
        payload.update(extra)
    return payload


def build_status_error_response(
    *,
    error: str,
    extra: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Build a normalized error payload with an optional metadata overlay."""
    payload: dict[str, Any] = {"status": "error", "error": error}
    if extra:
        payload.update(extra)
    return payload


def build_recall_error_response(*, query: str, error: str) -> dict[str, Any]:
    """Build an error payload for recall requests."""
    return {
        "query": query,
        "status": "error",
        "error": error,
        "results": [],
    }


def build_recall_chunked_response(
    *,
    query: str,
    status: str,
    error: str | None,
    preview_results: list[dict[str, Any]] | None,
    batches: list[list[dict[str, Any]]] | None,
    results: list[dict[str, Any]] | None,
) -> dict[str, Any]:
    """Build a chunked recall payload."""
    normalized_results = list(results or [])
    return {
        "query": query,
        "status": status,
        "error": error,
        "preview_results": list(preview_results or []),
        "batches": list(batches or []),
        "all_chunks_count": len(normalized_results),
        "results": normalized_results,
    }


def extract_graph_confidence(metadata: dict[str, Any] | None) -> tuple[float, str]:
    """Extract graph confidence score/level from policy metadata."""
    if not metadata:
        return 0.0, "none"

    score = metadata.get("graph_confidence_score", 0.0)
    level = str(metadata.get("graph_confidence_level", "none") or "none")
    try:
        numeric_score = float(score)
    except (TypeError, ValueError):
        numeric_score = 0.0
    return numeric_score, level


def override_retrieval_plan_mode(
    retrieval_plan: dict[str, Any] | None,
    *,
    selected_mode: str,
    reason: str,
) -> dict[str, Any] | None:
    """Copy and override the selected mode for an existing retrieval plan."""
    if retrieval_plan is None:
        return None

    updated = dict(retrieval_plan)
    updated["selected_mode"] = selected_mode
    updated["reason"] = reason
    return updated


def build_recall_search_response(
    *,
    query: str,
    keywords: list[str] | None,
    collection: str,
    preview: bool,
    retrieval_mode: str,
    retrieval_path: str,
    retrieval_reason: str,
    graph_backend: str,
    graph_hit_count: int,
    graph_confidence_score: float,
    graph_confidence_level: str,
    retrieval_plan_schema_id: str,
    retrieval_plan: dict[str, Any] | None,
    results: list[dict[str, Any]] | None,
) -> dict[str, Any]:
    """Build the normalized recall search payload."""
    normalized_results = list(results or [])
    normalized_schema_id = retrieval_plan_schema_id or None

    return {
        "status": "success",
        "query": query,
        "keywords": list(keywords or []),
        "collection": collection,
        "preview": preview,
        "retrieval_mode": retrieval_mode,
        "retrieval_path": retrieval_path,
        "retrieval_reason": retrieval_reason,
        "graph_backend": graph_backend,
        "graph_hit_count": graph_hit_count,
        "graph_confidence_score": graph_confidence_score,
        "graph_confidence_level": graph_confidence_level,
        "retrieval_plan_schema_id": normalized_schema_id,
        "retrieval_plan": retrieval_plan,
        "found": len(normalized_results),
        "results": normalized_results,
    }


__all__ = [
    "build_recall_chunked_response",
    "build_recall_error_response",
    "build_recall_search_response",
    "build_status_error_response",
    "build_status_message_response",
    "extract_graph_confidence",
    "override_retrieval_plan_mode",
]
