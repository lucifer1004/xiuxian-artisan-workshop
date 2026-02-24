#!/usr/bin/env python3
"""
Benchmark all knowledge skill tools (same path as MCP: call_tool -> kernel.execute_tool).

Reports per-tool latency as if MCP is already running: init and a warm phase are run
first (embedding, vector store, validation cache warmed), then each tool is timed.
So the numbers reflect "backend already up" latency, not cold boot.

Usage (from repo root):
    uv run python scripts/benchmark_knowledge_recall.py
    uv run python scripts/benchmark_knowledge_recall.py --runs 2 --skip-slow
    uv run python scripts/benchmark_knowledge_recall.py --tools knowledge.recall knowledge.stats
    uv run python scripts/benchmark_knowledge_recall.py --no-warm-phase   # measure cold first-call
    uv run python scripts/benchmark_knowledge_recall.py --write-snapshot
"""

from __future__ import annotations

import argparse
import asyncio
import json
import sys
import time
from contextlib import suppress
from pathlib import Path
from typing import Any

from omni.test_kit.knowledge_snapshot import (
    build_knowledge_snapshot_payload,
    default_knowledge_snapshot_path,
    detect_knowledge_snapshot_anomalies,
    load_knowledge_snapshot,
    save_knowledge_snapshot,
)

# Minimal safe arguments per knowledge tool (no side effects or tiny read-only).
# Tools not listed get args {}.
KNOWLEDGE_MIN_ARGS: dict[str, dict[str, Any]] = {
    "knowledge.recall": {"query": "architecture", "limit": 2},
    "knowledge.search": {"query": "architecture", "max_results": 2},
    "knowledge.stats": {"collection": "knowledge_chunks"},
    "knowledge.link_graph_stats": {},
    "knowledge.link_graph_toc": {"limit": 5},
    "knowledge.link_graph_links": {"note_id": "index", "direction": "both"},
    "knowledge.link_graph_find_related": {"note_id": "index", "max_distance": 1, "limit": 3},
    "knowledge.get_development_context": {},
    "knowledge.consult_architecture_doc": {"topic": "overview"},
    "knowledge.consult_language_expert": {"file_path": "README.md", "task_description": "lint"},
    "knowledge.get_language_standards": {"lang": "python"},
    "knowledge.get_best_practice": {"topic": "testing"},
    "knowledge.search_graph": {"query": "architecture", "limit": 2},
    "knowledge.search_documentation": {"query": "architecture"},
    "knowledge.search_standards": {"topic": "python"},
    "knowledge.code_search": {"query": "error handling", "limit": 2},
    "knowledge.dependency_search": {"query": "serde", "limit": 2},
    "knowledge.ingest": {"content": "benchmark test", "source": "benchmark"},
    # ingest_document requires file_path; use --tools knowledge.ingest_document and pass path in code if needed
    "knowledge.ingest_knowledge": {"clean": False},
    "knowledge.knowledge_status": {},
    "knowledge.link_graph_hybrid_search": {"query": "architecture", "max_results": 2},
}


def _min_args(tool_name: str) -> dict[str, Any]:
    return KNOWLEDGE_MIN_ARGS.get(tool_name, {}).copy()


