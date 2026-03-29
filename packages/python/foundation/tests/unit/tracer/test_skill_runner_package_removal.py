from __future__ import annotations

import importlib.util


def test_skill_runner_module_is_absent() -> None:
    assert importlib.util.find_spec("xiuxian_tracer.skill_runner") is None
