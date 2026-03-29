#!/usr/bin/env python3
"""Constants for agent channel blackbox probes."""

from __future__ import annotations

ERROR_PATTERNS = (
    "Telegram sendMessage failed",
    "Failed to send",
    "Foreground message handling failed",
    "tools/call: Tool runtime error",
)

TOOL_OBSERVABILITY_EVENTS = (
    "tool_runtime.pool.connect.attempt",
    "tool_runtime.pool.connect.waiting",
    "tool_runtime.pool.connect.failed",
    "tool_runtime.pool.connect.succeeded",
    "tool_runtime.pool.health.wait.start",
    "tool_runtime.pool.health.wait.ready",
    "tool_runtime.pool.health.wait.timeout",
    "tool_runtime.pool.call.waiting",
    "tool_runtime.pool.call.slow",
)

TOOL_WAITING_EVENTS = frozenset(
    {"tool_runtime.pool.connect.waiting", "tool_runtime.pool.call.waiting"}
)
TARGET_SESSION_SCOPE_PLACEHOLDER = "__target_session_scope__"
TELEGRAM_SESSION_SCOPE_PREFIX = "telegram:"
DISCORD_SESSION_SCOPE_PREFIX = "discord:"
