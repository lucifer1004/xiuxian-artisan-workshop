"""
Command execution lifecycle hooks.

Invoked by the CLI/runtime shell before and after each command-script execution.
Used to set the embedding override so command execution can prefer the retained
runtime embedding path when running from CLI.
"""

from __future__ import annotations

from collections.abc import Callable

_before_command_execute: list[Callable[[], None]] = []
_after_command_execute: list[Callable[[], None]] = []


def register_before_command_execute(cb: Callable[[], None]) -> None:
    """Register a callback to run before each command-script execution."""
    _before_command_execute.append(cb)


def register_after_command_execute(cb: Callable[[], None]) -> None:
    """Register a callback to run after each command-script execution."""
    _after_command_execute.append(cb)


def run_before_command_execute() -> None:
    """Run all registered before-execute callbacks (e.g. set embedding override)."""
    for cb in _before_command_execute:
        try:
            cb()
        except Exception:
            pass


def run_after_command_execute() -> None:
    """Run all registered after-execute callbacks (e.g. clear embedding override)."""
    for cb in _after_command_execute:
        try:
            cb()
        except Exception:
            pass


__all__ = [
    "register_after_command_execute",
    "register_before_command_execute",
    "run_after_command_execute",
    "run_before_command_execute",
]
