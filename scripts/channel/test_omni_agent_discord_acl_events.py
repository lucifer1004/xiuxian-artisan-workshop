#!/usr/bin/env python3
"""
Run Discord ACL black-box probes against local omni-agent Discord ingress runtime.

The probe posts synthetic Discord ingress events to `/discord/ingress`, then validates
managed-command observability events from runtime logs with strict target-scope checks.
"""

from __future__ import annotations

import argparse
import importlib
import json
import os
import re
import secrets
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path

DISCORD_INGRESS_SECRET_HEADER = "x-omni-discord-ingress-token"
SUITES = ("core", "all")
ERROR_PATTERNS = (
    "discord failed to send command reply",
    "Foreground message handling failed",
    "tools/call: Mcp error",
)
FORBIDDEN_LOG_PATTERN = "tools/call: Mcp error"

_SCRIPT_DIR = Path(__file__).resolve().parent
if str(_SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPT_DIR))

load_sibling_module = importlib.import_module("module_loader").load_sibling_module

BLACKBOX = load_sibling_module(
    module_name="agent_channel_blackbox",
    file_name="agent_channel_blackbox.py",
    caller_file=__file__,
    error_context="blackbox module",
)
TARGET_SESSION_SCOPE_PLACEHOLDER = getattr(
    BLACKBOX, "TARGET_SESSION_SCOPE_PLACEHOLDER", "__target_session_scope__"
)
DISCORD_SESSION_SCOPE_PREFIX = "discord:"


@dataclass(frozen=True)
class ProbeCase:
    case_id: str
    prompt: str
    event_name: str
    suites: tuple[str, ...]
    expect_reply_json_fields: tuple[str, ...] = ()


@dataclass(frozen=True)
class ProbeConfig:
    ingress_url: str
    log_file: Path
    max_wait_secs: int
    max_idle_secs: int
    channel_id: str
    user_id: str
    guild_id: str | None
    username: str | None
    role_ids: tuple[str, ...]
    secret_token: str | None
    session_partition: str
    no_follow: bool


def _normalize_ingress_bind_for_local_url(bind_addr: str) -> str:
    token = bind_addr.strip()
    if not token:
        return "127.0.0.1:8082"
    host, sep, port = token.rpartition(":")
    if not sep:
        return f"127.0.0.1:{token}"
    normalized_host = host.strip("[]").strip()
    if normalized_host in {"", "0.0.0.0", "::"}:
        normalized_host = "127.0.0.1"
    return f"{normalized_host}:{port.strip()}"


