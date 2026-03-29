"""Tests for scripts/channel/test_xiuxian_daochang_discord_acl_events.py."""

from __future__ import annotations

import argparse
import importlib.util
import sys
from typing import TYPE_CHECKING

from xiuxian_wendao_py.compat.runtime import get_project_root

if TYPE_CHECKING:
    from pathlib import Path
    from types import ModuleType


def _load_module() -> ModuleType:
    root = get_project_root()
    script_path = root / "scripts" / "channel" / "test_xiuxian_daochang_discord_acl_events.py"
    spec = importlib.util.spec_from_file_location(
        "xiuxian_daochang_discord_acl_events", script_path
    )
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def _sequence_reader(chunks: list[list[str]]):
    state = {"idx": 0}

    def _read_new_lines(_: Path, cursor: int) -> tuple[int, list[str]]:
        if state["idx"] >= len(chunks):
            return cursor, []
        chunk = chunks[state["idx"]]
        state["idx"] += 1
        return cursor + len(chunk), chunk

    return _read_new_lines


def test_normalize_partition_mode_aliases() -> None:
    module = _load_module()
    assert module.normalize_partition_mode("guild_channel_user") == "guild_channel_user"
    assert module.normalize_partition_mode("guild-channel-user") == "guild_channel_user"
    assert module.normalize_partition_mode("channel_only") == "channel"
    assert module.normalize_partition_mode("user") == "user"
    assert module.normalize_partition_mode("guild-user") == "guild_user"


def test_build_config_requires_channel_and_user_ids(tmp_path) -> None:
    module = _load_module()
    args = argparse.Namespace(
        ingress_url="http://127.0.0.1:8082/discord/ingress",
        log_file=str(tmp_path / "runtime.log"),
        max_wait=20,
        max_idle_secs=20,
        channel_id="",
        user_id="",
        guild_id=None,
        username=None,
        role_id=[],
        secret_token=None,
        session_partition="guild_channel_user",
        no_follow=True,
    )
    try:
        module.build_config(args)
    except ValueError as error:
        assert "--channel-id and --user-id are required" in str(error)
        return
    raise AssertionError("expected ValueError when channel/user ids are missing")


def test_expected_session_keys_by_partition() -> None:
    module = _load_module()
    assert module.expected_session_keys("guild_channel_user", "3001", "2001", "1001") == (
        "3001:2001:1001",
    )
    assert module.expected_session_keys("channel", "3001", "2001", "1001") == ("3001:2001",)
    assert module.expected_session_keys("user", "3001", "2001", "1001") == ("1001",)
    assert module.expected_session_keys("guild_user", "3001", "2001", "1001") == ("3001:1001",)
    assert module.expected_session_keys("guild_channel_user", None, "2001", "1001") == (
        "dm:2001:1001",
    )


def test_expected_session_scopes_by_partition() -> None:
    module = _load_module()
    assert module.expected_session_scopes("guild_channel_user", "3001", "2001", "1001") == (
        "discord:3001:2001:1001",
    )
    assert module.expected_session_scopes("channel", "3001", "2001", "1001") == (
        "discord:3001:2001",
    )
    assert module.expected_session_scopes("user", "3001", "2001", "1001") == ("discord:1001",)
    assert module.expected_session_scopes("guild_user", "3001", "2001", "1001") == (
        "discord:3001:1001",
    )
    assert module.expected_session_scopes("guild_channel_user", None, "2001", "1001") == (
        "discord:dm:2001:1001",
    )


def test_build_ingress_payload_includes_optional_fields(tmp_path) -> None:
    module = _load_module()
    config = module.ProbeConfig(
        ingress_url="http://127.0.0.1:8082/discord/ingress",
        log_file=tmp_path / "runtime.log",
        max_wait_secs=20,
        max_idle_secs=20,
        channel_id="2001",
        user_id="1001",
        guild_id="3001",
        username="alice",
        role_ids=("987654321012345678",),
        secret_token="secret",
        session_partition="guild_channel_user",
        no_follow=True,
    )
    payload = module.build_ingress_payload(
        config, event_id="1700000000000", prompt="/session memory"
    )
    assert '"guild_id": "3001"' in payload
    assert '"username": "alice"' in payload
    assert '"roles": ["987654321012345678"]' in payload


