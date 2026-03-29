#!/usr/bin/env python3
"""Tool-runtime event tracking and diagnostics rendering for blackbox probes."""

from __future__ import annotations

import json
from typing import Any


def record_tool_event(
    state: Any,
    *,
    event_token: str,
    tool_observability_events: tuple[str, ...],
    tool_waiting_events: frozenset[str],
) -> None:
    """Track tool-runtime observability counters for matched event token."""
    if event_token in tool_observability_events:
        state.tool_event_counts[event_token] += 1
        state.tool_last_event = event_token
        if event_token in tool_waiting_events:
            state.tool_waiting_seen = True


def emit_tool_diagnostics(
    state: Any,
    *,
    tool_observability_events: tuple[str, ...],
) -> None:
    """Print tool-runtime diagnostics snapshot."""
    counts_payload = {
        event: state.tool_event_counts[event]
        for event in tool_observability_events
        if state.tool_event_counts[event] > 0
    }
    print("Tool runtime diagnostics:")
    print(f"  tool_last_event={state.tool_last_event or ''}")
    print(f"  tool_waiting_seen={'true' if state.tool_waiting_seen else 'false'}")
    print(
        "  tool_event_counts="
        + json.dumps(counts_payload, ensure_ascii=True, separators=(",", ":"), sort_keys=True)
    )
