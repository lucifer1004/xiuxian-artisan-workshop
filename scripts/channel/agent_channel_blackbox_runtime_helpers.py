#!/usr/bin/env python3
"""Shared runtime helper functions for channel blackbox probes."""

from __future__ import annotations

import importlib

from agent_channel_blackbox_runtime_helpers_core import (
    ProbeRuntimeState,
    build_probe_runtime_state,
    print_probe_intro,
)

_tool_module = importlib.import_module("agent_channel_blackbox_runtime_tools")
_expectation_module = importlib.import_module("agent_channel_blackbox_runtime_expectations")

all_expectations_satisfied = _expectation_module.all_expectations_satisfied
missing_expectations = _expectation_module.missing_expectations
latest_json_summary_for_event = _expectation_module.latest_json_summary_for_event
recipient_matches_target = _expectation_module.recipient_matches_target
observation_matches_target_recipient = _expectation_module.observation_matches_target_recipient
observation_matches_target_scope = _expectation_module.observation_matches_target_scope
event_matches_expectations = _expectation_module.event_matches_expectations
event_line_matches_target_recipient = _expectation_module.event_line_matches_target_recipient
reply_json_field_matches = _expectation_module.reply_json_field_matches
mark_event_expectation = _expectation_module.mark_event_expectation
mark_reply_json_expectation = _expectation_module.mark_reply_json_expectation
mark_expect_log_patterns = _expectation_module.mark_expect_log_patterns
mark_expect_bot_patterns = _expectation_module.mark_expect_bot_patterns
pick_target_command_reply_observation = _expectation_module.pick_target_command_reply_observation
pick_target_json_summary_observation = _expectation_module.pick_target_json_summary_observation
validate_target_session_scope = _expectation_module.validate_target_session_scope


def record_tool_event(
    state: ProbeRuntimeState,
    *,
    event_token: str,
    tool_observability_events: tuple[str, ...],
    tool_waiting_events: frozenset[str],
) -> None:
    _tool_module.record_tool_event(
        state,
        event_token=event_token,
        tool_observability_events=tool_observability_events,
        tool_waiting_events=tool_waiting_events,
    )


def emit_tool_diagnostics(
    state: ProbeRuntimeState,
    *,
    tool_observability_events: tuple[str, ...],
) -> None:
    _tool_module.emit_tool_diagnostics(
        state,
        tool_observability_events=tool_observability_events,
    )


__all__ = [
    "ProbeRuntimeState",
    "all_expectations_satisfied",
    "build_probe_runtime_state",
    "emit_tool_diagnostics",
    "event_line_matches_target_recipient",
    "event_matches_expectations",
    "latest_json_summary_for_event",
    "mark_event_expectation",
    "mark_expect_bot_patterns",
    "mark_expect_log_patterns",
    "mark_reply_json_expectation",
    "missing_expectations",
    "observation_matches_target_recipient",
    "observation_matches_target_scope",
    "pick_target_command_reply_observation",
    "pick_target_json_summary_observation",
    "print_probe_intro",
    "recipient_matches_target",
    "record_tool_event",
    "reply_json_field_matches",
    "validate_target_session_scope",
]