def test_run_case_succeeds_with_target_reply_observation(tmp_path, monkeypatch) -> None:
    module = _load_module()
    log_file = tmp_path / "runtime.log"
    log_file.write_text("", encoding="utf-8")
    config = module.ProbeConfig(
        ingress_url="http://127.0.0.1:8082/discord/ingress",
        log_file=log_file,
        max_wait_secs=5,
        max_idle_secs=5,
        channel_id="2001",
        user_id="1001",
        guild_id="3001",
        username="alice",
        role_ids=(),
        secret_token=None,
        session_partition="guild_channel_user",
        no_follow=True,
    )
    case = module.ProbeCase(
        case_id="discord_control_admin_denied",
        prompt="/session admin add 1001",
        event_name="discord.command.control_admin_required.replied",
        suites=("core",),
    )

    monkeypatch.setattr(module, "post_ingress_event", lambda *_: (200, "ok"))
    monkeypatch.setattr(module, "now_event_id", lambda: "1700000000000")
    monkeypatch.setattr(module.BLACKBOX, "count_lines", lambda _: 0)
    monkeypatch.setattr(
        module.BLACKBOX,
        "read_new_lines",
        _sequence_reader(
            [
                [
                    "2026-02-21 INFO discord ingress parsed message "
                    'session_key="3001:2001:1001" recipient="2001"',
                    "2026-02-21 INFO discord command reply sent "
                    'event="discord.command.control_admin_required.replied" '
                    'session_key="3001:2001:1001" recipient="2001" reply_chars=88 reply_bytes=88',
                ]
            ]
        ),
    )

    assert module.run_case(config, case) == 0


def test_run_case_fails_on_session_scope_mismatch(tmp_path, monkeypatch) -> None:
    module = _load_module()
    log_file = tmp_path / "runtime.log"
    log_file.write_text("", encoding="utf-8")
    config = module.ProbeConfig(
        ingress_url="http://127.0.0.1:8082/discord/ingress",
        log_file=log_file,
        max_wait_secs=5,
        max_idle_secs=5,
        channel_id="2001",
        user_id="1001",
        guild_id="3001",
        username="alice",
        role_ids=(),
        secret_token=None,
        session_partition="guild_channel_user",
        no_follow=True,
    )
    case = module.ProbeCase(
        case_id="discord_control_admin_denied",
        prompt="/session admin add 1001",
        event_name="discord.command.control_admin_required.replied",
        suites=("core",),
    )

    monkeypatch.setattr(module, "post_ingress_event", lambda *_: (200, "ok"))
    monkeypatch.setattr(module, "now_event_id", lambda: "1700000000001")
    monkeypatch.setattr(module.BLACKBOX, "count_lines", lambda _: 0)
    monkeypatch.setattr(
        module.BLACKBOX,
        "read_new_lines",
        _sequence_reader(
            [
                [
                    "2026-02-21 INFO discord ingress parsed message "
                    'session_key="3001:2001:1001" recipient="2001"',
                    "2026-02-21 INFO discord command reply sent "
                    'event="discord.command.control_admin_required.replied" '
                    'session_key="3001:2001:9999" recipient="2001" reply_chars=88 reply_bytes=88',
                ]
            ]
        ),
    )

    assert module.run_case(config, case) == 10


def test_run_case_matches_json_session_scope_placeholder(tmp_path, monkeypatch) -> None:
    module = _load_module()
    log_file = tmp_path / "runtime.log"
    log_file.write_text("", encoding="utf-8")
    config = module.ProbeConfig(
        ingress_url="http://127.0.0.1:8082/discord/ingress",
        log_file=log_file,
        max_wait_secs=5,
        max_idle_secs=5,
        channel_id="2001",
        user_id="1001",
        guild_id="3001",
        username="alice",
        role_ids=(),
        secret_token=None,
        session_partition="guild_channel_user",
        no_follow=True,
    )
    case = module.ProbeCase(
        case_id="discord_session_memory_json_scope",
        prompt="/session memory json",
        event_name="discord.command.session_memory_json.replied",
        suites=("core",),
        expect_reply_json_fields=(
            "json_kind=session_memory",
            "json_session_scope=__target_session_scope__",
        ),
    )

    monkeypatch.setattr(module, "post_ingress_event", lambda *_: (200, "ok"))
    monkeypatch.setattr(module, "now_event_id", lambda: "1700000000002")
    monkeypatch.setattr(module.BLACKBOX, "count_lines", lambda _: 0)
    monkeypatch.setattr(
        module.BLACKBOX,
        "read_new_lines",
        _sequence_reader(
            [
                [
                    "2026-02-21 INFO discord ingress parsed message "
                    'session_key="3001:2001:1001" recipient="2001"',
                    "2026-02-21 INFO discord command reply sent "
                    'event="discord.command.session_memory_json.replied" '
                    'session_key="3001:2001:1001" recipient="2001" reply_chars=88 reply_bytes=88',
                    "2026-02-21 INFO discord command reply json summary "
                    'event="discord.command.session_memory_json.replied" '
                    'session_key="3001:2001:1001" recipient="2001" '
                    "json_kind=session_memory json_session_scope=discord:3001:2001:1001 "
                    "json_available=false json_status=not_found json_keys=7",
                ]
            ]
        ),
    )

    assert module.run_case(config, case) == 0


