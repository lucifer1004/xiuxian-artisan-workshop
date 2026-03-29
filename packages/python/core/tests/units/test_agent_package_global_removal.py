from __future__ import annotations

import importlib.util


def test_python_agent_package_is_absent() -> None:
    assert importlib.util.find_spec("omni.agent") is None
