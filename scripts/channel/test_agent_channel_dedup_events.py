"""Tests for scripts/channel/test_xiuxian_daochang_dedup_events.py."""

from __future__ import annotations

import argparse
import importlib.util
import sys
from typing import TYPE_CHECKING

from xiuxian_foundation.config.prj import get_project_root

if TYPE_CHECKING:
    from types import ModuleType


def _load_dedup_module() -> ModuleType:
    root = get_project_root()
    script_path = root / "scripts" / "channel" / "test_xiuxian_daochang_dedup_events.py"
    spec = importlib.util.spec_from_file_location(
        "xiuxian_daochang_channel_dedup_probe", script_path
    )
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def _make_args(**overrides: object) -> argparse.Namespace:
    defaults: dict[str, object] = {
        "max_wait": 25,
        "webhook_url": "http://127.0.0.1:18081/telegram/webhook",
        "log_file": ".run/logs/xiuxian-daochang-webhook.log",
        "chat_id": 1001,
        "user_id": 2002,
        "username": "tao3k",
        "thread_id": None,
        "secret_token": None,
        "text": "/session json",
    }
    defaults.update(overrides)
    return argparse.Namespace(**defaults)


def test_build_config_uses_resolver_secret_when_cli_secret_missing(monkeypatch) -> None:
    module = _load_dedup_module()
    monkeypatch.setattr(module, "telegram_webhook_secret_token", lambda: "resolver-secret")
    monkeypatch.setattr(module, "username_from_settings", lambda: None)
    monkeypatch.setattr(module, "username_from_runtime_log", lambda *_: "tao3k")

    cfg = module.build_config(_make_args(secret_token=None, username=None))
    assert cfg.secret_token == "resolver-secret"


def test_build_config_prefers_explicit_secret_over_resolver(monkeypatch) -> None:
    module = _load_dedup_module()
    monkeypatch.setattr(module, "telegram_webhook_secret_token", lambda: "resolver-secret")
    monkeypatch.setattr(module, "username_from_settings", lambda: None)
    monkeypatch.setattr(module, "username_from_runtime_log", lambda *_: "tao3k")

    cfg = module.build_config(_make_args(secret_token="explicit-secret", username=None))
    assert cfg.secret_token == "explicit-secret"


def test_read_new_lines_returns_cursor_and_lines(monkeypatch) -> None:
    module = _load_dedup_module()

    def _fake_read_new(_path: object, _cursor: object) -> tuple[object, list[str]]:
        return module._SharedLogCursor(kind="offset", value=31), ["dedup-a", "dedup-b"]

    monkeypatch.setattr(module, "_shared_read_new_log_lines_with_cursor", _fake_read_new)
    cursor, lines = module.read_new_lines(get_project_root() / ".run" / "dummy.log", 7)
    assert cursor == 31
    assert lines == ["dedup-a", "dedup-b"]


def test_count_lines_returns_offset_cursor(monkeypatch) -> None:
    module = _load_dedup_module()

    def _fake_init_cursor(_path: object, kind: str) -> object:
        assert kind == "offset"
        return module._SharedLogCursor(kind="offset", value=47)

    monkeypatch.setattr(module, "_shared_init_log_cursor", _fake_init_cursor)
    assert module.count_lines(get_project_root() / ".run" / "dummy.log") == 47
