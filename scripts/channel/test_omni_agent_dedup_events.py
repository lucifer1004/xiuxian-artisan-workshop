#!/usr/bin/env python3
"""
Deterministic webhook dedup black-box probe.

Posts the same Telegram update_id twice and verifies:
  - first post accepted (`telegram.dedup.update_accepted`)
  - second post dropped as duplicate (`telegram.dedup.duplicate_detected`)
"""

from __future__ import annotations

import argparse
import importlib
import json
import os
import random
import re
import sys
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from pathlib import Path

_SCRIPT_DIR = Path(__file__).resolve().parent
if str(_SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPT_DIR))

load_sibling_module = importlib.import_module("module_loader").load_sibling_module

_resolver_module = load_sibling_module(
    module_name="config_resolver",
    file_name="config_resolver.py",
    caller_file=__file__,
    error_context="resolver module",
)
default_telegram_webhook_url = _resolver_module.default_telegram_webhook_url
session_ids_from_runtime_log = _resolver_module.session_ids_from_runtime_log
telegram_webhook_secret_token = _resolver_module.telegram_webhook_secret_token
username_from_runtime_log = _resolver_module.username_from_runtime_log
username_from_settings = _resolver_module.username_from_settings

_log_io_module = load_sibling_module(
    module_name="log_io",
    file_name="log_io.py",
    caller_file=__file__,
    error_context="shared log I/O helpers",
)
_SharedLogCursor = _log_io_module.LogCursor
_shared_init_log_cursor = _log_io_module.init_log_cursor
_shared_read_new_log_lines_with_cursor = _log_io_module.read_new_log_lines_with_cursor

ANSI_ESCAPE_RE = re.compile(r"\x1b\[[0-9;]*m")


@dataclass(frozen=True)
class ProbeConfig:
    max_wait: int
    webhook_url: str
    log_file: Path
    chat_id: int
    user_id: int
    username: str | None
    thread_id: int | None
    secret_token: str | None
    text: str


def strip_ansi(value: str) -> str:
    return ANSI_ESCAPE_RE.sub("", value)


def parse_args() -> argparse.Namespace:
    webhook_url_default = os.environ.get("OMNI_WEBHOOK_URL") or default_telegram_webhook_url()
    parser = argparse.ArgumentParser(
        description=(
            "Post the same Telegram update_id twice to local webhook runtime and assert "
            "accepted/duplicate dedup events."
        )
    )
    parser.add_argument(
        "--max-wait",
        type=int,
        default=int(os.environ.get("OMNI_BLACKBOX_MAX_WAIT_SECS", "25")),
        help="Max wait for dedup logs in seconds (default: 25).",
    )
    parser.add_argument(
        "--webhook-url",
        default=webhook_url_default,
        help="Webhook URL.",
    )
    parser.add_argument(
        "--log-file",
        default=os.environ.get("OMNI_CHANNEL_LOG_FILE", ".run/logs/omni-agent-webhook.log"),
        help="Runtime log file path.",
    )
    parser.add_argument(
        "--chat-id",
        type=int,
        default=int(os.environ["OMNI_TEST_CHAT_ID"]) if "OMNI_TEST_CHAT_ID" in os.environ else None,
        help="Telegram chat id (default: $OMNI_TEST_CHAT_ID).",
    )
    parser.add_argument(
        "--user-id",
        type=int,
        default=int(os.environ["OMNI_TEST_USER_ID"]) if "OMNI_TEST_USER_ID" in os.environ else None,
        help="Telegram user id (default: $OMNI_TEST_USER_ID).",
    )
    parser.add_argument(
        "--username",
        default=os.environ.get("OMNI_TEST_USERNAME"),
        help="Telegram username (default: $OMNI_TEST_USERNAME).",
    )
    parser.add_argument(
        "--thread-id",
        type=int,
        default=int(os.environ["OMNI_TEST_THREAD_ID"])
        if "OMNI_TEST_THREAD_ID" in os.environ
        else None,
        help="Optional Telegram thread/topic id.",
    )
    parser.add_argument(
        "--secret-token",
        default=os.environ.get("TELEGRAM_WEBHOOK_SECRET"),
        help="Webhook secret header value.",
    )
    parser.add_argument(
        "--text",
        default="/session json",
        help="Message text payload (default: /session json).",
    )
    return parser.parse_args()


