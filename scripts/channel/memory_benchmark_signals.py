#!/usr/bin/env python3
"""Log signal parsing helpers for omni-agent memory benchmark scripts."""

from __future__ import annotations

import re
from typing import Any

ANSI_ESCAPE_RE = re.compile(r"\x1b\[[0-9;]*m")
EVENT_TOKEN_RE = re.compile(r"\bevent\s*=\s*(?:\"|')?([A-Za-z0-9_.:-]+)")
LOG_TOKEN_RE = re.compile(r"\b([A-Za-z0-9_.:-]+)\s*=\s*(?:\"([^\"]*)\"|'([^']*)'|([^\s]+))")


def strip_ansi(value: str) -> str:
    """Strip ANSI color/control sequences from a log line."""
    return ANSI_ESCAPE_RE.sub("", value)


def extract_event_token(value: str) -> str | None:
    """Extract structured `event=...` token from a log line."""
    match = EVENT_TOKEN_RE.search(value)
    return match.group(1) if match else None


def has_event(lines: list[str], event: str) -> bool:
    """Return whether any log line contains the target event token."""
    return any(extract_event_token(line) == event for line in lines)


def parse_log_tokens(value: str) -> dict[str, str]:
    """Parse key=value pairs from one log line."""
    normalized = strip_ansi(value)
    tokens: dict[str, str] = {}
    for match in LOG_TOKEN_RE.finditer(normalized):
        key = match.group(1)
        token = match.group(2) or match.group(3) or match.group(4) or ""
        tokens[key] = token
    return tokens


def token_as_int(tokens: dict[str, str], key: str) -> int | None:
    """Read an integer token from parsed log tokens."""
    raw = tokens.get(key)
    if raw is None:
        return None
    try:
        return int(raw)
    except ValueError:
        return None


def token_as_float(tokens: dict[str, str], key: str) -> float | None:
    """Read a float token from parsed log tokens."""
    raw = tokens.get(key)
    if raw is None:
        return None
    try:
        return float(raw)
    except ValueError:
        return None


def trim_text(value: str | None, *, max_chars: int = 280) -> str | None:
    """Trim text for compact report rendering."""
    if value is None:
        return None
    if len(value) <= max_chars:
        return value
    return value[: max_chars - 3] + "..."


def parse_turn_signals(
    lines: list[str],
    *,
    forbidden_log_pattern: str,
    bot_marker: str,
    recall_plan_event: str,
    recall_injected_event: str,
    recall_skipped_event: str,
    recall_feedback_event: str,
    embedding_timeout_fallback_event: str,
    embedding_cooldown_fallback_event: str,
    embedding_unavailable_fallback_event: str,
) -> dict[str, Any]:
    """Parse benchmark-relevant signals from runtime log lines."""
    signals: dict[str, Any] = {
        "plan": None,
        "decision": None,
        "feedback": None,
        "bot_line": None,
        "mcp_error": False,
        "embedding_timeout_fallback": False,
        "embedding_cooldown_fallback": False,
        "embedding_unavailable_fallback": False,
    }

    for line in lines:
        if forbidden_log_pattern in line:
            signals["mcp_error"] = True
        if bot_marker in line:
            signals["bot_line"] = line.split(bot_marker, 1)[1].strip()

        event = extract_event_token(line)
        if event is None:
            continue

        tokens = parse_log_tokens(line)
        if event == recall_plan_event:
            signals["plan"] = tokens
        elif event in (recall_injected_event, recall_skipped_event):
            signals["decision"] = {**tokens, "event": event}
        elif event == recall_feedback_event:
            signals["feedback"] = tokens
        elif event == embedding_timeout_fallback_event:
            signals["embedding_timeout_fallback"] = True
        elif event == embedding_cooldown_fallback_event:
            signals["embedding_cooldown_fallback"] = True
        elif event == embedding_unavailable_fallback_event:
            signals["embedding_unavailable_fallback"] = True

    return signals
