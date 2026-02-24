from __future__ import annotations

import importlib.util
import sys
from pathlib import Path


def _load_module():
    script_path = Path(__file__).resolve().with_name("resolve_libpython_path.py")
    module_name = "test_resolve_libpython_path_module"
    spec = importlib.util.spec_from_file_location(module_name, script_path)
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


def test_resolve_libpython_path_returns_joined_path(monkeypatch) -> None:
    module = _load_module()

    def _fake_get_config_var(key: str):
        if key == "LIBDIR":
            return "/usr/lib"
        if key == "LDLIBRARY":
            return "libpython3.13.dylib"
        return None

    monkeypatch.setattr(module.sysconfig, "get_config_var", _fake_get_config_var)
    assert module.resolve_libpython_path() == "/usr/lib/libpython3.13.dylib"


def test_resolve_libpython_path_returns_empty_when_missing(monkeypatch) -> None:
    module = _load_module()

    def _fake_get_config_var(_key: str):
        return None

    monkeypatch.setattr(module.sysconfig, "get_config_var", _fake_get_config_var)
    assert module.resolve_libpython_path() == ""