def default_ingress_url() -> str:
    explicit = os.environ.get("OMNI_DISCORD_INGRESS_URL", "").strip()
    if explicit:
        return explicit
    bind_addr = os.environ.get("OMNI_AGENT_DISCORD_INGRESS_BIND", "127.0.0.1:8082")
    path = os.environ.get("OMNI_AGENT_DISCORD_INGRESS_PATH", "/discord/ingress").strip()
    if not path:
        path = "/discord/ingress"
    if not path.startswith("/"):
        path = f"/{path}"
    return f"http://{_normalize_ingress_bind_for_local_url(bind_addr)}{path}"


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Run Discord ACL black-box probes against local ingress runtime. "
            "Each probe requires a command-specific reply event."
        )
    )
    parser.add_argument(
        "--ingress-url",
        default=default_ingress_url(),
        help="Discord ingress URL.",
    )
    parser.add_argument(
        "--log-file",
        default=os.environ.get("OMNI_CHANNEL_LOG_FILE", ".run/logs/omni-agent-webhook.log"),
        help="Runtime log file path.",
    )
    parser.add_argument(
        "--max-wait",
        type=int,
        default=int(os.environ.get("OMNI_BLACKBOX_MAX_WAIT_SECS", "25")),
        help="Overall wait upper-bound per probe in seconds.",
    )
    parser.add_argument(
        "--max-idle-secs",
        type=int,
        default=int(os.environ.get("OMNI_BLACKBOX_MAX_IDLE_SECS", "25")),
        help="Max idle wait for new logs per probe in seconds.",
    )
    parser.add_argument(
        "--channel-id",
        default=os.environ.get("OMNI_TEST_DISCORD_CHANNEL_ID", "").strip(),
        help="Discord channel_id used for synthetic ingress event.",
    )
    parser.add_argument(
        "--user-id",
        default=os.environ.get("OMNI_TEST_DISCORD_USER_ID", "").strip(),
        help="Discord user_id used for synthetic ingress event.",
    )
    parser.add_argument(
        "--guild-id",
        default=os.environ.get("OMNI_TEST_DISCORD_GUILD_ID", "").strip() or None,
        help="Discord guild_id (optional, defaults to DM scope).",
    )
    parser.add_argument(
        "--username",
        default=os.environ.get("OMNI_TEST_DISCORD_USERNAME", "").strip() or None,
        help="Discord username (optional).",
    )
    parser.add_argument(
        "--role-id",
        action="append",
        default=[],
        help="Discord role id attached to synthetic member.roles (repeatable).",
    )
    parser.add_argument(
        "--secret-token",
        default=os.environ.get("OMNI_TEST_DISCORD_INGRESS_SECRET", "").strip() or None,
        help="Ingress secret token for header x-omni-discord-ingress-token.",
    )
    parser.add_argument(
        "--session-partition",
        default=os.environ.get("OMNI_AGENT_DISCORD_SESSION_PARTITION", "guild_channel_user"),
        help="Discord session partition mode: guild_channel_user|channel|user|guild_user.",
    )
    parser.add_argument(
        "--suite",
        action="append",
        choices=SUITES,
        default=[],
        help="Run selected suite(s): core, all. Repeatable. Default: all.",
    )
    parser.add_argument(
        "--case",
        action="append",
        default=[],
        help="Run only specific case id(s). Repeatable. Use --list-cases to inspect ids.",
    )
    parser.add_argument(
        "--list-cases",
        action="store_true",
        help="List available case ids and exit.",
    )
    parser.add_argument(
        "--no-follow",
        action="store_true",
        help="Disable live log streaming while waiting.",
    )
    parser.add_argument(
        "--allow-no-bot",
        action="store_true",
        default=True,
        help="Reserved flag for compatibility with Telegram probe semantics.",
    )
    return parser.parse_args()


def normalize_partition_mode(value: str) -> str:
    token = value.strip().lower().replace("-", "_")
    if token in {"guild_channel_user", "channel_user", "guildchanneluser"}:
        return "guild_channel_user"
    if token in {"channel", "channel_only", "channelonly"}:
        return "channel"
    if token in {"user", "user_only", "useronly"}:
        return "user"
    if token in {"guild_user", "guilduser"}:
        return "guild_user"
    raise ValueError(
        "invalid --session-partition; expected guild_channel_user|channel|user|guild_user"
    )


def dedup(values: list[str]) -> tuple[str, ...]:
    ordered: list[str] = []
    for value in values:
        token = value.strip()
        if not token:
            continue
        if token not in ordered:
            ordered.append(token)
    return tuple(ordered)


def build_config(args: argparse.Namespace) -> ProbeConfig:
    channel_id = args.channel_id.strip()
    user_id = args.user_id.strip()
    if not channel_id or not user_id:
        raise ValueError(
            "--channel-id and --user-id are required (or set OMNI_TEST_DISCORD_CHANNEL_ID "
            "and OMNI_TEST_DISCORD_USER_ID)."
        )
    return ProbeConfig(
        ingress_url=args.ingress_url,
        log_file=Path(args.log_file),
        max_wait_secs=args.max_wait,
        max_idle_secs=args.max_idle_secs,
        channel_id=channel_id,
        user_id=user_id,
        guild_id=(
            args.guild_id.strip() if isinstance(args.guild_id, str) and args.guild_id else None
        ),
        username=args.username,
        role_ids=dedup(args.role_id),
        secret_token=args.secret_token,
        session_partition=normalize_partition_mode(args.session_partition),
        no_follow=bool(args.no_follow),
    )


def now_event_id() -> str:
    base_ms = int(time.time() * 1000)
    pid_component = os.getpid() % 10_000
    rand_component = secrets.randbelow(100)
    return str((base_ms * 1_000_000) + (pid_component * 100) + rand_component)


