#!/usr/bin/env python3
"""Benchmark a curated safe set of skill tools and persist latency snapshots."""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import shutil
import statistics
import sys
import time
from contextlib import suppress
from datetime import UTC, datetime
from pathlib import Path
from typing import Any

from omni.test_kit.skills_snapshot import (
    build_skills_snapshot_payload,
    default_skills_snapshot_path,
    detect_skills_snapshot_anomalies,
    load_skills_snapshot,
    save_skills_snapshot,
)

# Curated non-destructive tools + minimal safe args.
BASE_SAFE_TOOL_ARGS: dict[str, dict[str, Any]] = {
    "advanced_tools.smart_find": {
        "pattern": "benchmark_note",
        "extension": "md",
        "search_root": "scripts/fixtures/benchmark",
    },
    "advanced_tools.smart_search": {
        "pattern": "Hard Constraints",
        "file_globs": "*.md",
        "search_root": "scripts/fixtures/benchmark",
        "context_lines": 0,
    },
    "code.code_search": {"query": "run_skill_command"},
    "demo.echo": {"message": "bench"},
    "demo.hello": {"name": "bench"},
    "demo.list_pipeline_examples": {},
    "demo.test_yaml_pipeline": {"pipeline_type": "minimal"},
    "knowledge.get_development_context": {},
    "knowledge.link_graph_search": {"query": "architecture", "max_results": 3},
    "knowledge.link_graph_stats": {},
    "knowledge.link_graph_toc": {"limit": 10},
    "knowledge.recall": {
        "query": "architecture",
        "limit": 2,
        "chunked": False,
        "retrieval_mode": "graph_only",
    },
    "knowledge.search": {"query": "architecture", "mode": "link_graph", "max_results": 3},
    "knowledge.stats": {"collection": "knowledge_chunks"},
    "memory.get_memory_stats": {},
    "omniCell.nuShell": {"command": "ls scripts/fixtures/benchmark", "intent": "observe"},
    "skill.discover": {"intent": "find files quickly"},
    "skill.list_index": {},
    "skill.list_templates": {"skill_name": "git"},
    "writer.check_markdown_structure": {"text": "# Title\n\nSample body."},
    "writer.lint_writing_style": {"text": "This is a sample sentence."},
    "writer.load_writing_memory": {},
    "writer.polish_text": {"text": "This are bad grammar."},
}

EMBEDDING_OPTIONAL_TOOL_ARGS: dict[str, dict[str, Any]] = {
    "memory.search_memory": {"query": "benchmark", "limit": 3},
}

WARM_TOOLS: tuple[tuple[str, dict[str, Any]], ...] = (
    ("demo.echo", {"message": "warm"}),
    ("knowledge.stats", {"collection": "knowledge_chunks"}),
)

CRAWL4AI_TOOL = "crawl4ai.crawl_url"
CRAWL4AI_SCENARIO_NETWORK = "network_http"
CRAWL4AI_SCENARIO_LOCAL = "local_file"
CRAWL4AI_SCENARIOS = (CRAWL4AI_SCENARIO_NETWORK, CRAWL4AI_SCENARIO_LOCAL)
SNAPSHOT_GATE_SCOPE_ALL = "all"
SNAPSHOT_GATE_SCOPE_DETERMINISTIC = "deterministic"
SNAPSHOT_GATE_SCOPES = (SNAPSHOT_GATE_SCOPE_ALL, SNAPSHOT_GATE_SCOPE_DETERMINISTIC)
SNAPSHOT_METRIC_AVG = "avg"
SNAPSHOT_METRIC_P50 = "p50"
SNAPSHOT_METRIC_P95 = "p95"
SNAPSHOT_METRIC_TRIMMED_AVG = "trimmed_avg"
SNAPSHOT_METRICS = (
    SNAPSHOT_METRIC_AVG,
    SNAPSHOT_METRIC_P50,
    SNAPSHOT_METRIC_P95,
    SNAPSHOT_METRIC_TRIMMED_AVG,
)
SNAPSHOT_NETWORK_METRIC_P95 = SNAPSHOT_METRIC_P95
SNAPSHOT_NETWORK_METRIC_TRIMMED_AVG = SNAPSHOT_METRIC_TRIMMED_AVG
SNAPSHOT_NETWORK_METRICS = (
    SNAPSHOT_NETWORK_METRIC_P95,
    SNAPSHOT_NETWORK_METRIC_TRIMMED_AVG,
)
CLI_RUNNER_TOOL = "cli.skill_run"
CLI_RUNNER_SCENARIO = "cli_runner"
CLI_RUNNER_MODE_DEFAULT_COLD = "default_cold"
CLI_RUNNER_MODE_DEFAULT_WARM = "default_warm"
CLI_RUNNER_MODE_NO_REUSE = "no_reuse"
CLI_RUNNER_MODES = (
    CLI_RUNNER_MODE_DEFAULT_COLD,
    CLI_RUNNER_MODE_DEFAULT_WARM,
    CLI_RUNNER_MODE_NO_REUSE,
)
CLI_RUNNER_CASE_IDS = tuple(f"{CLI_RUNNER_TOOL}.{mode}" for mode in CLI_RUNNER_MODES)
CLI_RUNNER_DEFAULT_COMMAND = "demo.hello"
CLI_RUNNER_DEFAULT_ARGS_JSON = '{"name":"bench"}'
CLI_RUNNER_KNOWLEDGE_PROFILE = "knowledge_search"
CLI_RUNNER_KNOWLEDGE_COMMAND = "knowledge.search"
CLI_RUNNER_KNOWLEDGE_ARGS_JSON = '{"query":"architecture","mode":"link_graph","max_results":3}'
CLI_RUNNER_KNOWLEDGE_CASE_IDS = tuple(
    f"{CLI_RUNNER_TOOL}.{CLI_RUNNER_KNOWLEDGE_PROFILE}.{mode}" for mode in CLI_RUNNER_MODES
)
CLI_RUNNER_CONTROL_TIMEOUT_SECONDS = 10.0
CLI_RUNNER_SUMMARY_SCHEMA = "omni.skills.cli_runner_summary.v1"
CLI_RUNNER_GATE_EXPECTED_ORDER = "default_warm < no_reuse < default_cold (p50_ms)"
CLI_RUNNER_GATE_DEFAULT_TOLERANCE_MS = 50.0
CLI_RUNNER_TIMING_ENV_KEY = "OMNI_SKILL_RUN_TIMING"
CLI_RUNNER_TIMING_PREFIX = "__OMNI_SKILL_TIMING__ "


