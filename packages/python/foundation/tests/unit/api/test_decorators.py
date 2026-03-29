"""Removal guards for deleted script metadata assumptions."""

from __future__ import annotations

import importlib


def test_skill_discovery_module_no_longer_relies_on_command_metadata() -> None:
    decorators = importlib.import_module("xiuxian_foundation.api.decorators")
    assert not hasattr(decorators, "tool_command")
    assert not hasattr(decorators, "get_command_metadata")