def expected_session_keys(
    partition_mode: str,
    guild_id: str | None,
    channel_id: str,
    user_id: str,
) -> tuple[str, ...]:
    scope = guild_id if guild_id else "dm"
    if partition_mode == "guild_channel_user":
        return (f"{scope}:{channel_id}:{user_id}",)
    if partition_mode == "channel":
        return (f"{scope}:{channel_id}",)
    if partition_mode == "user":
        return (user_id,)
    return (f"{scope}:{user_id}",)


def expected_session_scopes(
    partition_mode: str,
    guild_id: str | None,
    channel_id: str,
    user_id: str,
) -> tuple[str, ...]:
    return tuple(
        f"{DISCORD_SESSION_SCOPE_PREFIX}{session_key}"
        for session_key in expected_session_keys(partition_mode, guild_id, channel_id, user_id)
    )


def build_ingress_payload(config: ProbeConfig, event_id: str, prompt: str) -> str:
    payload: dict[str, object] = {
        "id": event_id,
        "content": prompt,
        "channel_id": config.channel_id,
        "author": {"id": config.user_id},
    }
    if config.username:
        payload["author"] = {"id": config.user_id, "username": config.username}
    if config.guild_id:
        payload["guild_id"] = config.guild_id
        if config.role_ids:
            payload["member"] = {"roles": list(config.role_ids)}
    return json.dumps(payload, ensure_ascii=False)


def post_ingress_event(url: str, payload: str, secret_token: str | None) -> tuple[int, str]:
    data = payload.encode("utf-8")
    request = urllib.request.Request(url=url, data=data, method="POST")
    request.add_header("content-type", "application/json")
    if secret_token:
        request.add_header(DISCORD_INGRESS_SECRET_HEADER, secret_token)
    try:
        with urllib.request.urlopen(request, timeout=15) as response:
            body = response.read().decode("utf-8", errors="replace")
            return response.status, body
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", errors="replace")
        return int(error.code), body
    except urllib.error.URLError as error:
        return 0, f"connection_error: {error.reason}"


def parse_expected_field(value: str) -> tuple[str, str]:
    return BLACKBOX.parse_expected_field(value)


def compile_patterns(patterns: tuple[str, ...]) -> list[re.Pattern[str]]:
    return [re.compile(pattern) for pattern in patterns]


def reply_json_field_matches(
    *,
    key: str,
    expected: str,
    observation: dict[str, str],
    expected_session_scopes_values: tuple[str, ...],
) -> bool:
    actual = observation.get(key)
    if key == "json_session_scope" and expected == TARGET_SESSION_SCOPE_PLACEHOLDER:
        return actual in expected_session_scopes_values
    return actual == expected


