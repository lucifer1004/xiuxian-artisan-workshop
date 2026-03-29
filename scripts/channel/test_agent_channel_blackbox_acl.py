"""ACL-focused black-box probe regressions for scripts/channel/agent_channel_blackbox.py."""

from __future__ import annotations

import importlib.util
import sys
from typing import TYPE_CHECKING

from xiuxian_foundation.config.prj import get_project_root

if TYPE_CHECKING:
    from pathlib import Path
    from types import ModuleType


def _load_probe_module() -> ModuleType:
    root = get_project_root()
    script_path = root / "scripts" / "channel" / "agent_channel_blackbox.py"
    spec = importlib.util.spec_from_file_location(
        "xiuxian_daochang_channel_blackbox_probe_acl", script_path
    )
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def _make_config(module: ModuleType, log_file: Path, **overrides: object):
    base = {
        "prompt": "/session admin list json",
        "max_wait_secs": 5,
        "max_idle_secs": 5,
        "webhook_url": "http://127.0.0.1:18081/telegram/webhook",
        "log_file": log_file,
        "chat_id": 1001,
        "user_id": 2002,
        "username": None,
        "chat_title": None,
        "thread_id": None,
        "secret_token": None,
        "follow_logs": False,
        "expect_events": (),
        "expect_reply_json_fields": (),
        "expect_log_regexes": (),
        "expect_bot_regexes": (),
        "forbid_log_regexes": (),
        "fail_fast_error_logs": True,
        "allow_no_bot": False,
        "allow_chat_ids": (),
        "session_partition": None,
        "strong_update_id": True,
    }
    base.update(overrides)
    return module.ProbeConfig(**base)


def _sequence_reader(chunks: list[list[str]]):
    state = {"idx": 0}

    def _read_new_lines(_: Path, cursor: int) -> tuple[int, list[str]]:
        if state["idx"] >= len(chunks):
            return cursor, []
        chunk = chunks[state["idx"]]
        state["idx"] += 1
        return cursor + len(chunk), chunk

    return _read_new_lines


def _fake_clock(step: float = 0.1):
    state = {"now": 0.0}

    def _monotonic() -> float:
        state["now"] += step
        return state["now"]

    return _monotonic


def _patch_runtime(monkeypatch, module: ModuleType, update_id: int) -> None:
    monkeypatch.setattr(module.time, "time", lambda: update_id / 1000)
    monkeypatch.setattr(module.os, "getpid", lambda: 42)
    monkeypatch.setattr(module.time, "monotonic", _fake_clock(step=0.1))
    monkeypatch.setattr(module.time, "sleep", lambda _: None)
    monkeypatch.setattr(module, "post_webhook_update", lambda *_: (200, "ok"))


def test_run_probe_allow_no_bot_matches_control_admin_required_event(tmp_path, monkeypatch) -> None:
    module = _load_probe_module()
    log_file = tmp_path / "agent.log"
    log_file.write_text("", encoding="utf-8")
    cfg = _make_config(
        module,
        log_file,
        prompt="/session admin add 1001",
        allow_no_bot=True,
        expect_events=("telegram.command.control_admin_required.replied",),
    )

    update_id = 1_700_000_020_000
    _patch_runtime(monkeypatch, module, update_id)
    monkeypatch.setattr(
        module,
        "read_new_lines",
        _sequence_reader(
            [
                [
                    "2026-02-20 INFO Parsed message, forwarding to agent "
                    "session_key=1001:2002 content_preview=/session admin add 1001"
                ],
                [
                    "2026-02-20 INFO xiuxian_daochang::channels::telegram::runtime::jobs: "
                    "telegram command reply sent "
                    'event="telegram.command.control_admin_required.replied" '
                    'session_key="1001:2002" recipient="1001" '
                    "reply_chars=220 reply_bytes=220"
                ],
            ]
        ),
    )

    assert module.run_probe(cfg) == 0


def test_run_probe_allow_no_bot_matches_slash_permission_required_event(
    tmp_path, monkeypatch
) -> None:
    module = _load_probe_module()
    log_file = tmp_path / "agent.log"
    log_file.write_text("", encoding="utf-8")
    cfg = _make_config(
        module,
        log_file,
        prompt="/session memory",
        allow_no_bot=True,
        expect_events=("telegram.command.slash_permission_required.replied",),
    )

    update_id = 1_700_000_020_100
    _patch_runtime(monkeypatch, module, update_id)
    monkeypatch.setattr(
        module,
        "read_new_lines",
        _sequence_reader(
            [
                [
                    "2026-02-20 INFO Parsed message, forwarding to agent "
                    "session_key=1001:2002 content_preview=/session memory"
                ],
                [
                    "2026-02-20 INFO xiuxian_daochang::channels::telegram::runtime::jobs: "
                    "telegram command reply sent "
                    'event="telegram.command.slash_permission_required.replied" '
                    'session_key="1001:2002" recipient="1001" '
                    "reply_chars=180 reply_bytes=180"
                ],
            ]
        ),
    )

    assert module.run_probe(cfg) == 0


def test_run_probe_acl_event_from_other_recipient_does_not_satisfy_expectation(
    tmp_path, monkeypatch
) -> None:
    module = _load_probe_module()
    log_file = tmp_path / "agent.log"
    log_file.write_text("", encoding="utf-8")
    cfg = _make_config(
        module,
        log_file,
        prompt="/session memory",
        expect_events=("telegram.command.slash_permission_required.replied",),
    )

    update_id = 1_700_000_020_200
    _patch_runtime(monkeypatch, module, update_id)
    monkeypatch.setattr(
        module,
        "read_new_lines",
        _sequence_reader(
            [
                [
                    "2026-02-20 INFO Parsed message, forwarding to agent "
                    "session_key=1001:2002 content_preview=/session memory"
                ],
                ['2026-02-20 INFO → Bot: "permission denied"'],
                [
                    "2026-02-20 INFO xiuxian_daochang::channels::telegram::runtime::jobs: "
                    "telegram command reply sent "
                    'event="telegram.command.slash_permission_required.replied" '
                    'session_key="9999:2002" recipient="9999" '
                    "reply_chars=180 reply_bytes=180"
                ],
            ]
        ),
    )

    assert module.run_probe(cfg) == 8


def test_run_probe_acl_event_with_mismatched_session_key_fails_scope_validation(
    tmp_path, monkeypatch
) -> None:
    module = _load_probe_module()
    log_file = tmp_path / "agent.log"
    log_file.write_text("", encoding="utf-8")
    cfg = _make_config(
        module,
        log_file,
        prompt="/session admin add 1001",
        allow_no_bot=True,
        expect_events=("telegram.command.control_admin_required.replied",),
    )

    update_id = 1_700_000_020_300
    _patch_runtime(monkeypatch, module, update_id)
    monkeypatch.setattr(
        module,
        "read_new_lines",
        _sequence_reader(
            [
                [
                    "2026-02-20 INFO Parsed message, forwarding to agent "
                    "session_key=1001:2002 content_preview=/session admin add 1001"
                ],
                [
                    "2026-02-20 INFO xiuxian_daochang::channels::telegram::runtime::jobs: "
                    "telegram command reply sent "
                    'event="telegram.command.control_admin_required.replied" '
                    'session_key="1001:9999" recipient="1001" '
                    "reply_chars=220 reply_bytes=220"
                ],
            ]
        ),
    )

    assert module.run_probe(cfg) == 10
