"""Tests for LinkGraph backend factory cache behavior."""

from __future__ import annotations

from typing import TYPE_CHECKING

import pytest

from omni.rag.link_graph.factory import get_link_graph_backend, reset_link_graph_backend_cache

if TYPE_CHECKING:
    from pathlib import Path

GATEWAY_URL_A = "http://gateway-a"
GATEWAY_URL_B = "http://gateway-b"


@pytest.fixture(autouse=True)
def _reset_cache() -> None:
    reset_link_graph_backend_cache()
    yield
    reset_link_graph_backend_cache()


def test_get_link_graph_backend_reuses_cached_instance(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
) -> None:
    import omni.rag.link_graph.factory as factory_module

    monkeypatch.setattr(factory_module, "get_setting", lambda _k, default=None: default)
    monkeypatch.setattr(factory_module, "_resolve_backend_name", lambda name=None: "wendao")
    monkeypatch.setattr(
        factory_module, "_resolve_notebook_dir", lambda notebook_dir=None: str(tmp_path)
    )

    build_calls = {"count": 0}

    class _FakeBackend:
        backend_name = "wendao"

    def _fake_build(_backend_name: str, _notebook_dir: str | None, _gateway_base_url: str | None):
        build_calls["count"] += 1
        return _FakeBackend()

    monkeypatch.setattr(factory_module, "_build_backend", _fake_build)

    first = get_link_graph_backend()
    second = get_link_graph_backend()

    assert first is second
    assert build_calls["count"] == 1


def test_get_link_graph_backend_cache_key_includes_notebook_dir(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
) -> None:
    import omni.rag.link_graph.factory as factory_module

    monkeypatch.setattr(factory_module, "get_setting", lambda _k, default=None: default)
    monkeypatch.setattr(factory_module, "_resolve_backend_name", lambda name=None: "wendao")

    calls = {"count": 0}

    class _FakeBackend:
        backend_name = "wendao"

    def _fake_build(_backend_name: str, _notebook_dir: str | None, _gateway_base_url: str | None):
        calls["count"] += 1
        return _FakeBackend()

    monkeypatch.setattr(factory_module, "_build_backend", _fake_build)

    a = get_link_graph_backend(notebook_dir=tmp_path / "a")
    b = get_link_graph_backend(notebook_dir=tmp_path / "b")

    assert a is not b
    assert calls["count"] == 2


def test_get_link_graph_backend_cache_key_includes_gateway_url(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    import omni.rag.link_graph.factory as factory_module

    monkeypatch.setattr(factory_module, "get_setting", lambda _k, default=None: default)
    monkeypatch.setattr(factory_module, "_resolve_backend_name", lambda name=None: "gateway")
    monkeypatch.setattr(factory_module, "_resolve_notebook_dir", lambda notebook_dir=None: None)
    monkeypatch.setattr(factory_module, "_resolve_gateway_base_url", lambda base_url=None: base_url)

    calls = {"count": 0}

    class _FakeBackend:
        backend_name = "gateway"

        def __init__(self, base_url: str | None) -> None:
            self.base_url = base_url

    def _fake_build(_backend_name: str, _notebook_dir: str | None, _gateway_base_url: str | None):
        calls["count"] += 1
        return _FakeBackend(_gateway_base_url)

    monkeypatch.setattr(factory_module, "_build_backend", _fake_build)

    first = get_link_graph_backend(gateway_base_url=GATEWAY_URL_A)
    second = get_link_graph_backend(gateway_base_url=GATEWAY_URL_B)

    assert first is not second
    assert calls["count"] == 2


def test_get_link_graph_backend_can_bypass_cache(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
) -> None:
    import omni.rag.link_graph.factory as factory_module

    monkeypatch.setattr(factory_module, "get_setting", lambda _k, default=None: default)
    monkeypatch.setattr(factory_module, "_resolve_backend_name", lambda name=None: "wendao")
    monkeypatch.setattr(
        factory_module, "_resolve_notebook_dir", lambda notebook_dir=None: str(tmp_path)
    )

    calls = {"count": 0}

    class _FakeBackend:
        backend_name = "wendao"

    def _fake_build(_backend_name: str, _notebook_dir: str | None, _gateway_base_url: str | None):
        calls["count"] += 1
        return _FakeBackend()

    monkeypatch.setattr(factory_module, "_build_backend", _fake_build)

    first = get_link_graph_backend(use_cache=False)
    second = get_link_graph_backend(use_cache=False)

    assert first is not second
    assert calls["count"] == 2


def test_reset_link_graph_backend_cache_forces_rebuild(
    monkeypatch: pytest.MonkeyPatch,
    tmp_path: Path,
) -> None:
    import omni.rag.link_graph.factory as factory_module

    monkeypatch.setattr(factory_module, "get_setting", lambda _k, default=None: default)
    monkeypatch.setattr(factory_module, "_resolve_backend_name", lambda name=None: "wendao")
    monkeypatch.setattr(
        factory_module, "_resolve_notebook_dir", lambda notebook_dir=None: str(tmp_path)
    )

    calls = {"count": 0}

    class _FakeBackend:
        backend_name = "wendao"

    def _fake_build(_backend_name: str, _notebook_dir: str | None, _gateway_base_url: str | None):
        calls["count"] += 1
        return _FakeBackend()

    monkeypatch.setattr(factory_module, "_build_backend", _fake_build)

    first = get_link_graph_backend()
    reset_link_graph_backend_cache()
    second = get_link_graph_backend()

    assert first is not second
    assert calls["count"] == 2


def test_resolve_backend_name_rejects_unknown_backend() -> None:
    import omni.rag.link_graph.factory as factory_module

    with pytest.raises(ValueError, match="Unsupported link_graph backend"):
        factory_module._resolve_backend_name("legacy")


def test_get_link_graph_backend_rejects_unknown_backend_setting(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    import omni.rag.link_graph.factory as factory_module

    monkeypatch.setattr(factory_module, "get_setting", lambda _k, default=None: "legacy")
    with pytest.raises(ValueError, match="Unsupported link_graph backend"):
        get_link_graph_backend(use_cache=False)
