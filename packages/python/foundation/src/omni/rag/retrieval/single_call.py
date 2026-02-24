"""Common single-call recall execution with graph policy and resilient fallback."""

from __future__ import annotations

from collections.abc import Awaitable, Callable
from typing import Any

from omni.foundation.runtime.skill_optimization import is_low_signal_query

from .executor import run_recall_query_rows
from .response import build_recall_search_response

type QueryRowsRunnerFn = Callable[..., Awaitable[list[dict[str, Any]]]]
type RecallPostprocessRowsFn = Callable[
    [list[dict[str, Any]], str, int, float, bool, int, bool],
    Awaitable[list[dict[str, Any]]],
]
type RecallLogFn = Callable[[str, Any], None]


async def _evaluate_recall_policy(
    *,
    query: str,
    limit: int,
    retrieval_mode: str,
    store: Any,
    collection: str,
) -> Any:
    from omni.rag.link_graph import evaluate_link_graph_recall_policy

    return await evaluate_link_graph_recall_policy(
        query=query,
        limit=limit,
        retrieval_mode=retrieval_mode,
        store=store,
        collection=collection,
    )


def _no_op_log(_message: str, _arg: Any) -> None:
    return None


def _build_response(
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
    results: list[dict[str, Any]],
) -> dict[str, Any]:
    return build_recall_search_response(
        query=query,
        keywords=keywords,
        collection=collection,
        preview=preview,
        retrieval_mode=retrieval_mode,
        retrieval_path=retrieval_path,
        retrieval_reason=retrieval_reason,
        graph_backend=graph_backend,
        graph_hit_count=graph_hit_count,
        graph_confidence_score=graph_confidence_score,
        graph_confidence_level=graph_confidence_level,
        retrieval_plan_schema_id=retrieval_plan_schema_id,
        retrieval_plan=retrieval_plan,
        results=results,
    )


