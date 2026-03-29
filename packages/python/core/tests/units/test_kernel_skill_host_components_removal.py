from __future__ import annotations

import importlib.util


def test_kernel_skill_host_components_are_removed() -> None:
    assert importlib.util.find_spec("xiuxian_core.kernel.components.skill_loader") is None
    assert importlib.util.find_spec("xiuxian_core.kernel.components.skill_plugin") is None
    assert importlib.util.find_spec("xiuxian_core.kernel.components.registry") is None
    assert importlib.util.find_spec("xiuxian_core.kernel.components.mcp_tool") is None
