from __future__ import annotations

import importlib.util

import xiuxian_test_kit as test_kit


def test_test_kit_skill_module_is_removed() -> None:
    assert importlib.util.find_spec("xiuxian_test_kit.core") is None
    assert importlib.util.find_spec("xiuxian_test_kit.skill") is None
    assert importlib.util.find_spec("xiuxian_test_kit.mcp") is None
    assert importlib.util.find_spec("xiuxian_test_kit.mcp_tools_list_snapshot") is None
    assert importlib.util.find_spec("xiuxian_test_kit.fixtures.skill_builder") is None


def test_test_kit_root_no_longer_exports_skill_command_tester() -> None:
    assert not hasattr(test_kit, "SkillCommandTester")