def _benchmark_runner_socket_path() -> Path:
    """Return isolated socket path used by CLI runner benchmark cases."""
    return Path("/tmp").resolve() / f"xiuxian-skill-runner-benchmark-{os.getpid()}.sock"


def _cli_base_command() -> list[str]:
    """Resolve CLI base command (`omni` binary or module fallback)."""
    cli_bin = shutil.which("omni")
    if cli_bin:
        return [cli_bin]
    return [sys.executable, "-m", "omni.agent.cli.app"]


def _crawl4ai_local_fixture_url() -> str:
    """Return file:// URL for deterministic local crawl benchmark fixture."""
    fixture = (
        Path(__file__).resolve().parent / "fixtures" / "benchmark" / "crawl4ai_local_fixture.html"
    )
    return fixture.resolve().as_uri()


def _build_case_catalog(
    *,
    crawl4ai_scenarios: str,
    include_embedding_tools: bool,
    include_cli_runner_cases: bool,
) -> dict[str, dict[str, Any]]:
    """Build benchmark case catalog keyed by case id."""
    catalog: dict[str, dict[str, Any]] = {
        tool_name: {"tool": tool_name, "args": dict(tool_args)}
        for tool_name, tool_args in BASE_SAFE_TOOL_ARGS.items()
    }
    if include_embedding_tools:
        for tool_name, tool_args in EMBEDDING_OPTIONAL_TOOL_ARGS.items():
            catalog[tool_name] = {"tool": tool_name, "args": dict(tool_args)}
    if include_cli_runner_cases:
        for mode in CLI_RUNNER_MODES:
            case_id = f"{CLI_RUNNER_TOOL}.{mode}"
            catalog[case_id] = {
                "tool": CLI_RUNNER_TOOL,
                "executor": "cli_runner",
                "scenario": CLI_RUNNER_SCENARIO,
                "runner_mode": mode,
                "command": CLI_RUNNER_DEFAULT_COMMAND,
                "args_json": CLI_RUNNER_DEFAULT_ARGS_JSON,
            }
        for mode in CLI_RUNNER_MODES:
            case_id = f"{CLI_RUNNER_TOOL}.{CLI_RUNNER_KNOWLEDGE_PROFILE}.{mode}"
            catalog[case_id] = {
                "tool": CLI_RUNNER_TOOL,
                "executor": "cli_runner",
                "scenario": CLI_RUNNER_SCENARIO,
                "runner_mode": mode,
                "command": CLI_RUNNER_KNOWLEDGE_COMMAND,
                "args_json": CLI_RUNNER_KNOWLEDGE_ARGS_JSON,
            }
    crawl_base = {
        "action": "crawl",
        "fit_markdown": True,
        "max_depth": 0,
    }
    if crawl4ai_scenarios in ("both", "network"):
        catalog[CRAWL4AI_TOOL] = {
            "tool": CRAWL4AI_TOOL,
            "scenario": CRAWL4AI_SCENARIO_NETWORK,
            "args": {"url": "https://example.com", **crawl_base},
        }
    if crawl4ai_scenarios in ("both", "local"):
        catalog[f"{CRAWL4AI_TOOL}.{CRAWL4AI_SCENARIO_LOCAL}"] = {
            "tool": CRAWL4AI_TOOL,
            "scenario": CRAWL4AI_SCENARIO_LOCAL,
            "args": {"url": _crawl4ai_local_fixture_url(), **crawl_base},
        }
    return catalog


async def _run_tool_once(
    tool_name: str,
    args: dict[str, Any],
    *,
    timeout_s: float,
) -> tuple[float, str | None]:
    from omni.core.skills.runner import run_tool_with_monitor

    started = time.perf_counter()
    try:
        _result, monitor = await asyncio.wait_for(
            run_tool_with_monitor(
                tool_name,
                args,
                output_json=False,
                auto_report=False,
            ),
            timeout=timeout_s,
        )
        if monitor is not None:
            report = monitor.build_report()
            return float(report.elapsed_sec * 1000.0), None
        return float((time.perf_counter() - started) * 1000.0), None
    except Exception as exc:
        elapsed = float((time.perf_counter() - started) * 1000.0)
        return elapsed, str(exc)


async def _run_cli_command_once(
    *,
    cli_args: list[str],
    env: dict[str, str],
    timeout_s: float,
) -> tuple[float, str | None, dict[str, Any] | None]:
    """Run one CLI command and return elapsed time in ms."""
    command = [*_cli_base_command(), *cli_args]
    started = time.perf_counter()
    try:
        process = await asyncio.create_subprocess_exec(
            *command,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
            env=env,
        )
    except Exception as exc:
        elapsed = float((time.perf_counter() - started) * 1000.0)
        return elapsed, f"failed to spawn CLI command: {exc}", None

    try:
        stdout_bytes, stderr_bytes = await asyncio.wait_for(
            process.communicate(), timeout=timeout_s
        )
    except TimeoutError:
        with suppress(ProcessLookupError):
            process.kill()
        with suppress(Exception):
            await process.wait()
        elapsed = float((time.perf_counter() - started) * 1000.0)
        return elapsed, f"CLI timeout after {timeout_s:.1f}s: {' '.join(cli_args)}", None

    elapsed = float((time.perf_counter() - started) * 1000.0)
    stdout_text = stdout_bytes.decode("utf-8", errors="replace").strip()
    stderr_text = stderr_bytes.decode("utf-8", errors="replace").strip()
    timing = _parse_cli_timing_payload(stderr_text)
    if process.returncode != 0:
        details = stderr_text or stdout_text or "no output"
        return elapsed, f"CLI exited with code {process.returncode}: {details}", timing
    if not stdout_text:
        return elapsed, "CLI returned empty stdout payload", timing
    return elapsed, None, timing


