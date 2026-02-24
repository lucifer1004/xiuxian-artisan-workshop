"""Discord ACL black-box probe regressions for scripts/channel/agent_channel_blackbox.py."""

from __future__ import annotations

import importlib.util
import sys
from typing import TYPE_CHECKING

from omni.foundation.runtime.gitops import get_project_root

if TYPE_CHECKING:
    from pathlib import Path
    from types import ModuleType


def _load_probe_module() -> ModuleType:
    root = get_project_root()
    script_path = root / "scripts" / "channel" / "agent_channel_blackbox.py"
    spec = importlib.util.spec_from_file_location(
        "omni_agent_channel_blackbox_probe_discord_acl", script_path
    )
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def _make_config(module: ModuleType, log_file: Path, **overrides: object):
    base = {
        "prompt": "/session admin add 1001",
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


def test_parse_command_reply_event_line_accepts_discord_log() -> None:
    module = _load_probe_module()
    line = (
        "2026-02-21 INFO omni_agent::channels::discord::runtime::managed::handlers::send: "
        "discord command reply sent "
        'event="discord.command.session_admin_json.replied" '
        'session_key="1001:2002" recipient="1001" '
        "reply_chars=88 reply_bytes=88"
    )
    parsed = module.parse_command_reply_event_line(line)
    assert parsed is not None
    assert parsed["event"] == "discord.command.session_admin_json.replied"
    assert parsed["session_key"] == "1001:2002"
    assert parsed["recipient"] == "1001"


def test_parse_command_reply_json_summary_line_accepts_generic_phrase() -> None:
    module = _load_probe_module()
    line = (
        "2026-02-21 INFO omni_agent::channels::discord::runtime::managed::handlers::send: "
        "discord command reply json summary "
        'event="discord.command.session_budget_json.replied" '
        'session_key="1001:2002" recipient="1001" '
        "json_kind=session_budget json_session_scope=discord:1001:2002 "
        "json_available=false json_status=not_found json_keys=4"
    )
    parsed = module.parse_command_reply_json_summary_line(line)
    assert parsed is not None
    assert parsed["event"] == "discord.command.session_budget_json.replied"
    assert parsed["json_kind"] == "session_budget"
    assert parsed["json_session_scope"] == "discord:1001:2002"


def test_run_probe_allow_no_bot_matches_discord_control_admin_required_event(
    tmp_path, monkeypatch
) -> None:
    module = _load_probe_module()
    log_file = tmp_path / "agent.log"
    log_file.write_text("", encoding="utf-8")
    cfg = _make_config(
        module,
        log_file,
        allow_no_bot=True,
        expect_events=("discord.command.control_admin_required.replied",),
    )

    update_id = 1_700_000_030_000
    _patch_runtime(monkeypatch, module, update_id)
    monkeypatch.setattr(
        module,
        "read_new_lines",
        _sequence_reader(
            [
                [
                    "2026-02-21 INFO Parsed message, forwarding to agent "
                    "session_key=1001:2002 content_preview=/session admin add 1001"
                ],
                [
                    "2026-02-21 INFO omni_agent::channels::discord::runtime::managed::handlers::send: "
                    "discord command reply sent "
                    'event="discord.command.control_admin_required.replied" '
                    'session_key="1001:2002" recipient="1001" '
                    "reply_chars=210 reply_bytes=210"
                ],
            ]
        ),
    )

    assert module.run_probe(cfg) == 0


def test_run_probe_discord_mismatched_session_key_fails_scope_validation(
    tmp_path, monkeypatch
) -> None:
    module = _load_probe_module()
    log_file = tmp_path / "agent.log"
    log_file.write_text("", encoding="utf-8")
    cfg = _make_config(
        module,
        log_file,
        allow_no_bot=True,
        expect_events=("discord.command.control_admin_required.replied",),
    )

    update_id = 1_700_000_030_100
    _patch_runtime(monkeypatch, module, update_id)
    monkeypatch.setattr(
        module,
        "read_new_lines",
        _sequence_reader(
            [
                [
                    "2026-02-21 INFO Parsed message, forwarding to agent "
                    "session_key=1001:2002 content_preview=/session admin add 1001"
                ],
                [
                    "2026-02-21 INFO omni_agent::channels::discord::runtime::managed::handlers::send: "
                    "discord command reply sent "
                    'event="discord.command.control_admin_required.replied" '
                    'session_key="1001:7777" recipient="1001" '
                    "reply_chars=210 reply_bytes=210"
                ],
            ]
        ),
    )

    assert module.run_probe(cfg) == 10


def test_run_probe_discord_session_scope_placeholder_fails_on_prefix_mismatch(
    tmp_path, monkeypatch
) -> None:
    module = _load_probe_module()
    log_file = tmp_path / "agent.log"
    log_file.write_text("", encoding="utf-8")
    cfg = _make_config(
        module,
        log_file,
        prompt="/session memory json",
        allow_no_bot=True,
        expect_events=("discord.command.session_memory_json.replied",),
        expect_reply_json_fields=(
            ("json_kind", "session_memory"),
            ("json_session_scope", "__target_session_scope__"),
        ),
    )

    update_id = 1_700_000_030_300
    _patch_runtime(monkeypatch, module, update_id)
    monkeypatch.setattr(
        module,
        "read_new_lines",
        _sequence_reader(
            [
                [
                    "2026-02-21 INFO Parsed message, forwarding to agent "
                    "session_key=1001:2002 content_preview=/session memory json"
                ],
                [
                    "2026-02-21 INFO omni_agent::channels::discord::runtime::managed::handlers::send: "
                    "discord command reply sent "
                    'event="discord.command.session_memory_json.replied" '
                    'session_key="1001:2002" recipient="1001" reply_chars=210 reply_bytes=210',
                    "2026-02-21 INFO omni_agent::channels::discord::runtime::managed::handlers::send: "
                    "discord command reply json summary "
                    'event="discord.command.session_memory_json.replied" '
                    'session_key="1001:2002" recipient="1001" '
                    "json_kind=session_memory json_session_scope=telegram:1001:2002 "
                    "json_available=false json_status=not_found json_keys=7",
                ],
            ]
        ),
    )

    assert module.run_probe(cfg) == 3


def test_run_probe_discord_session_scope_placeholder_matches_discord_scope(
    tmp_path, monkeypatch
) -> None:
    module = _load_probe_module()
    log_file = tmp_path / "agent.log"
    log_file.write_text("", encoding="utf-8")
    cfg = _make_config(
        module,
        log_file,
        prompt="/session memory json",
        allow_no_bot=True,
        expect_events=("discord.command.session_memory_json.replied",),
        expect_reply_json_fields=(
            ("json_kind", "session_memory"),
            ("json_session_scope", "__target_session_scope__"),
        ),
    )

    update_id = 1_700_000_030_301
    _patch_runtime(monkeypatch, module, update_id)
    monkeypatch.setattr(
        module,
        "read_new_lines",
        _sequence_reader(
            [
                [
                    "2026-02-21 INFO Parsed message, forwarding to agent "
                    "session_key=1001:2002 content_preview=/session memory json"
                ],
                [
                    "2026-02-21 INFO omni_agent::channels::discord::runtime::managed::handlers::send: "
                    "discord command reply sent "
                    'event="discord.command.session_memory_json.replied" '
                    'session_key="1001:2002" recipient="1001" reply_chars=210 reply_bytes=210',
                    "2026-02-21 INFO omni_agent::channels::discord::runtime::managed::handlers::send: "
                    "discord command reply json summary "
                    'event="discord.command.session_memory_json.replied" '
                    'session_key="1001:2002" recipient="1001" '
                    "json_kind=session_memory json_session_scope=discord:1001:2002 "
                    "json_available=false json_status=not_found json_keys=7",
                ],
            ]
        ),
    )

    assert module.run_probe(cfg) == 0


def test_run_probe_discord_event_without_command_reply_observation_fails_scope_check(
    tmp_path, monkeypatch
) -> None:
    module = _load_probe_module()
    log_file = tmp_path / "agent.log"
    log_file.write_text("", encoding="utf-8")
    cfg = _make_config(
        module,
        log_file,
        allow_no_bot=True,
        expect_events=("discord.command.control_admin_required.replied",),
    )

    update_id = 1_700_000_030_200
    _patch_runtime(monkeypatch, module, update_id)
    monkeypatch.setattr(
        module,
        "read_new_lines",
        _sequence_reader(
            [
                [
                    "2026-02-21 INFO Parsed message, forwarding to agent "
                    "session_key=1001:2002 content_preview=/session admin add 1001"
                ],
                [
                    '2026-02-21 DEBUG event="discord.command.control_admin_required.replied" '
                    "component=discord.runtime.auth"
                ],
            ]
        ),
    )

    assert module.run_probe(cfg) == 10
