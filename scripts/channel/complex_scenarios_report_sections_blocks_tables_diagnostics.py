#!/usr/bin/env python3
"""Diagnostics table builders for complex scenario report sections."""

from __future__ import annotations

from complex_scenarios_report_sections_support import format_tool_event_counts


def append_tool_diagnostics(lines: list[str], scenario: dict[str, object]) -> None:
    """Append tool-runtime diagnostics table."""
    lines.extend(
        [
            "",
            "Tool runtime diagnostics:",
            "",
            "| Step | tool_last_event | waiting_seen | tool_event_counts |",
            "|---|---|---|---|",
        ]
    )
    for step in scenario["steps"]:
        tool_last_event = str(step.get("tool_last_event") or "-")
        waiting_seen = "true" if step.get("tool_waiting_seen") else "false"
        counts_text = format_tool_event_counts(step.get("tool_event_counts"))
        lines.append(
            "| `{sid}` | `{last}` | {waiting} | `{counts}` |".format(
                sid=step["step_id"],
                last=tool_last_event.replace("|", "\\|"),
                waiting=waiting_seen,
                counts=counts_text.replace("|", "\\|"),
            )
        )


def append_failure_tails(lines: list[str], scenario: dict[str, object]) -> None:
    """Append stderr/stdout tails for failed steps."""
    failure_steps = [
        step for step in scenario["steps"] if not step["passed"] and not step["skipped"]
    ]
    if not failure_steps:
        return

    lines.append("")
    lines.append("Failure tails:")
    for step in failure_steps:
        lines.extend(
            [
                f"- `{step['step_id']}`",
                "```text",
                step["stderr_tail"] or step["stdout_tail"] or "(no output)",
                "```",
            ]
        )
