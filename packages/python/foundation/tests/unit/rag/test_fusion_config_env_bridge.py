"""Tests for fusion config bridge that injects KG Valkey URL from settings."""

from __future__ import annotations

import os
import sys
import types
from unittest.mock import MagicMock

from omni.rag.fusion import _config as fusion_config


def test_ensure_graph_valkey_env_prefers_explicit_graph_env(monkeypatch) -> None:
    monkeypatch.setenv(fusion_config.GRAPH_VALKEY_URL_ENV, "redis://127.0.0.1:6380/0")
    monkeypatch.delenv("VALKEY_URL", raising=False)
    resolver = MagicMock(return_value="redis://127.0.0.1:6390/0")
    monkeypatch.setattr(fusion_config, "_resolve_graph_valkey_url_from_runtime_config", resolver)

    fusion_config._ensure_graph_valkey_env()

    assert os.environ[fusion_config.GRAPH_VALKEY_URL_ENV] == "redis://127.0.0.1:6380/0"
    resolver.assert_not_called()


def test_ensure_graph_valkey_env_uses_valkey_url_fallback(monkeypatch) -> None:
    monkeypatch.delenv(fusion_config.GRAPH_VALKEY_URL_ENV, raising=False)
    monkeypatch.setenv("VALKEY_URL", "redis://127.0.0.1:6381/0")
    resolver = MagicMock(return_value="redis://127.0.0.1:6391/0")
    monkeypatch.setattr(fusion_config, "_resolve_graph_valkey_url_from_runtime_config", resolver)

    fusion_config._ensure_graph_valkey_env()

    assert os.environ["VALKEY_URL"] == "redis://127.0.0.1:6381/0"
    assert not os.getenv(fusion_config.GRAPH_VALKEY_URL_ENV, "").strip()
    resolver.assert_not_called()


def test_ensure_graph_valkey_env_injects_from_runtime_config(monkeypatch) -> None:
    monkeypatch.delenv(fusion_config.GRAPH_VALKEY_URL_ENV, raising=False)
    monkeypatch.delenv("VALKEY_URL", raising=False)
    monkeypatch.setattr(
        fusion_config,
        "_resolve_graph_valkey_url_from_runtime_config",
        lambda: "redis://127.0.0.1:6392/0",
    )

    fusion_config._ensure_graph_valkey_env()

    assert os.environ[fusion_config.GRAPH_VALKEY_URL_ENV] == "redis://127.0.0.1:6392/0"


def test_load_kg_injects_graph_valkey_env_before_rust_call(monkeypatch) -> None:
    monkeypatch.delenv(fusion_config.GRAPH_VALKEY_URL_ENV, raising=False)
    monkeypatch.delenv("VALKEY_URL", raising=False)
    monkeypatch.setattr(
        fusion_config,
        "_resolve_graph_valkey_url_from_runtime_config",
        lambda: "redis://127.0.0.1:6393/0",
    )

    mock_load = MagicMock(return_value=None)
    module = types.ModuleType("xiuxian_core_rs")
    module.load_kg_from_valkey_cached = mock_load
    monkeypatch.setitem(sys.modules, "xiuxian_core_rs", module)

    out = fusion_config._load_kg(scope_key="test.scope")

    assert out is None
    assert os.environ[fusion_config.GRAPH_VALKEY_URL_ENV] == "redis://127.0.0.1:6393/0"
    mock_load.assert_called_once_with("test.scope")


def test_save_kg_injects_graph_valkey_env_before_rust_call(monkeypatch) -> None:
    monkeypatch.delenv(fusion_config.GRAPH_VALKEY_URL_ENV, raising=False)
    monkeypatch.delenv("VALKEY_URL", raising=False)
    monkeypatch.setattr(
        fusion_config,
        "_resolve_graph_valkey_url_from_runtime_config",
        lambda: "redis://127.0.0.1:6394/0",
    )

    class _FakeKg:
        saved_scope: str | None = None

        def save_to_valkey(self, scope_key: str) -> None:
            self.saved_scope = scope_key

    fake = _FakeKg()
    fusion_config._save_kg(fake, scope_key="save.scope")

    assert os.environ[fusion_config.GRAPH_VALKEY_URL_ENV] == "redis://127.0.0.1:6394/0"
    assert fake.saved_scope == "save.scope"
