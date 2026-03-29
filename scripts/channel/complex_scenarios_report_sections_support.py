#!/usr/bin/env python3
"""Support helpers for complex scenario markdown report sections."""

from __future__ import annotations


def coerce_positive_int(value: object) -> int | None:
    """Parse positive integer-like values, returning None for invalid/zero."""
    if isinstance(value, bool):
        return None
    if isinstance(value, int):
        parsed = value
    elif isinstance(value, float):
        parsed = int(value)
    elif isinstance(value, str):
        try:
            parsed = int(value)
        except ValueError:
            return None
    else:
        return None
    return parsed if parsed > 0 else None


def format_tool_event_counts(counts: object) -> str:
    """Render tool-runtime event count map as deterministic compact text."""
    if not isinstance(counts, dict):
        return "-"

    pairs: list[tuple[str, int]] = []
    for key, value in counts.items():
        if not isinstance(key, str):
            continue
        parsed = coerce_positive_int(value)
        if parsed is None:
            continue
        pairs.append((key, parsed))
    if not pairs:
        return "-"

    pairs.sort(key=lambda item: item[0])
    return ",".join(f"{key}:{value}" for key, value in pairs)


def behavioral_evidence_summary(scenario: dict[str, object]) -> str:
    """Build one-line behavioral evidence summary."""
    steps = scenario["steps"]
    natural_language_steps = [
        step for step in steps if not str(step["prompt"]).strip().startswith("/")
    ]
    with_bot_excerpt = [step for step in steps if step.get("bot_excerpt")]
    planned_hits = sum(1 for step in steps if step.get("memory_planned_seen"))
    injected_hits = sum(1 for step in steps if step.get("memory_injected_seen"))
    skipped_hits = sum(1 for step in steps if step.get("memory_skipped_seen"))
    feedback_hits = sum(1 for step in steps if step.get("memory_feedback_updated_seen"))
    recall_credit_steps = sum(1 for step in steps if step.get("memory_recall_credit_seen"))
    decay_steps = sum(1 for step in steps if step.get("memory_decay_seen"))
    recall_credit_events = sum(int(step.get("memory_recall_credit_count") or 0) for step in steps)
    decay_events = sum(int(step.get("memory_decay_count") or 0) for step in steps)
    tool_waiting_steps = sum(1 for step in steps if step.get("tool_waiting_seen"))

    tool_waiting_events = 0
    for step in steps:
        counts = step.get("tool_event_counts")
        if not isinstance(counts, dict):
            continue
        tool_waiting_events += int(counts.get("tool_runtime.pool.connect.waiting", 0) or 0)
        tool_waiting_events += int(counts.get("tool_runtime.pool.call.waiting", 0) or 0)

    return (
        f"natural_language_steps={len(natural_language_steps)}, "
        f"steps_with_bot_excerpt={len(with_bot_excerpt)}, "
        f"planned_hits={planned_hits}, injected_hits={injected_hits}, "
        f"skipped_hits={skipped_hits}, feedback_updated_hits={feedback_hits}, "
        f"recall_credit_steps={recall_credit_steps}, decay_steps={decay_steps}, "
        f"recall_credit_events={recall_credit_events}, decay_events={decay_events}, "
        f"tool_waiting_steps={tool_waiting_steps}, tool_waiting_events={tool_waiting_events}"
    )