def _parse_cli_timing_payload(stderr_text: str) -> dict[str, Any] | None:
    """Extract the timing payload line emitted by the JSON fast runner."""
    if not stderr_text:
        return None
    for line in reversed(stderr_text.splitlines()):
        candidate = line.strip()
        if not candidate.startswith(CLI_RUNNER_TIMING_PREFIX):
            continue
        raw_payload = candidate.removeprefix(CLI_RUNNER_TIMING_PREFIX).strip()
        if not raw_payload:
            continue
        try:
            parsed = json.loads(raw_payload)
        except Exception:
            return None
        if isinstance(parsed, dict):
            return parsed
        return None
    return None


async def _runner_control_stop_best_effort(*, socket_path: Path, timeout_s: float) -> None:
    """Best-effort stop for isolated benchmark daemon socket."""
    env = os.environ.copy()
    env["OMNI_SKILL_RUNNER_SOCKET"] = str(socket_path)
    await _run_cli_command_once(
        cli_args=["skill", "runner", "stop", "--json"],
        env=env,
        timeout_s=max(1.0, min(timeout_s, CLI_RUNNER_CONTROL_TIMEOUT_SECONDS)),
    )


async def _run_cli_runner_case_once(
    case: dict[str, Any],
    *,
    timeout_s: float,
) -> tuple[float, str | None, dict[str, Any] | None]:
    """Run one CLI runner benchmark case sample."""
    runner_mode = str(case.get("runner_mode") or "").strip()
    if runner_mode not in CLI_RUNNER_MODES:
        return 0.0, f"unsupported CLI runner mode: {runner_mode or '<empty>'}", None

    command_name = str(case.get("command") or "").strip()
    args_json = str(case.get("args_json") or "").strip()
    if not command_name:
        return 0.0, "CLI runner case is missing command", None

    socket_path = _benchmark_runner_socket_path()
    env = os.environ.copy()
    env["OMNI_SKILL_RUNNER_SOCKET"] = str(socket_path)
    env[CLI_RUNNER_TIMING_ENV_KEY] = "1"

    if runner_mode == CLI_RUNNER_MODE_DEFAULT_COLD:
        await _runner_control_stop_best_effort(socket_path=socket_path, timeout_s=timeout_s)

    cli_args = ["skill", "run", command_name]
    if args_json:
        cli_args.append(args_json)
    cli_args.append("--json")
    if runner_mode == CLI_RUNNER_MODE_NO_REUSE:
        cli_args.append("--no-reuse-process")

    return await _run_cli_command_once(
        cli_args=cli_args,
        env=env,
        timeout_s=timeout_s,
    )


async def _prepare_cli_runner_case(
    case: dict[str, Any],
    *,
    timeout_s: float,
) -> str | None:
    """Prepare CLI case setup before measured runs."""
    runner_mode = str(case.get("runner_mode") or "").strip()
    if runner_mode not in CLI_RUNNER_MODES:
        return f"unsupported CLI runner mode: {runner_mode or '<empty>'}"

    socket_path = _benchmark_runner_socket_path()
    await _runner_control_stop_best_effort(socket_path=socket_path, timeout_s=timeout_s)
    if runner_mode != CLI_RUNNER_MODE_DEFAULT_WARM:
        return None

    warm_case = dict(case)
    warm_case["runner_mode"] = CLI_RUNNER_MODE_DEFAULT_COLD
    _elapsed_ms, warm_err, _timing = await _run_cli_runner_case_once(
        warm_case,
        timeout_s=timeout_s,
    )
    return warm_err


async def _cleanup_cli_runner_case(timeout_s: float) -> None:
    """Cleanup benchmark runner daemon socket/process after CLI case loop."""
    await _runner_control_stop_best_effort(
        socket_path=_benchmark_runner_socket_path(),
        timeout_s=timeout_s,
    )


async def _run_global_warm_phase(timeout_s: float) -> None:
    for tool_name, args in WARM_TOOLS:
        try:
            await _run_tool_once(tool_name, args, timeout_s=timeout_s)
        except Exception:
            continue


async def _run_case_warm_phase(cases: list[dict[str, Any]], timeout_s: float) -> None:
    """Warm each selected case once to reduce first-call cold-start skew."""
    for case in cases:
        tool_name = str(case.get("tool") or "").strip()
        args = case.get("args")
        if not tool_name or not isinstance(args, dict):
            continue
        try:
            await _run_tool_once(tool_name, dict(args), timeout_s=timeout_s)
        except Exception:
            continue


async def _close_open_clients_if_loaded() -> None:
    """Close lazily loaded HTTP clients to avoid unclosed-session warnings."""
    if "omni.foundation.embedding_client" in sys.modules:
        with suppress(Exception):
            from omni.foundation.embedding_client import close_embedding_client

            await close_embedding_client()
    if "omni.agent.cli.mcp_embed" in sys.modules:
        with suppress(Exception):
            from omni.agent.cli.mcp_embed import close_shared_http_client

            await close_shared_http_client()


def _percentile(samples_ms: list[float], percentile: float) -> float:
    """Compute percentile via linear interpolation on sorted samples."""
    if not samples_ms:
        return 0.0
    ordered = sorted(float(value) for value in samples_ms)
    if len(ordered) == 1:
        return ordered[0]
    bounded = max(0.0, min(100.0, float(percentile)))
    rank = (len(ordered) - 1) * (bounded / 100.0)
    lower = int(rank)
    upper = min(lower + 1, len(ordered) - 1)
    weight = rank - float(lower)
    return (ordered[lower] * (1.0 - weight)) + (ordered[upper] * weight)


def _trimmed_mean(samples_ms: list[float], trim_ratio: float = 0.2) -> float | None:
    """Compute trimmed mean for noisy scenarios; returns None for insufficient samples."""
    if len(samples_ms) < 5:
        return None
    ordered = sorted(float(value) for value in samples_ms)
    clamped = max(0.0, min(0.49, float(trim_ratio)))
    trim_count = int(len(ordered) * clamped)
    if trim_count <= 0 or (trim_count * 2) >= len(ordered):
        return sum(ordered) / len(ordered)
    trimmed = ordered[trim_count : len(ordered) - trim_count]
    if not trimmed:
        return sum(ordered) / len(ordered)
    return sum(trimmed) / len(trimmed)