def test_run_case_fails_on_json_summary_session_scope_mismatch(tmp_path, monkeypatch) -> None:
    module = _load_module()
    log_file = tmp_path / "runtime.log"
    log_file.write_text("", encoding="utf-8")
    config = module.ProbeConfig(
        ingress_url="http://127.0.0.1:8082/discord/ingress",
        log_file=log_file,
        max_wait_secs=5,
        max_idle_secs=5,
        channel_id="2001",
        user_id="1001",
        guild_id="3001",
        username="alice",
        role_ids=(),
        secret_token=None,
        session_partition="guild_channel_user",
        no_follow=True,
    )
    case = module.ProbeCase(
        case_id="discord_control_admin_denied",
        prompt="/session admin add 1001",
        event_name="discord.command.control_admin_required.replied",
        suites=("core",),
    )

    monkeypatch.setattr(module, "post_ingress_event", lambda *_: (200, "ok"))
    monkeypatch.setattr(module, "now_event_id", lambda: "1700000000003")
    monkeypatch.setattr(module.BLACKBOX, "count_lines", lambda _: 0)
    monkeypatch.setattr(
        module.BLACKBOX,
        "read_new_lines",
        _sequence_reader(
            [
                [
                    "2026-02-21 INFO discord ingress parsed message "
                    'session_key="3001:2001:1001" recipient="2001"',
                    "2026-02-21 INFO discord command reply sent "
                    'event="discord.command.control_admin_required.replied" '
                    'session_key="3001:2001:1001" recipient="2001" reply_chars=88 reply_bytes=88',
                    "2026-02-21 INFO discord command reply json summary "
                    'event="discord.command.control_admin_required.replied" '
                    'session_key="3001:2001:1001" recipient="2001" '
                    "json_session_scope=discord:3001:2001:9999 json_keys=2",
                ]
            ]
        ),
    )

    assert module.run_case(config, case) == 10


def test_main_list_cases_does_not_require_channel_or_user_ids(monkeypatch, capsys) -> None:
    module = _load_module()
    monkeypatch.setattr(
        sys, "argv", ["test_xiuxian_daochang_discord_acl_events.py", "--list-cases"]
    )
    assert module.main() == 0
    stdout = capsys.readouterr().out
    assert "discord_control_admin_denied" in stdout
    assert "discord_slash_permission_denied" in stdout


def test_default_ingress_url_prefers_discord_bind_and_path(monkeypatch) -> None:
    module = _load_module()
    monkeypatch.delenv("OMNI_DISCORD_INGRESS_URL", raising=False)
    monkeypatch.setenv("XIUXIAN_DAOCHANG_DISCORD_INGRESS_BIND", "0.0.0.0:19082")
    monkeypatch.setenv("XIUXIAN_DAOCHANG_DISCORD_INGRESS_PATH", "/ingress/discord")
    expected_bind = module._normalize_ingress_bind_for_local_url("0.0.0.0:19082")
    assert module.default_ingress_url() == f"http://{expected_bind}/ingress/discord"


def test_default_ingress_url_prefers_explicit_url_override(monkeypatch) -> None:
    module = _load_module()
    monkeypatch.setenv("OMNI_DISCORD_INGRESS_URL", "http://127.0.0.1:29999/custom")
    monkeypatch.setenv("XIUXIAN_DAOCHANG_DISCORD_INGRESS_BIND", "0.0.0.0:19082")
    monkeypatch.setenv("XIUXIAN_DAOCHANG_DISCORD_INGRESS_PATH", "/ingress/discord")
    assert module.default_ingress_url() == "http://127.0.0.1:29999/custom"