def run_case(
    config: ProbeConfig,
    case: ProbeCase,
) -> int:
    expect_fields = tuple(parse_expected_field(item) for item in case.expect_reply_json_fields)
    expected_sessions = expected_session_keys(
        config.session_partition,
        config.guild_id,
        config.channel_id,
        config.user_id,
    )
    expected_session_scopes_values = expected_session_scopes(
        config.session_partition,
        config.guild_id,
        config.channel_id,
        config.user_id,
    )
    expected_recipient = config.channel_id
    cursor = BLACKBOX.count_lines(config.log_file)

    event_id = now_event_id()
    payload = build_ingress_payload(config, event_id=event_id, prompt=case.prompt)
    status, body = post_ingress_event(config.ingress_url, payload, config.secret_token)
    if status != 200:
        print(f"[{case.case_id}] ingress POST failed: HTTP {status}", file=sys.stderr)
        for line in body.splitlines():
            print(f"  {line}", file=sys.stderr)
        return 1

    deadline = time.monotonic() + config.max_wait_secs
    last_log_activity = time.monotonic()
    seen_dispatch = False
    seen_bot = False
    bot_lines: list[str] = []
    command_reply_observations: list[dict[str, object]] = []
    json_reply_summary_observations: list[dict[str, str]] = []
    matched_expect_event = False
    matched_expect_reply_json = [False] * len(expect_fields)
    forbid_log_patterns = compile_patterns((FORBIDDEN_LOG_PATTERN,))

    while True:
        if time.monotonic() > deadline:
            break

        cursor, chunk = BLACKBOX.read_new_lines(config.log_file, cursor)
        if chunk:
            last_log_activity = time.monotonic()
            normalized_chunk = [BLACKBOX.strip_ansi(line) for line in chunk]
            if not config.no_follow:
                for line in chunk:
                    print(f"[log] {line}")
            for line in normalized_chunk:
                event = BLACKBOX.extract_event_token(line)
                if event == case.event_name:
                    tokens = BLACKBOX.parse_log_tokens(line)
                    recipient = tokens.get("recipient", "")
                    if recipient == expected_recipient:
                        matched_expect_event = True

                reply_obs = BLACKBOX.parse_command_reply_event_line(line)
                if reply_obs:
                    command_reply_observations.append(reply_obs)
                json_summary_obs = BLACKBOX.parse_command_reply_json_summary_line(line)
                if json_summary_obs:
                    json_reply_summary_observations.append(json_summary_obs)
                    if (
                        json_summary_obs.get("event") == case.event_name
                        and json_summary_obs.get("recipient") == expected_recipient
                    ):
                        for idx, (key, expected) in enumerate(expect_fields):
                            if matched_expect_reply_json[idx]:
                                continue
                            if reply_json_field_matches(
                                key=key,
                                expected=expected,
                                observation=json_summary_obs,
                                expected_session_scopes_values=expected_session_scopes_values,
                            ):
                                matched_expect_reply_json[idx] = True

                for pattern in forbid_log_patterns:
                    if pattern.search(line):
                        print(
                            f"[{case.case_id}] forbidden log matched: {pattern.pattern}",
                            file=sys.stderr,
                        )
                        print(f"  line={line}", file=sys.stderr)
                        return 5
                if any(pattern in line for pattern in ERROR_PATTERNS):
                    print(f"[{case.case_id}] fail-fast error log detected.", file=sys.stderr)
                    print(f"  line={line}", file=sys.stderr)
                    return 6

                if "discord ingress parsed message" in line:
                    seen_dispatch = True
                if "→ Bot:" in line:
                    seen_bot = True
                    bot_lines.append(line)

            if seen_dispatch and matched_expect_event and all(matched_expect_reply_json):
                break

        if (
            config.max_idle_secs > 0
            and (time.monotonic() - last_log_activity) > config.max_idle_secs
        ):
            print(f"[{case.case_id}] max-idle exceeded.", file=sys.stderr)
            return 7
        time.sleep(1)

    if not seen_dispatch:
        print(f"[{case.case_id}] timed out: no discord ingress dispatch marker.", file=sys.stderr)
        return 9
    if not matched_expect_event or not all(matched_expect_reply_json):
        print(f"[{case.case_id}] timed out: missing expected event/json fields.", file=sys.stderr)
        print(f"  expect_event={case.event_name}", file=sys.stderr)
        if expect_fields:
            print(
                "  expect_reply_json=" + ",".join(f"{key}={value}" for key, value in expect_fields),
                file=sys.stderr,
            )
        return 8

    target_obs = None
    for obs in command_reply_observations:
        if obs.get("event") != case.event_name:
            continue
        if str(obs.get("recipient") or "") != expected_recipient:
            continue
        target_obs = obs
        break
    if target_obs is None:
        print(f"[{case.case_id}] missing target-scoped command reply observation.", file=sys.stderr)
        return 10
    observed_session = str(target_obs.get("session_key") or "")
    if observed_session and observed_session not in expected_sessions:
        print(f"[{case.case_id}] command reply session_key mismatch.", file=sys.stderr)
        print(f"  expected_session_keys={list(expected_sessions)}", file=sys.stderr)
        print(f"  observed_session_key={observed_session}", file=sys.stderr)
        return 10
    target_summary = None
    for summary in json_reply_summary_observations:
        if summary.get("event") != case.event_name:
            continue
        if str(summary.get("recipient") or "") != expected_recipient:
            continue
        target_summary = summary
        break
    observed_session_scope = ""
    if target_summary is not None:
        observed_summary_session = str(target_summary.get("session_key") or "")
        if observed_summary_session and observed_summary_session not in expected_sessions:
            print(
                f"[{case.case_id}] command reply json summary session_key mismatch.",
                file=sys.stderr,
            )
            print(f"  expected_session_keys={list(expected_sessions)}", file=sys.stderr)
            print(f"  observed_session_key={observed_summary_session}", file=sys.stderr)
            return 10
        observed_session_scope = str(target_summary.get("json_session_scope") or "")
        if observed_session_scope and observed_session_scope not in expected_session_scopes_values:
            print(
                f"[{case.case_id}] command reply json summary session_scope mismatch.",
                file=sys.stderr,
            )
            print(
                f"  expected_json_session_scopes={list(expected_session_scopes_values)}",
                file=sys.stderr,
            )
            print(f"  observed_json_session_scope={observed_session_scope}", file=sys.stderr)
            return 10

    print(f"[{case.case_id}] pass")
    if seen_bot:
        print(f"  bot_logs={len(bot_lines)}")
    print(f"  event={case.event_name}")
    print(f"  session_key={observed_session}")
    if observed_session_scope:
        print(f"  json_session_scope={observed_session_scope}")
    return 0


