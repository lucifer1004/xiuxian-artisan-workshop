"""Common link-graph engine contract and adapters."""

from __future__ import annotations

from importlib import import_module
from typing import Any

_EXPORTS: dict[str, tuple[str, str]] = {
    "LinkGraphConfidenceLevel": (".policy", "LinkGraphConfidenceLevel"),
    "LinkGraphDirection": (".models", "LinkGraphDirection"),
    "LinkGraphHit": (".models", "LinkGraphHit"),
    "LinkGraphMatchStrategy": (".models", "LinkGraphMatchStrategy"),
    "LinkGraphMetadata": (".models", "LinkGraphMetadata"),
    "LinkGraphNeighbor": (".models", "LinkGraphNeighbor"),
    "LinkGraphPolicyConfig": (".policy", "LinkGraphPolicyConfig"),
    "LinkGraphRecallPolicyDecision": (".recall_policy", "LinkGraphRecallPolicyDecision"),
    "LinkGraphRetrievalBudget": (".policy", "LinkGraphRetrievalBudget"),
    "LinkGraphRetrievalMode": (".policy", "LinkGraphRetrievalMode"),
    "LinkGraphRetrievalPlan": (".policy", "LinkGraphRetrievalPlan"),
    "LinkGraphSearchOptions": (".models", "LinkGraphSearchOptions"),
    "LinkGraphSourceHint": (".policy", "LinkGraphSourceHint"),
    "apply_link_graph_proximity_boost": (".proximity", "apply_link_graph_proximity_boost"),
    "clear_link_graph_stats_cache": (".stats_cache", "clear_link_graph_stats_cache"),
    "evaluate_link_graph_recall_policy": (".recall_policy", "evaluate_link_graph_recall_policy"),
    "fetch_graph_rows_by_policy": (".policy", "fetch_graph_rows_by_policy"),
    "get_cached_link_graph_stats": (".stats_cache", "get_cached_link_graph_stats"),
    "get_link_graph_retrieval_plan_schema_id": (
        ".policy",
        "get_link_graph_retrieval_plan_schema_id",
    ),
    "get_link_graph_stats_for_response": (".stats_cache", "get_link_graph_stats_for_response"),
    "link_graph_hits_to_hybrid_results": (".search_results", "link_graph_hits_to_hybrid_results"),
    "link_graph_hits_to_search_results": (".search_results", "link_graph_hits_to_search_results"),
    "merge_hybrid_results": (".search_results", "merge_hybrid_results"),
    "neighbors_to_link_rows": (".search_results", "neighbors_to_link_rows"),
    "normalize_link_graph_direction": (".search_results", "normalize_link_graph_direction"),
    "note_recent_graph_search_timeout": (".policy", "note_recent_graph_search_timeout"),
    "plan_link_graph_retrieval": (".policy", "plan_link_graph_retrieval"),
    "resolve_link_graph_policy_config": (".policy", "resolve_link_graph_policy_config"),
    "schedule_link_graph_stats_refresh": (".stats_cache", "schedule_link_graph_stats_refresh"),
    "serialize_link_graph_retrieval_plan": (".policy", "serialize_link_graph_retrieval_plan"),
    "take_recent_graph_search_timeout": (".policy", "take_recent_graph_search_timeout"),
    "vector_rows_to_hybrid_results": (".search_results", "vector_rows_to_hybrid_results"),
}

__all__ = sorted(_EXPORTS)


def __getattr__(name: str) -> Any:
    target = _EXPORTS.get(name)
    if target is None:
        raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
    module_name, attr_name = target
    module = import_module(module_name, package=__name__)
    value = getattr(module, attr_name)
    globals()[name] = value
    return value


def __dir__() -> list[str]:
    return sorted(set(globals()) | set(__all__))
