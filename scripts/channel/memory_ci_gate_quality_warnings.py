#!/usr/bin/env python3
"""Warning-budget quality gates for memory CI runtime logs."""

from __future__ import annotations

from typing import Any


def assert_tool_waiting_warning_budget(cfg: Any, *, count_log_event_fn: Any) -> None:
    """Validate warning budgets for tool-runtime waiting events."""
    if not cfg.runtime_log_file.exists():
        raise RuntimeError(f"missing runtime log file: {cfg.runtime_log_file}")

    call_waiting = count_log_event_fn(cfg.runtime_log_file, "tool_runtime.pool.call.waiting")
    connect_waiting = count_log_event_fn(cfg.runtime_log_file, "tool_runtime.pool.connect.waiting")
    waiting_total = call_waiting + connect_waiting

    failures: list[str] = []
    if call_waiting > cfg.max_tool_call_waiting_events:
        failures.append(
            f"tool_runtime.pool.call.waiting={call_waiting} > {cfg.max_tool_call_waiting_events}"
        )
    if connect_waiting > cfg.max_tool_connect_waiting_events:
        failures.append(
            f"tool_runtime.pool.connect.waiting={connect_waiting} > {cfg.max_tool_connect_waiting_events}"
        )
    if waiting_total > cfg.max_tool_waiting_events_total:
        failures.append(
            f"tool_waiting_events_total={waiting_total} > {cfg.max_tool_waiting_events_total}"
        )

    if failures:
        raise RuntimeError("tool waiting warning budget exceeded: " + "; ".join(failures))

    print(
        "Tool waiting warning budget passed: "
        f"call_waiting={call_waiting}, "
        f"connect_waiting={connect_waiting}, "
        f"total={waiting_total}",
        flush=True,
    )


def assert_memory_stream_warning_budget(cfg: Any, *, count_log_event_fn: Any) -> None:
    """Validate warning budgets for memory stream read failures."""
    if not cfg.runtime_log_file.exists():
        raise RuntimeError(f"missing runtime log file: {cfg.runtime_log_file}")

    read_failed = count_log_event_fn(
        cfg.runtime_log_file, "agent.memory.stream_consumer.read_failed"
    )
    if read_failed > cfg.max_memory_stream_read_failed_events:
        raise RuntimeError(
            "memory stream warning budget exceeded: "
            f"agent.memory.stream_consumer.read_failed={read_failed} > "
            f"{cfg.max_memory_stream_read_failed_events}"
        )

    print(
        "Memory stream warning budget passed: "
        f"agent.memory.stream_consumer.read_failed={read_failed}",
        flush=True,
    )