def build_config(args: argparse.Namespace) -> ProbeConfig:
    chat_id: int | None = args.chat_id
    user_id: int | None = args.user_id
    thread_id: int | None = args.thread_id

    if chat_id is None or user_id is None:
        inferred_chat, inferred_user, inferred_thread = session_ids_from_runtime_log(
            Path(args.log_file)
        )
        if chat_id is None:
            chat_id = inferred_chat
        if user_id is None:
            user_id = inferred_user
        if thread_id is None:
            thread_id = inferred_thread

    if chat_id is None or user_id is None:
        raise ValueError(
            "chat/user id are required. Use --chat-id/--user-id "
            "(or OMNI_TEST_CHAT_ID/OMNI_TEST_USER_ID). "
            "Tip: send one real Telegram message first so session_key can be inferred from logs."
        )
    if args.max_wait <= 0:
        raise ValueError("--max-wait must be a positive integer.")

    username: str | None = args.username.strip() if args.username else None
    if not username:
        username = username_from_settings()
    if not username:
        username = username_from_runtime_log(Path(args.log_file))

    secret_token: str | None = args.secret_token.strip() if args.secret_token else None
    if not secret_token:
        secret_token = telegram_webhook_secret_token()

    return ProbeConfig(
        max_wait=args.max_wait,
        webhook_url=args.webhook_url,
        log_file=Path(args.log_file),
        chat_id=int(chat_id),
        user_id=int(user_id),
        username=username,
        thread_id=thread_id,
        secret_token=secret_token,
        text=args.text,
    )


def count_lines(path: Path) -> int:
    return _shared_init_log_cursor(path, kind="offset").value


def read_new_lines(path: Path, cursor: int) -> tuple[int, list[str]]:
    next_cursor, lines = _shared_read_new_log_lines_with_cursor(
        path,
        _SharedLogCursor(kind="offset", value=cursor),
    )
    return next_cursor.value, lines


def post_webhook_update(url: str, payload: bytes, secret_token: str | None) -> tuple[int, str]:
    request = urllib.request.Request(url=url, data=payload, method="POST")
    request.add_header("Content-Type", "application/json")
    if secret_token:
        request.add_header("X-Telegram-Bot-Api-Secret-Token", secret_token)
    try:
        with urllib.request.urlopen(request, timeout=15) as response:
            body = response.read().decode("utf-8", errors="replace")
            return int(response.status), body
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", errors="replace")
        return int(error.code), body
    except urllib.error.URLError as error:
        return 0, f"connection_error: {error.reason}"


def build_payload(cfg: ProbeConfig, update_id: int) -> bytes:
    from_user: dict[str, object] = {"id": cfg.user_id, "is_bot": False, "first_name": "DedupProbe"}
    if cfg.username:
        from_user["username"] = cfg.username
    message: dict[str, object] = {
        "message_id": update_id % 2_000_000_000,
        "date": update_id // 1_000_000,
        "text": cfg.text,
        "chat": {"id": cfg.chat_id, "type": "private" if cfg.chat_id > 0 else "group"},
        "from": from_user,
    }
    if cfg.thread_id is not None:
        message["message_thread_id"] = cfg.thread_id
    body = {"update_id": update_id, "message": message}
    return json.dumps(body, ensure_ascii=False).encode("utf-8")


def collect_stats(lines: list[str], update_id: int) -> dict[str, int]:
    normalized = [strip_ansi(line) for line in lines]
    accepted = [
        idx
        for idx, line in enumerate(normalized, start=1)
        if 'event="telegram.dedup.update_accepted"' in line and f"update_id={update_id}" in line
    ]
    duplicate = [
        idx
        for idx, line in enumerate(normalized, start=1)
        if 'event="telegram.dedup.duplicate_detected"' in line and f"update_id={update_id}" in line
    ]
    evaluated = [
        line
        for line in normalized
        if 'event="telegram.dedup.evaluated"' in line and f"update_id={update_id}" in line
    ]
    evaluated_true = sum("duplicate=true" in line for line in evaluated)
    evaluated_false = sum("duplicate=false" in line for line in evaluated)
    return {
        "accepted_count": len(accepted),
        "duplicate_count": len(duplicate),
        "accepted_line": accepted[0] if accepted else 0,
        "duplicate_line": duplicate[0] if duplicate else 0,
        "evaluated_total": len(evaluated),
        "evaluated_true": evaluated_true,
        "evaluated_false": evaluated_false,
    }


