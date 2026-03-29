from __future__ import annotations

import importlib
import importlib.util

import pytest


@pytest.mark.parametrize(
    "module_name",
    [
        "xiuxian_core.skills.registry",
        "xiuxian_core.skills.registry.holographic",
        "xiuxian_core.kernel.components.holographic_mcp",
    ],
)
def test_registry_related_modules_removed(module_name: str) -> None:
    with pytest.raises(ModuleNotFoundError):
        importlib.import_module(module_name)


def test_core_skills_module_no_longer_exports_registry_surface() -> None:
    assert importlib.util.find_spec("xiuxian_core.skills") is None
