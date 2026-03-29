from __future__ import annotations

import importlib.util


def test_removed_skill_host_modules_are_absent() -> None:
    removed_modules = (
        "xiuxian_core.skills.discovery",
        "xiuxian_core.skills.index_loader",
        "xiuxian_core.skills.memory",
        "xiuxian_core.skills.runner",
        "xiuxian_core.skills.runtime",
        "xiuxian_core.skills.schema_gen",
        "xiuxian_core.skills.tools_loader",
        "xiuxian_core.skills.universal",
        "xiuxian_core.services.skill_manager",
    )

    for module_name in removed_modules:
        try:
            spec = importlib.util.find_spec(module_name)
        except ModuleNotFoundError:
            spec = None
        assert spec is None