def _resolve_snapshot_metric_observed(
    item: dict[str, Any],
    *,
    metric: str,
) -> tuple[float | None, str | None]:
    """Resolve observed metric value with robust fallback chain."""
    metric_to_fields: dict[str, tuple[str, ...]] = {
        SNAPSHOT_METRIC_AVG: ("avg_ms", "p50_ms", "trimmed_avg_ms", "p95_ms"),
        SNAPSHOT_METRIC_P50: ("p50_ms", "avg_ms", "trimmed_avg_ms", "p95_ms"),
        SNAPSHOT_METRIC_P95: ("p95_ms", "avg_ms", "p50_ms", "trimmed_avg_ms"),
        SNAPSHOT_METRIC_TRIMMED_AVG: ("trimmed_avg_ms", "p50_ms", "avg_ms", "p95_ms"),
    }
    fallback_fields = metric_to_fields.get(metric, metric_to_fields[SNAPSHOT_METRIC_P50])
    for field in fallback_fields:
        raw = item.get(field)
        if isinstance(raw, int | float) and not isinstance(raw, bool):
            return float(raw), field
    return None, None


def _prepare_results_for_snapshot_gate(
    results: list[dict[str, Any]],
    *,
    gate_scope: str,
    default_metric: str,
    network_metric: str,
) -> list[dict[str, Any]]:
    """Filter/annotate benchmark results used for snapshot anomaly gating."""
    prepared: list[dict[str, Any]] = []
    for item in results:
        scenario = item.get("scenario")
        if (
            gate_scope == SNAPSHOT_GATE_SCOPE_DETERMINISTIC
            and isinstance(scenario, str)
            and scenario == CRAWL4AI_SCENARIO_NETWORK
        ):
            continue
        entry = dict(item)
        selected_metric = (
            network_metric
            if isinstance(scenario, str) and scenario == CRAWL4AI_SCENARIO_NETWORK
            else default_metric
        )
        observed_ms, observed_field = _resolve_snapshot_metric_observed(
            entry,
            metric=selected_metric,
        )
        if observed_ms is not None and observed_field:
            entry["anomaly_observed_ms"] = observed_ms
            entry["anomaly_observed_metric"] = observed_field
        prepared.append(entry)
    return prepared


def _parse_cli_runner_case_id(case_id: str) -> tuple[str, str] | None:
    """Parse CLI runner case id into `(profile, mode)`."""
    prefix = f"{CLI_RUNNER_TOOL}."
    raw = str(case_id).strip()
    if not raw.startswith(prefix):
        return None
    for mode in CLI_RUNNER_MODES:
        suffix = f".{mode}"
        if not raw.endswith(suffix):
            continue
        middle = raw[len(prefix) : -len(suffix)].strip(".")
        profile = middle if middle else "demo_hello"
        return profile, mode
    return None


def _round_or_none(value: float | None) -> float | None:
    """Round float to 2 decimals when present."""
    if value is None:
        return None
    return round(float(value), 2)


def _safe_ratio(numerator: float | None, denominator: float | None) -> float | None:
    """Return stable ratio or None when denominator is invalid."""
    if numerator is None or denominator is None:
        return None
    if denominator <= 0:
        return None
    return float(numerator / denominator)


def _metric_as_float(item: dict[str, Any], key: str) -> float | None:
    """Best-effort metric extraction from benchmark result entry."""
    raw = item.get(key)
    if isinstance(raw, bool):
        return None
    if isinstance(raw, int | float):
        return float(raw)
    return None


def _timing_metric_as_float(timing: dict[str, Any] | None, key: str) -> float | None:
    """Extract numeric timing metric from parsed CLI timing payload."""
    if not isinstance(timing, dict):
        return None
    raw = timing.get(key)
    if isinstance(raw, bool):
        return None
    if isinstance(raw, int | float):
        return float(raw)
    return None


def _build_timing_breakdown(
    *,
    bootstrap_samples_ms: list[float],
    daemon_connect_samples_ms: list[float],
    tool_exec_samples_ms: list[float],
) -> dict[str, Any]:
    """Build stable per-phase timing breakdown for one benchmark case."""

    def _phase(samples_ms: list[float]) -> dict[str, Any]:
        if not samples_ms:
            return {"samples": 0, "avg_ms": None, "p50_ms": None, "p95_ms": None}
        avg_ms = sum(samples_ms) / len(samples_ms)
        return {
            "samples": len(samples_ms),
            "avg_ms": _round_or_none(avg_ms),
            "p50_ms": _round_or_none(_percentile(samples_ms, 50.0)),
            "p95_ms": _round_or_none(_percentile(samples_ms, 95.0)),
        }

    return {
        "bootstrap": _phase(bootstrap_samples_ms),
        "daemon_connect": _phase(daemon_connect_samples_ms),
        "tool_exec": _phase(tool_exec_samples_ms),
    }


