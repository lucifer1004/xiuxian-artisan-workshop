from __future__ import annotations

import importlib

import pytest


def test_skill_extensions_package_is_deleted() -> None:
    for module_name in (
        "xiuxian_core.skills.extensions",
        "xiuxian_core.skills.extensions.loader",
        "xiuxian_core.skills.extensions.wrapper",
        "xiuxian_core.skills.extensions.fixtures",
        "xiuxian_core.skills.extensions.directory_loader",
        "xiuxian_core.skills.extensions.rust_bridge",
        "xiuxian_core.skills.extensions.sniffer",
    ):
        with pytest.raises(ModuleNotFoundError):
            importlib.import_module(module_name)