def print_relevant_tail(lines: list[str], update_id: int) -> None:
    relevant = [
        strip_ansi(line)
        for line in lines
        if "telegram.dedup." in line or f"update_id={update_id}" in line
    ]
    print("Relevant tail:", file=sys.stderr)
    for line in relevant[-60:]:
        print(f"  {line}", file=sys.stderr)


def main() -> int:
    try:
        cfg = build_config(parse_args())
    except ValueError as error:
        print(f"Error: {error}", file=sys.stderr)
        return 2

    cfg.log_file.parent.mkdir(parents=True, exist_ok=True)
    if not cfg.log_file.exists():
        cfg.log_file.touch()

    cursor = count_lines(cfg.log_file)
    update_id = (time.time_ns() // 1_000) + random.randint(0, 999)
    payload = build_payload(cfg, update_id)

    status_first, body_first = post_webhook_update(cfg.webhook_url, payload, cfg.secret_token)
    status_second, body_second = post_webhook_update(cfg.webhook_url, payload, cfg.secret_token)

    print("Dedup probe posted.")
    print(f"  update_id={update_id}")
    print(f"  webhook_url={cfg.webhook_url}")
    print(f"  log_file={cfg.log_file}")
    print(f"  first_status={status_first} second_status={status_second}")

    if status_first != 200 or status_second != 200:
        print("Error: webhook POST failed.", file=sys.stderr)
        print(f"  first_status={status_first} body={body_first}", file=sys.stderr)
        print(f"  second_status={status_second} body={body_second}", file=sys.stderr)
        return 1

    stats = {
        "accepted_count": 0,
        "duplicate_count": 0,
        "accepted_line": 0,
        "duplicate_line": 0,
        "evaluated_total": 0,
        "evaluated_true": 0,
        "evaluated_false": 0,
    }
    deadline = time.monotonic() + cfg.max_wait
    observed_lines: list[str] = []
    while time.monotonic() < deadline:
        cursor, chunk = read_new_lines(cfg.log_file, cursor)
        if chunk:
            observed_lines.extend(chunk)
        stats = collect_stats(observed_lines, update_id)
        if stats["accepted_count"] >= 1 and stats["duplicate_count"] >= 1:
            break
        time.sleep(1)

    if stats["accepted_count"] < 1 or stats["duplicate_count"] < 1:
        print(
            f"Error: expected dedup events were not observed within {cfg.max_wait}s.",
            file=sys.stderr,
        )
        print(
            "  update_accepted="
            f"{stats['accepted_count']} duplicate_detected={stats['duplicate_count']}",
            file=sys.stderr,
        )
        print(f"  update_id={update_id}", file=sys.stderr)
        print_relevant_tail(observed_lines, update_id)
        return 1

    if stats["accepted_line"] >= stats["duplicate_line"]:
        print(f"Error: unexpected dedup event order for update_id={update_id}.", file=sys.stderr)
        print(f"  line_update_accepted={stats['accepted_line']}", file=sys.stderr)
        print(f"  line_duplicate_detected={stats['duplicate_line']}", file=sys.stderr)
        return 1

    if stats["evaluated_total"] > 0 and (
        stats["evaluated_true"] < 1 or stats["evaluated_false"] < 1
    ):
        print(
            "Warning: dedup evaluated events were observed but did not include both duplicate states.",
            file=sys.stderr,
        )
        print(
            "  evaluated_total="
            f"{stats['evaluated_total']} duplicate_true={stats['evaluated_true']} "
            f"duplicate_false={stats['evaluated_false']}",
            file=sys.stderr,
        )

    print("Dedup probe passed.")
    print(f"  update_accepted={stats['accepted_count']}")
    print(f"  duplicate_detected={stats['duplicate_count']}")
    print(f"  evaluated_total={stats['evaluated_total']}")
    print(f"  evaluated_duplicate_false={stats['evaluated_false']}")
    print(f"  evaluated_duplicate_true={stats['evaluated_true']}")
    print("  order_ok=true (accepted before duplicate)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