def _build_cli_runner_summary(results: list[dict[str, Any]]) -> dict[str, Any]:
    """Build grouped CLI runner latency summary for quick profiling."""
    summary: dict[str, Any] = {
        "case_count": 0,
        "profiles": {},
        "rank_by_p50_ms": [],
    }

    profiles: dict[str, dict[str, Any]] = {}
    ranking: list[dict[str, Any]] = []
    for item in results:
        case_id = str(item.get("tool") or "").strip()
        parsed = _parse_cli_runner_case_id(case_id)
        if parsed is None:
            continue
        profile, mode = parsed
        command = str(item.get("command") or "").strip()
        if not command:
            command = (
                CLI_RUNNER_KNOWLEDGE_COMMAND
                if profile == CLI_RUNNER_KNOWLEDGE_PROFILE
                else CLI_RUNNER_DEFAULT_COMMAND
            )
        profile_entry = profiles.setdefault(
            profile,
            {
                "command": command,
                "cases": {},
                "comparisons": {},
            },
        )
        avg_ms = _metric_as_float(item, "avg_ms")
        p50_ms = _metric_as_float(item, "p50_ms")
        p95_ms = _metric_as_float(item, "p95_ms")
        profile_entry["cases"][mode] = {
            "tool": case_id,
            "avg_ms": _round_or_none(avg_ms),
            "p50_ms": _round_or_none(p50_ms),
            "p95_ms": _round_or_none(p95_ms),
            "ok": bool(item.get("ok", False)),
        }
        timing_breakdown = item.get("timing_breakdown_ms")
        if isinstance(timing_breakdown, dict):
            profile_entry["cases"][mode]["timing_breakdown_ms"] = dict(timing_breakdown)
        ranking.append(
            {
                "tool": case_id,
                "profile": profile,
                "mode": mode,
                "p50_ms": _round_or_none(p50_ms),
                "p95_ms": _round_or_none(p95_ms),
                "ok": bool(item.get("ok", False)),
            }
        )

    for _profile, profile_entry in profiles.items():
        cases = profile_entry.get("cases", {})
        warm = cases.get(CLI_RUNNER_MODE_DEFAULT_WARM, {})
        no_reuse = cases.get(CLI_RUNNER_MODE_NO_REUSE, {})
        cold = cases.get(CLI_RUNNER_MODE_DEFAULT_COLD, {})
        warm_p50 = warm.get("p50_ms")
        no_reuse_p50 = no_reuse.get("p50_ms")
        cold_p50 = cold.get("p50_ms")

        profile_entry["comparisons"] = {
            "warm_vs_no_reuse_p50_delta_ms": _round_or_none(
                (warm_p50 - no_reuse_p50)
                if isinstance(warm_p50, int | float) and isinstance(no_reuse_p50, int | float)
                else None
            ),
            "warm_vs_no_reuse_p50_ratio": _round_or_none(
                _safe_ratio(warm_p50, no_reuse_p50)
                if isinstance(warm_p50, int | float) and isinstance(no_reuse_p50, int | float)
                else None
            ),
            "warm_vs_default_cold_p50_delta_ms": _round_or_none(
                (warm_p50 - cold_p50)
                if isinstance(warm_p50, int | float) and isinstance(cold_p50, int | float)
                else None
            ),
            "warm_vs_default_cold_p50_ratio": _round_or_none(
                _safe_ratio(warm_p50, cold_p50)
                if isinstance(warm_p50, int | float) and isinstance(cold_p50, int | float)
                else None
            ),
        }

    ranking.sort(
        key=lambda entry: float(entry["p50_ms"])
        if isinstance(entry.get("p50_ms"), int | float)
        else 0.0,
        reverse=True,
    )
    summary["profiles"] = dict(sorted(profiles.items(), key=lambda item: item[0]))
    summary["rank_by_p50_ms"] = ranking
    summary["case_count"] = len(ranking)
    return summary


def _evaluate_cli_runner_ordering(
    cli_runner_summary: dict[str, Any],
    *,
    tolerance_ms: float = CLI_RUNNER_GATE_DEFAULT_TOLERANCE_MS,
) -> dict[str, Any]:
    """Validate expected CLI p50 ordering per profile with jitter tolerance."""
    jitter_tolerance_ms = max(0.0, float(tolerance_ms))
    profiles_raw = cli_runner_summary.get("profiles", {})
    profiles = profiles_raw if isinstance(profiles_raw, dict) else {}

    violations: list[dict[str, Any]] = []
    checked_profile_count = 0
    for profile_name, profile_entry in profiles.items():
        if not isinstance(profile_entry, dict):
            continue
        cases = profile_entry.get("cases", {})
        if not isinstance(cases, dict):
            continue
        checked_profile_count += 1

        warm_case = cases.get(CLI_RUNNER_MODE_DEFAULT_WARM)
        no_reuse_case = cases.get(CLI_RUNNER_MODE_NO_REUSE)
        cold_case = cases.get(CLI_RUNNER_MODE_DEFAULT_COLD)
        command = str(profile_entry.get("command") or "")

        missing_modes = [
            mode
            for mode, value in (
                (CLI_RUNNER_MODE_DEFAULT_WARM, warm_case),
                (CLI_RUNNER_MODE_NO_REUSE, no_reuse_case),
                (CLI_RUNNER_MODE_DEFAULT_COLD, cold_case),
            )
            if not isinstance(value, dict)
        ]
        if missing_modes:
            violations.append(
                {
                    "profile": profile_name,
                    "command": command,
                    "reason": "missing_modes",
                    "missing_modes": missing_modes,
                    "expected_order": CLI_RUNNER_GATE_EXPECTED_ORDER,
                    "tolerance_ms": round(jitter_tolerance_ms, 2),
                }
            )
            continue

        warm_ok = bool(warm_case.get("ok", False))
        no_reuse_ok = bool(no_reuse_case.get("ok", False))
        cold_ok = bool(cold_case.get("ok", False))
        if not (warm_ok and no_reuse_ok and cold_ok):
            violations.append(
                {
                    "profile": profile_name,
                    "command": command,
                    "reason": "case_not_ok",
                    "ok": {
                        CLI_RUNNER_MODE_DEFAULT_WARM: warm_ok,
                        CLI_RUNNER_MODE_NO_REUSE: no_reuse_ok,
                        CLI_RUNNER_MODE_DEFAULT_COLD: cold_ok,
                    },
                    "expected_order": CLI_RUNNER_GATE_EXPECTED_ORDER,
                    "tolerance_ms": round(jitter_tolerance_ms, 2),
                }
            )
            continue

        warm_p50 = warm_case.get("p50_ms")
        no_reuse_p50 = no_reuse_case.get("p50_ms")
        cold_p50 = cold_case.get("p50_ms")
        if not (
            isinstance(warm_p50, int | float)
            and isinstance(no_reuse_p50, int | float)
            and isinstance(cold_p50, int | float)
        ):
            violations.append(
                {
                    "profile": profile_name,
                    "command": command,
                    "reason": "invalid_p50",
                    "p50_ms": {
                        CLI_RUNNER_MODE_DEFAULT_WARM: warm_p50,
                        CLI_RUNNER_MODE_NO_REUSE: no_reuse_p50,
                        CLI_RUNNER_MODE_DEFAULT_COLD: cold_p50,
                    },
                    "expected_order": CLI_RUNNER_GATE_EXPECTED_ORDER,
                    "tolerance_ms": round(jitter_tolerance_ms, 2),
                }
            )
            continue

        warm_vs_no_reuse_ok = float(warm_p50) <= float(no_reuse_p50) + jitter_tolerance_ms
        no_reuse_vs_cold_ok = float(no_reuse_p50) <= float(cold_p50) + jitter_tolerance_ms
        if not (warm_vs_no_reuse_ok and no_reuse_vs_cold_ok):
            violations.append(
                {
                    "profile": profile_name,
                    "command": command,
                    "reason": "ordering_violation",
                    "p50_ms": {
                        CLI_RUNNER_MODE_DEFAULT_WARM: round(float(warm_p50), 2),
                        CLI_RUNNER_MODE_NO_REUSE: round(float(no_reuse_p50), 2),
                        CLI_RUNNER_MODE_DEFAULT_COLD: round(float(cold_p50), 2),
                    },
                    "expected_order": CLI_RUNNER_GATE_EXPECTED_ORDER,
                    "tolerance_ms": round(jitter_tolerance_ms, 2),
                }
            )

    return {
        "expected_order": CLI_RUNNER_GATE_EXPECTED_ORDER,
        "tolerance_ms": round(jitter_tolerance_ms, 2),
        "checked_profile_count": checked_profile_count,
        "violation_count": len(violations),
        "violations": violations,
    }