async def main() -> int:
    parser = argparse.ArgumentParser(description="Benchmark all knowledge skill tools (MCP path)")
    parser.add_argument(
        "--runs",
        type=int,
        default=1,
        help="Runs per tool (default: 1)",
    )
    parser.add_argument(
        "--no-warm",
        action="store_true",
        help="Skip validation cache warm-up (cold first call)",
    )
    parser.add_argument(
        "--no-warm-phase",
        action="store_true",
        help="Skip warm phase (stats + get_development_context + recall once). Default: run warm phase so reported latency assumes MCP already up.",
    )
    parser.add_argument(
        "--skip-slow",
        action="store_true",
        help="Skip tools that typically take >10s (e.g. ingest, full search)",
    )
    parser.add_argument(
        "--tools",
        nargs="*",
        default=None,
        help="Only run these tools (e.g. knowledge.recall knowledge.stats)",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output machine-readable JSON report",
    )
    parser.add_argument(
        "--warm-timeout",
        type=float,
        default=20.0,
        help="Timeout in seconds for validation warm-up (default: 20)",
    )
    parser.add_argument(
        "--tool-timeout",
        type=float,
        default=60.0,
        help="Timeout in seconds per tool call (default: 60)",
    )
    parser.add_argument(
        "--snapshot-file",
        type=str,
        default="",
        help=(
            "YAML snapshot path for baseline tracking. Default: "
            "<SKILLS_DIR>/_snapshots/benchmark/knowledge_tools.yaml"
        ),
    )
    parser.add_argument(
        "--write-snapshot",
        action="store_true",
        help="Write/update snapshot YAML with current benchmark results.",
    )
    parser.add_argument(
        "--snapshot-alpha",
        type=float,
        default=0.35,
        help="Snapshot baseline smoothing alpha in [0,1] when --write-snapshot (default: 0.35).",
    )
    parser.add_argument(
        "--snapshot-factor",
        type=float,
        default=2.0,
        help="Default regression factor for anomaly detection (default: 2.0).",
    )
    parser.add_argument(
        "--snapshot-delta-ms",
        type=float,
        default=40.0,
        help="Default minimum regression delta in ms for anomaly detection (default: 40.0).",
    )
    parser.add_argument(
        "--strict-snapshot",
        action="store_true",
        help="Return non-zero when snapshot detects anomalies.",
    )
    args = parser.parse_args()

    from omni.agent.server import create_agent_handler

    # 1. Init (simulates MCP startup)
    t0 = time.perf_counter()
    handler = create_agent_handler()
    await handler.initialize()
    init_ms = (time.perf_counter() - t0) * 1000
    if not args.json:
        print("Knowledge skill benchmark (per-tool latency = MCP already running)", file=sys.stderr)
        print("=" * 60, file=sys.stderr)
        print(f"[0] Handler init: {init_ms:.0f} ms (excluded from per-tool below)", file=sys.stderr)

    kernel = handler._kernel
    if not kernel or not getattr(kernel, "execute_tool", None):
        print("ERROR: Kernel or execute_tool not available", file=sys.stderr)
        return 1

    # 2. Optional warm-up (can be slow: Rust get_skill_index_sync)
    if not args.no_warm:
        t1 = time.perf_counter()
        try:
            from omni.core.skills.validation import warm_tool_schema_cache

            await asyncio.wait_for(
                asyncio.to_thread(warm_tool_schema_cache),
                timeout=args.warm_timeout,
            )
            warm_ms = (time.perf_counter() - t1) * 1000
            if not args.json:
                print(f"[1] Validation cache warm: {warm_ms:.0f} ms", file=sys.stderr)
        except TimeoutError:
            if not args.json:
                print(f"[1] Validation cache warm: TIMEOUT ({args.warm_timeout}s)", file=sys.stderr)

    # 3. Warm phase: simulate "MCP already running" (embedding, vector store, link_graph_stats cache)
    if not args.no_warm_phase and not args.json:
        print(
            "[2] Warm phase (excluded from report): stats, get_development_context, recall(limit=1)...",
            file=sys.stderr,
        )
    warm_tools = [
        ("knowledge.stats", {"collection": "knowledge_chunks"}),
        ("knowledge.get_development_context", {}),
        ("knowledge.link_graph_stats", {}),  # warms link_graph_stats 60s cache
        ("knowledge.recall", {"query": "warm", "limit": 1}),  # warms embedding + vector store
    ]
    if not args.no_warm_phase:
        for tool_name, tool_args in warm_tools:
            with suppress(Exception):
                await asyncio.wait_for(
                    kernel.execute_tool(tool_name, tool_args, caller=None),
                    timeout=args.tool_timeout,
                )

    # 4. List knowledge tools to benchmark
    core = kernel.skill_context.get_core_commands()
    knowledge_tools = sorted(c for c in core if c.startswith("knowledge."))
    if args.tools:
        knowledge_tools = [t for t in knowledge_tools if t in args.tools]
        if not knowledge_tools:
            print("No matching knowledge tools.", file=sys.stderr)
            return 1

    # Skip heavy/destructive if requested
    skip_tools = set()
    if args.skip_slow:
        skip_tools = {
            "knowledge.ingest_document",  # needs file_path, often slow
            "knowledge.ingest_knowledge",  # can reindex
            "knowledge.clear",
        }
    knowledge_tools = [t for t in knowledge_tools if t not in skip_tools]

    if not args.json:
        print(
            f"[3] Per-tool latency (MCP already warm, {len(knowledge_tools)} tools, {args.runs} run(s) each):",
            file=sys.stderr,
        )
        print("-" * 60, file=sys.stderr)

    results: list[dict[str, Any]] = []
    errors: list[tuple[str, str]] = []

    for tool_name in knowledge_tools:
        min_args = _min_args(tool_name)
        run_ms_list: list[float] = []
        last_error: str | None = None
        for _ in range(args.runs):
            t2 = time.perf_counter()
            try:
                await asyncio.wait_for(
                    kernel.execute_tool(tool_name, min_args, caller=None),
                    timeout=args.tool_timeout,
                )
                run_ms_list.append((time.perf_counter() - t2) * 1000)
            except TimeoutError:
                last_error = f"timeout ({args.tool_timeout}s)"
                run_ms_list.append(args.tool_timeout * 1000)
                break
            except Exception as e:
                last_error = str(e)
                run_ms_list.append((time.perf_counter() - t2) * 1000)
                break
        avg_ms = sum(run_ms_list) / len(run_ms_list) if run_ms_list else 0
        results.append(
            {
                "tool": tool_name,
                "avg_ms": round(avg_ms, 1),
                "runs": len(run_ms_list),
                "ok": last_error is None,
            }
        )
        if last_error:
            errors.append((tool_name, last_error))
        if not args.json:
            status = "ok" if last_error is None else "FAIL"
            print(f"  {tool_name}: {avg_ms:.0f} ms  [{status}]", file=sys.stderr)

    # 5. Report: slowest first (per-tool only; init and warm phase excluded)
    results.sort(key=lambda x: -x["avg_ms"])
    snapshot_path = (
        Path(args.snapshot_file).expanduser().resolve()
        if args.snapshot_file.strip()
        else default_knowledge_snapshot_path()
    )
    snapshot_loaded = load_knowledge_snapshot(snapshot_path)
    anomalies = detect_knowledge_snapshot_anomalies(
        results=results,
        snapshot=snapshot_loaded,
        default_regression_factor=args.snapshot_factor,
        default_min_regression_delta_ms=args.snapshot_delta_ms,
    )
    anomaly_records = [item.to_record() for item in anomalies]
    snapshot_written = False
    if args.write_snapshot:
        snapshot_payload = build_knowledge_snapshot_payload(
            results=results,
            runs_per_tool=args.runs,
            warm_phase=(not args.no_warm_phase),
            previous=snapshot_loaded,
            alpha=args.snapshot_alpha,
            default_regression_factor=args.snapshot_factor,
            default_min_regression_delta_ms=args.snapshot_delta_ms,
        )
        save_knowledge_snapshot(snapshot_path, snapshot_payload)
        snapshot_written = True

    exit_code = 0
    if errors:
        exit_code = 1
    if args.strict_snapshot and anomalies:
        exit_code = 1

    if args.json:
        out = {
            "init_ms": round(init_ms, 1),
            "warm_phase": not args.no_warm_phase,
            "tools": results,
            "errors": [{"tool": t, "error": e} for t, e in errors],
            "snapshot": {
                "path": str(snapshot_path),
                "loaded": snapshot_loaded is not None,
                "written": snapshot_written,
                "anomaly_count": len(anomaly_records),
                "anomalies": anomaly_records,
                "strict": bool(args.strict_snapshot),
            },
        }
        print(json.dumps(out, indent=2))
        return exit_code

    print("-" * 60, file=sys.stderr)
    print(
        "Report (MCP already warm; slowest first) — optimize: 1) skill-level, 2) per-tool",
        file=sys.stderr,
    )
    print("-" * 60, file=sys.stderr)
    for r in results[:15]:
        print(f"  {r['avg_ms']:>8.0f} ms  {r['tool']}", file=sys.stderr)
    if errors:
        print("\nErrors:", file=sys.stderr)
        for t, e in errors[:10]:
            print(f"  {t}: {e[:80]}", file=sys.stderr)
    print("\nOptimization order:", file=sys.stderr)
    print(
        "  1) Skill-level: shared embedding init, validation cache, vector/store open.",
        file=sys.stderr,
    )
    print(
        "  2) Per-tool: reduce I/O, cache hot paths, lower default limits for fast first response.",
        file=sys.stderr,
    )
    print("\nSnapshot tracking:", file=sys.stderr)
    print(f"  path: {snapshot_path}", file=sys.stderr)
    print(
        f"  loaded: {snapshot_loaded is not None}  written: {snapshot_written}  "
        f"anomalies: {len(anomaly_records)}",
        file=sys.stderr,
    )
    if anomaly_records:
        print("  anomaly details (large regression only):", file=sys.stderr)
        for item in anomaly_records:
            print(
                "    - {tool}: observed={observed_ms:.1f}ms baseline={baseline_ms:.1f}ms "
                "threshold={threshold_ms:.1f}ms ratio={ratio:.2f}".format(**item),
                file=sys.stderr,
            )
        if args.strict_snapshot:
            print(
                "  strict snapshot is enabled: anomalies mark this run as failed.", file=sys.stderr
            )
    elif snapshot_loaded is None:
        print(
            "  no snapshot found yet (run with --write-snapshot to create baseline).",
            file=sys.stderr,
        )

    return exit_code


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