async def run_recall_single_call(
    *,
    vector_store: Any,
    query: str,
    keywords: list[str] | None,
    collection: str,
    limit: int,
    fetch_limit: int,
    min_score: float,
    preview: bool,
    snippet_chars: int,
    retrieval_mode: str,
    postprocess_rows: RecallPostprocessRowsFn,
    query_rows_runner: QueryRowsRunnerFn = run_recall_query_rows,
    debug_log: RecallLogFn | None = None,
    warning_log: RecallLogFn | None = None,
    allow_graph_policy: bool = True,
    allow_graph_fallback_on_vector_error: bool = True,
) -> dict[str, Any]:
    """Execute one-shot recall with graph policy and vector fallback resilience."""
    debug = debug_log or _no_op_log
    warning = warning_log or _no_op_log

    retrieval_path = "vector_only"
    retrieval_reason = "vector_default"
    graph_backend = ""
    graph_hit_count = 0
    graph_confidence_score = 0.0
    graph_confidence_level = "none"
    retrieval_plan_record: dict[str, Any] | None = None
    retrieval_plan_schema_id = ""
    enable_fusion_boost = retrieval_mode != "vector_only"
    active_store = vector_store.get_store_for_collection(collection)
    keywords = list(keywords or [])

    if keywords and retrieval_mode == "graph_only":
        retrieval_reason = "keywords_force_vector"

    can_use_graph = (
        not keywords
        and not is_low_signal_query(query, min_non_space_chars=2)
        and bool((query or "").strip())
    )
    can_try_graph_policy = allow_graph_policy and can_use_graph
    can_try_graph_fallback = allow_graph_fallback_on_vector_error and can_use_graph

    if can_try_graph_policy:
        try:
            policy = await _evaluate_recall_policy(
                query=query,
                limit=limit,
                retrieval_mode=retrieval_mode,
                store=active_store,
                collection=collection,
            )

            retrieval_path = policy.retrieval_path
            retrieval_reason = policy.retrieval_reason
            graph_backend = policy.graph_backend
            graph_hit_count = policy.graph_hit_count
            graph_confidence_score = policy.graph_confidence_score
            graph_confidence_level = policy.graph_confidence_level
            retrieval_plan_schema_id = policy.retrieval_plan_schema_id
            retrieval_plan_record = policy.retrieval_plan

            if policy.graph_rows:
                graph_rows = await postprocess_rows(
                    list(policy.graph_rows),
                    query,
                    limit,
                    min_score,
                    preview,
                    snippet_chars,
                    False,
                )
                return _build_response(
                    query=query,
                    keywords=keywords,
                    collection=collection,
                    preview=preview,
                    retrieval_mode=retrieval_mode,
                    retrieval_path=retrieval_path,
                    retrieval_reason=retrieval_reason,
                    graph_backend=graph_backend,
                    graph_hit_count=graph_hit_count,
                    graph_confidence_score=graph_confidence_score,
                    graph_confidence_level=graph_confidence_level,
                    retrieval_plan_schema_id=retrieval_plan_schema_id,
                    retrieval_plan=retrieval_plan_record,
                    results=graph_rows,
                )

            if policy.graph_only_empty:
                return _build_response(
                    query=query,
                    keywords=keywords,
                    collection=collection,
                    preview=preview,
                    retrieval_mode=retrieval_mode,
                    retrieval_path=retrieval_path,
                    retrieval_reason=retrieval_reason,
                    graph_backend=graph_backend,
                    graph_hit_count=graph_hit_count,
                    graph_confidence_score=graph_confidence_score,
                    graph_confidence_level=graph_confidence_level,
                    retrieval_plan_schema_id=retrieval_plan_schema_id,
                    retrieval_plan=retrieval_plan_record,
                    results=[],
                )
        except Exception as exc:
            debug("LinkGraph policy retrieval skipped: %s", exc)
            retrieval_path = "vector_only"
            retrieval_reason = "policy_error_fallback_vector"

    try:
        result_dicts = await query_rows_runner(
            vector_store=vector_store,
            query=query,
            keywords=keywords,
            collection=collection,
            fetch_limit=fetch_limit,
            use_semantic_cache=False,
            on_parse_error=lambda exc: debug("Failed to parse search result: %s", exc),
        )
    except Exception as vector_error:
        if can_try_graph_fallback:
            warning(
                "Recall vector retrieval failed; attempting graph-only fallback: %s",
                vector_error,
            )
        else:
            warning(
                "Recall vector retrieval failed; graph-only fallback disabled: %s",
                vector_error,
            )
        if can_try_graph_fallback:
            try:
                fallback_policy = await _evaluate_recall_policy(
                    query=query,
                    limit=limit,
                    retrieval_mode="graph_only",
                    store=active_store,
                    collection=collection,
                )

                fallback_path = str(fallback_policy.retrieval_path or "graph_only")
                fallback_reason = str(fallback_policy.retrieval_reason or "graph_only_empty")
                fallback_backend = str(fallback_policy.graph_backend or graph_backend)
                fallback_hit_count = max(graph_hit_count, int(fallback_policy.graph_hit_count or 0))
                fallback_confidence_score = max(
                    graph_confidence_score,
                    float(fallback_policy.graph_confidence_score or 0.0),
                )
                fallback_confidence_level = str(
                    fallback_policy.graph_confidence_level or graph_confidence_level or "none"
                )
                fallback_plan_schema_id = str(
                    fallback_policy.retrieval_plan_schema_id or retrieval_plan_schema_id or ""
                )
                fallback_plan_record = (
                    fallback_policy.retrieval_plan
                    if isinstance(fallback_policy.retrieval_plan, dict)
                    else retrieval_plan_record
                )

                if fallback_policy.graph_rows:
                    fallback_rows = await postprocess_rows(
                        list(fallback_policy.graph_rows),
                        query,
                        limit,
                        min_score,
                        preview,
                        snippet_chars,
                        False,
                    )
                    return _build_response(
                        query=query,
                        keywords=keywords,
                        collection=collection,
                        preview=preview,
                        retrieval_mode=retrieval_mode,
                        retrieval_path=fallback_path,
                        retrieval_reason=fallback_reason,
                        graph_backend=fallback_backend,
                        graph_hit_count=fallback_hit_count,
                        graph_confidence_score=fallback_confidence_score,
                        graph_confidence_level=fallback_confidence_level,
                        retrieval_plan_schema_id=fallback_plan_schema_id,
                        retrieval_plan=fallback_plan_record,
                        results=fallback_rows,
                    )

                if fallback_policy.graph_only_empty:
                    return _build_response(
                        query=query,
                        keywords=keywords,
                        collection=collection,
                        preview=preview,
                        retrieval_mode=retrieval_mode,
                        retrieval_path=fallback_path,
                        retrieval_reason=fallback_reason,
                        graph_backend=fallback_backend,
                        graph_hit_count=fallback_hit_count,
                        graph_confidence_score=fallback_confidence_score,
                        graph_confidence_level=fallback_confidence_level,
                        retrieval_plan_schema_id=fallback_plan_schema_id,
                        retrieval_plan=fallback_plan_record,
                        results=[],
                    )
            except Exception as fallback_error:
                debug(
                    "Recall graph-only fallback skipped after vector failure: %s",
                    fallback_error,
                )

        return _build_response(
            query=query,
            keywords=keywords,
            collection=collection,
            preview=preview,
            retrieval_mode=retrieval_mode,
            retrieval_path="vector_only",
            retrieval_reason="vector_error_fallback_empty",
            graph_backend=graph_backend,
            graph_hit_count=graph_hit_count,
            graph_confidence_score=graph_confidence_score,
            graph_confidence_level=graph_confidence_level,
            retrieval_plan_schema_id=retrieval_plan_schema_id,
            retrieval_plan=retrieval_plan_record,
            results=[],
        )

    result_rows = await postprocess_rows(
        result_dicts,
        query,
        limit,
        min_score,
        preview,
        snippet_chars,
        enable_fusion_boost,
    )
    return _build_response(
        query=query,
        keywords=keywords,
        collection=collection,
        preview=preview,
        retrieval_mode=retrieval_mode,
        retrieval_path=retrieval_path,
        retrieval_reason=retrieval_reason,
        graph_backend=graph_backend,
        graph_hit_count=graph_hit_count,
        graph_confidence_score=graph_confidence_score,
        graph_confidence_level=graph_confidence_level,
        retrieval_plan_schema_id=retrieval_plan_schema_id,
        retrieval_plan=retrieval_plan_record,
        results=result_rows,
    )


__all__ = ["run_recall_single_call"]