def _build_cli_runner_summary_artifact(
    *,
    cli_runner_summary: dict[str, Any],
    runs_per_tool: int,
    warm_phase: bool,
    gate_scope: str,
    checked_case_count: int,
    anomaly_count: int,
) -> dict[str, Any]:
    """Build standalone artifact payload for long-term CLI runner trend tracking."""
    return {
        "schema": CLI_RUNNER_SUMMARY_SCHEMA,
        "generated_at_utc": datetime.now(UTC).isoformat(),
        "benchmark": {
            "runs_per_tool": int(runs_per_tool),
            "warm_phase": bool(warm_phase),
        },
        "snapshot": {
            "gate_scope": str(gate_scope),
            "checked_case_count": int(checked_case_count),
            "anomaly_count": int(anomaly_count),
        },
        "cli_runner_summary": dict(cli_runner_summary),
    }


def _write_json_file(path: Path, payload: dict[str, Any]) -> Path:
    """Write JSON artifact to file with stable formatting."""
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return path


async def _benchmark_tools(
    cases: list[dict[str, Any]],
    *,
    runs: int,
    timeout_s: float,
) -> tuple[list[dict[str, Any]], list[dict[str, str]]]:
    results: list[dict[str, Any]] = []
    errors: list[dict[str, str]] = []

    for case in cases:
        case_id = str(case.get("case_id") or "").strip()
        tool_name = str(case.get("tool") or "").strip()
        executor = str(case.get("executor") or "tool").strip().lower()
        if not tool_name:
            continue
        args = dict(case.get("args") or {})
        scenario = case.get("scenario")
        samples_ms: list[float] = []
        error_text: str | None = None
        bootstrap_samples_ms: list[float] = []
        daemon_connect_samples_ms: list[float] = []
        tool_exec_samples_ms: list[float] = []

        if executor == "cli_runner":
            prepare_err = await _prepare_cli_runner_case(case, timeout_s=timeout_s)
            if prepare_err is not None:
                error_text = prepare_err

        if error_text is None:
            for _ in range(runs):
                if executor == "cli_runner":
                    elapsed_ms, err, timing = await _run_cli_runner_case_once(
                        case,
                        timeout_s=timeout_s,
                    )
                    bootstrap_ms = _timing_metric_as_float(timing, "bootstrap_ms")
                    daemon_connect_ms = _timing_metric_as_float(timing, "daemon_connect_ms")
                    tool_exec_ms = _timing_metric_as_float(timing, "tool_exec_ms")
                    if isinstance(bootstrap_ms, float):
                        bootstrap_samples_ms.append(bootstrap_ms)
                    if isinstance(daemon_connect_ms, float):
                        daemon_connect_samples_ms.append(daemon_connect_ms)
                    if isinstance(tool_exec_ms, float):
                        tool_exec_samples_ms.append(tool_exec_ms)
                else:
                    elapsed_ms, err = await _run_tool_once(tool_name, args, timeout_s=timeout_s)
                samples_ms.append(elapsed_ms)
                if err is not None:
                    error_text = err
                    break

        if executor == "cli_runner":
            await _cleanup_cli_runner_case(timeout_s=timeout_s)

        avg_ms = sum(samples_ms) / len(samples_ms) if samples_ms else 0.0
        p50_ms = _percentile(samples_ms, 50.0)
        p95_ms = _percentile(samples_ms, 95.0)
        min_ms = min(samples_ms) if samples_ms else 0.0
        max_ms = max(samples_ms) if samples_ms else 0.0
        stdev_ms = statistics.pstdev(samples_ms) if len(samples_ms) > 1 else 0.0
        trimmed_avg_ms = _trimmed_mean(samples_ms)
        result = {
            "tool": case_id or tool_name,
            "tool_name": tool_name,
            "avg_ms": round(avg_ms, 2),
            "p50_ms": round(p50_ms, 2),
            "p95_ms": round(p95_ms, 2),
            "min_ms": round(min_ms, 2),
            "max_ms": round(max_ms, 2),
            "stdev_ms": round(stdev_ms, 2),
            "runs": len(samples_ms),
            "ok": error_text is None,
        }
        if trimmed_avg_ms is not None:
            result["trimmed_avg_ms"] = round(float(trimmed_avg_ms), 2)
        if isinstance(scenario, str) and scenario:
            result["scenario"] = scenario
        if executor == "cli_runner":
            result["timing_breakdown_ms"] = _build_timing_breakdown(
                bootstrap_samples_ms=bootstrap_samples_ms,
                daemon_connect_samples_ms=daemon_connect_samples_ms,
                tool_exec_samples_ms=tool_exec_samples_ms,
            )
        results.append(result)
        if error_text is not None:
            errors.append({"tool": case_id or tool_name, "error": error_text})

    results.sort(key=lambda item: float(item.get("avg_ms", 0.0)), reverse=True)
    return results, errors