def selected_suites(args: argparse.Namespace) -> tuple[str, ...]:
    if not args.suite:
        return ("all",)
    ordered = dedup(args.suite)
    if "all" in ordered:
        return ("all",)
    return ordered


def build_cases(user_id: str) -> list[ProbeCase]:
    return [
        ProbeCase(
            case_id="discord_control_admin_denied",
            prompt=f"/session admin add {user_id}",
            event_name="discord.command.control_admin_required.replied",
            suites=("core",),
        ),
        ProbeCase(
            case_id="discord_slash_permission_denied",
            prompt="/session memory",
            event_name="discord.command.slash_permission_required.replied",
            suites=("core",),
        ),
    ]


def filter_cases(
    cases: list[ProbeCase], suites: tuple[str, ...], requested_case_ids: tuple[str, ...]
) -> list[ProbeCase]:
    result = []
    for case in cases:
        if requested_case_ids and case.case_id not in requested_case_ids:
            continue
        if "all" not in suites and not any(suite in suites for suite in case.suites):
            continue
        result.append(case)
    return result


def list_cases(cases: list[ProbeCase]) -> int:
    print("Available Discord ACL cases:")
    for case in cases:
        print(f"- {case.case_id} ({','.join(case.suites)}) -> {case.prompt}")
    return 0


def main() -> int:
    args = parse_args()
    suites = selected_suites(args)
    requested_case_ids = dedup(args.case)

    if args.list_cases:
        preview_cases = build_cases(user_id=args.user_id.strip() or "{user_id}")
        selected_preview = filter_cases(preview_cases, suites, requested_case_ids)
        return list_cases(selected_preview if selected_preview else preview_cases)

    try:
        config = build_config(args)
    except ValueError as error:
        print(f"Error: {error}", file=sys.stderr)
        return 2

    cases = build_cases(user_id=config.user_id)

    selected = filter_cases(cases, suites, requested_case_ids)
    if not selected:
        print("No Discord ACL cases selected.", file=sys.stderr)
        return 2

    start = time.time()
    records: list[dict[str, object]] = []
    failures = 0
    for case in selected:
        started = time.time()
        rc = run_case(config, case)
        duration_ms = int((time.time() - started) * 1000)
        records.append(
            {
                "case_id": case.case_id,
                "prompt": case.prompt,
                "event_name": case.event_name,
                "returncode": rc,
                "passed": rc == 0,
                "duration_ms": duration_ms,
            }
        )
        if rc != 0:
            failures += 1

    elapsed_ms = int((time.time() - start) * 1000)
    report = {
        "generated_at_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "ingress_url": config.ingress_url,
        "log_file": str(config.log_file),
        "session_partition": config.session_partition,
        "channel_id": config.channel_id,
        "user_id": config.user_id,
        "guild_id": config.guild_id,
        "total": len(records),
        "passed": len(records) - failures,
        "failed": failures,
        "duration_ms": elapsed_ms,
        "records": records,
    }
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