def _resolve_cases(
    requested: list[str] | None,
    *,
    crawl4ai_scenarios: str,
    include_embedding_tools: bool,
    include_cli_runner_cases: bool,
) -> list[dict[str, Any]]:
    catalog = _build_case_catalog(
        crawl4ai_scenarios=crawl4ai_scenarios,
        include_embedding_tools=include_embedding_tools,
        include_cli_runner_cases=include_cli_runner_cases,
    )
    if not requested:
        return [
            dict(case, case_id=case_id)
            for case_id, case in sorted(catalog.items(), key=lambda x: x[0])
        ]
    selected_ids: list[str] = []
    seen: set[str] = set()
    for name in requested:
        tool_name = str(name).strip()
        if not tool_name or tool_name in seen:
            continue
        seen.add(tool_name)
        if tool_name == CRAWL4AI_TOOL:
            for case_id in (CRAWL4AI_TOOL, f"{CRAWL4AI_TOOL}.{CRAWL4AI_SCENARIO_LOCAL}"):
                if case_id in catalog and case_id not in selected_ids:
                    selected_ids.append(case_id)
            continue
        if tool_name == CLI_RUNNER_TOOL:
            for case_id in (*CLI_RUNNER_CASE_IDS, *CLI_RUNNER_KNOWLEDGE_CASE_IDS):
                if case_id in catalog and case_id not in selected_ids:
                    selected_ids.append(case_id)
            continue
        if tool_name == f"{CLI_RUNNER_TOOL}.{CLI_RUNNER_KNOWLEDGE_PROFILE}":
            for case_id in CLI_RUNNER_KNOWLEDGE_CASE_IDS:
                if case_id in catalog and case_id not in selected_ids:
                    selected_ids.append(case_id)
            continue
        if tool_name in catalog:
            selected_ids.append(tool_name)
            continue
    return [dict(catalog[case_id], case_id=case_id) for case_id in selected_ids]


async def main() -> int:
    parser = argparse.ArgumentParser(
        description="Benchmark curated safe skill tools and track latency snapshots."
    )
    parser.add_argument("--runs", type=int, default=1, help="Runs per tool (default: 1)")
    parser.add_argument(
        "--tools",
        nargs="*",
        default=None,
        help="Optional subset of tools to benchmark (space-separated).",
    )
    parser.add_argument(
        "--tool-timeout",
        type=float,
        default=30.0,
        help="Per-tool timeout in seconds (default: 30).",
    )
    parser.add_argument(
        "--no-warm-phase",
        action="store_true",
        help="Skip warm-up phase before measurement.",
    )
    parser.add_argument("--json", action="store_true", help="Emit machine-readable JSON report.")
    parser.add_argument(
        "--snapshot-file",
        type=str,
        default="",
        help=(
            "YAML snapshot path for baseline tracking. Default: "
            "<SKILLS_DIR>/_snapshots/benchmark/skills_tools.yaml"
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
    parser.add_argument(
        "--enforce-cli-ordering",
        action="store_true",
        help=(f"Enforce CLI latency ordering per profile: {CLI_RUNNER_GATE_EXPECTED_ORDER}."),
    )
    parser.add_argument(
        "--cli-ordering-tolerance-ms",
        type=float,
        default=CLI_RUNNER_GATE_DEFAULT_TOLERANCE_MS,
        help=(
            "Jitter tolerance for CLI ordering checks in ms "
            f"(default: {CLI_RUNNER_GATE_DEFAULT_TOLERANCE_MS})."
        ),
    )
    parser.add_argument(
        "--crawl4ai-scenarios",
        choices=["both", "local", "network"],
        default="both",
        help="Benchmark crawl4ai in both/local/network scenarios (default: both).",
    )
    parser.add_argument(
        "--snapshot-gate-scope",
        choices=list(SNAPSHOT_GATE_SCOPES),
        default=SNAPSHOT_GATE_SCOPE_DETERMINISTIC,
        help=(
            "Snapshot anomaly gate scope. 'deterministic' excludes known noisy "
            "network_http cases; 'all' checks every case."
        ),
    )
    parser.add_argument(
        "--snapshot-default-metric",
        choices=list(SNAPSHOT_METRICS),
        default=SNAPSHOT_METRIC_P50,
        help=(
            "Default metric used for snapshot anomaly gate in deterministic cases: "
            "avg|p50|p95|trimmed_avg (default: p50)."
        ),
    )
    parser.add_argument(
        "--snapshot-network-metric",
        choices=list(SNAPSHOT_NETWORK_METRICS),
        default=SNAPSHOT_NETWORK_METRIC_TRIMMED_AVG,
        help=(
            "Metric used for crawl4ai network_http anomaly gate when included: "
            "'trimmed_avg' (default) or 'p95'."
        ),
    )
    parser.add_argument(
        "--include-embedding-tools",
        action="store_true",
        help=(
            "Include embedding-dependent tools (for example memory.search_memory). "
            "Disabled by default to keep deterministic gate independent of embedding services."
        ),
    )
    parser.add_argument(
        "--no-cli-runner-cases",
        action="store_true",
        help=(
            "Disable CLI `skill run` cold/warm/no-reuse benchmark cases. "
            "By default these cases are included in deterministic gate snapshots."
        ),
    )
    parser.add_argument(
        "--cli-summary-file",
        type=str,
        default="",
        help=(
            "Optional JSON artifact path for standalone cli_runner_summary output "
            "(useful for long-term trend tracking)."
        ),
    )
    args = parser.parse_args()

    cases = _resolve_cases(
        args.tools,
        crawl4ai_scenarios=args.crawl4ai_scenarios,
        include_embedding_tools=bool(args.include_embedding_tools),
        include_cli_runner_cases=not bool(args.no_cli_runner_cases),
    )
    if not cases:
        print("No benchmarkable tools selected.", file=sys.stderr)
        return 1

    if not args.no_warm_phase:
        # Only run global warm-up for full-suite runs.
        # For targeted `--tools` benchmarks, case-specific warm-up is sufficient
        # and avoids unrelated service dependencies (e.g. embedding endpoint).
        if args.tools is None:
            await _run_global_warm_phase(timeout_s=args.tool_timeout)
        await _run_case_warm_phase(cases, timeout_s=args.tool_timeout)

    results, errors = await _benchmark_tools(
        cases,
        runs=max(1, int(args.runs)),
        timeout_s=max(0.1, float(args.tool_timeout)),
    )

    snapshot_path = (
        Path(args.snapshot_file).expanduser().resolve()
        if args.snapshot_file.strip()
        else default_skills_snapshot_path()
    )
    snapshot_loaded = load_skills_snapshot(snapshot_path)
    snapshot_gate_results = _prepare_results_for_snapshot_gate(
        results,
        gate_scope=str(args.snapshot_gate_scope),
        default_metric=str(args.snapshot_default_metric),
        network_metric=str(args.snapshot_network_metric),
    )
    anomalies = detect_skills_snapshot_anomalies(
        results=snapshot_gate_results,
        snapshot=snapshot_loaded,
        default_regression_factor=args.snapshot_factor,
        default_min_regression_delta_ms=args.snapshot_delta_ms,
    )
    anomaly_records = [item.to_record() for item in anomalies]
    cli_runner_summary = _build_cli_runner_summary(results)
    cli_runner_gate = _evaluate_cli_runner_ordering(
        cli_runner_summary,
        tolerance_ms=max(0.0, float(args.cli_ordering_tolerance_ms)),
    )

    snapshot_written = False
    if args.write_snapshot:
        payload = build_skills_snapshot_payload(
            results=results,
            runs_per_tool=max(1, int(args.runs)),
            warm_phase=(not args.no_warm_phase),
            previous=snapshot_loaded,
            alpha=args.snapshot_alpha,
            default_regression_factor=args.snapshot_factor,
            default_min_regression_delta_ms=args.snapshot_delta_ms,
        )
        save_skills_snapshot(snapshot_path, payload)
        snapshot_written = True

    report = {
        "warm_phase": not args.no_warm_phase,
        "runs_per_tool": max(1, int(args.runs)),
        "tools": results,
        "cli_runner_summary": cli_runner_summary,
        "errors": errors,
        "cli_runner_gate": {
            "enabled": bool(args.enforce_cli_ordering),
            **cli_runner_gate,
        },
        "snapshot": {
            "path": str(snapshot_path),
            "loaded": snapshot_loaded is not None,
            "written": snapshot_written,
            "gate_scope": str(args.snapshot_gate_scope),
            "default_metric": str(args.snapshot_default_metric),
            "network_metric": str(args.snapshot_network_metric),
            "checked_case_count": len(snapshot_gate_results),
            "anomaly_count": len(anomaly_records),
            "anomalies": anomaly_records,
            "strict": bool(args.strict_snapshot),
        },
    }

    cli_summary_artifact_path = ""
    if str(args.cli_summary_file).strip():
        cli_summary_artifact = _build_cli_runner_summary_artifact(
            cli_runner_summary=cli_runner_summary,
            runs_per_tool=max(1, int(args.runs)),
            warm_phase=(not args.no_warm_phase),
            gate_scope=str(args.snapshot_gate_scope),
            checked_case_count=len(snapshot_gate_results),
            anomaly_count=len(anomaly_records),
        )
        path = Path(str(args.cli_summary_file)).expanduser().resolve()
        _write_json_file(path, cli_summary_artifact)
        cli_summary_artifact_path = str(path)
        report["artifacts"] = {"cli_runner_summary": cli_summary_artifact_path}

    exit_code = 0
    if errors:
        exit_code = 1
    if args.strict_snapshot and anomalies:
        exit_code = 1
    if bool(args.enforce_cli_ordering) and int(cli_runner_gate.get("violation_count", 0)) > 0:
        exit_code = 1

    if args.json:
        print(json.dumps(report, ensure_ascii=False, indent=2))
    else:
        print("Skill Tools Benchmark (safe set)")
        print("=" * 60)
        print(
            f"tools={len(results)} runs={max(1, int(args.runs))} warm_phase={not args.no_warm_phase}"
        )
        print("-" * 60)
        for item in results:
            status = "ok" if item.get("ok") else "FAIL"
            print(
                f"{item['tool']:<38} "
                f"avg={item['avg_ms']:>7.2f}ms "
                f"p50={item['p50_ms']:>7.2f}ms "
                f"p95={item['p95_ms']:>7.2f}ms "
                f"[{status}]"
            )
        if cli_runner_summary.get("case_count", 0) > 0:
            print("-" * 60)
            print("CLI runner summary (p50, ms):")
            profiles = cli_runner_summary.get("profiles", {})
            if isinstance(profiles, dict):
                for profile_name, profile_entry in sorted(
                    profiles.items(), key=lambda item: item[0]
                ):
                    if not isinstance(profile_entry, dict):
                        continue
                    cases = profile_entry.get("cases", {})
                    if not isinstance(cases, dict):
                        continue
                    warm_p50 = (
                        cases.get(CLI_RUNNER_MODE_DEFAULT_WARM, {}).get("p50_ms")
                        if isinstance(cases.get(CLI_RUNNER_MODE_DEFAULT_WARM), dict)
                        else None
                    )
                    no_reuse_p50 = (
                        cases.get(CLI_RUNNER_MODE_NO_REUSE, {}).get("p50_ms")
                        if isinstance(cases.get(CLI_RUNNER_MODE_NO_REUSE), dict)
                        else None
                    )
                    cold_p50 = (
                        cases.get(CLI_RUNNER_MODE_DEFAULT_COLD, {}).get("p50_ms")
                        if isinstance(cases.get(CLI_RUNNER_MODE_DEFAULT_COLD), dict)
                        else None
                    )
                    command = str(profile_entry.get("command") or "")
                    print(
                        f"- {profile_name:<18} "
                        f"command={command:<20} "
                        f"cold={cold_p50} warm={warm_p50} no_reuse={no_reuse_p50}"
                    )
        if bool(args.enforce_cli_ordering):
            print("-" * 60)
            print(
                "CLI ordering gate: "
                f"tolerance_ms={cli_runner_gate.get('tolerance_ms', 0)} "
                f"checked_profiles={cli_runner_gate.get('checked_profile_count', 0)} "
                f"violations={cli_runner_gate.get('violation_count', 0)}"
            )
            violations = cli_runner_gate.get("violations", [])
            if isinstance(violations, list):
                for violation in violations:
                    if not isinstance(violation, dict):
                        continue
                    profile = str(violation.get("profile") or "")
                    reason = str(violation.get("reason") or "")
                    print(f"- {profile}: {reason}")
        if errors:
            print("-" * 60)
            print("Errors:")
            for err in errors:
                print(f"- {err['tool']}: {err['error']}")
        print("-" * 60)
        print(
            "Snapshot: "
            f"path={snapshot_path} loaded={snapshot_loaded is not None} "
            f"written={snapshot_written} anomalies={len(anomaly_records)}"
        )

    await _close_open_clients_if_loaded()
    return exit_code


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
