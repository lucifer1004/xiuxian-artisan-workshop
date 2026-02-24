#!/usr/bin/env python3
"""
Shared config fallback helpers for Telegram channel black-box scripts.

Resolution priority:
1) Explicit CLI / env values
2) User settings:   $PRJ_CONFIG_HOME/omni-dev-fusion/settings.yaml
3) System settings: <repo>/packages/conf/settings.yaml
4) Runtime log inference (when available)
"""

from __future__ import annotations

import importlib
import os
import re
import sys
from pathlib import Path

_SCRIPT_DIR = Path(__file__).resolve().parent
if str(_SCRIPT_DIR) not in sys.path:
    sys.path.insert(0, str(_SCRIPT_DIR))

load_sibling_module = importlib.import_module("module_loader").load_sibling_module

_log_io_module = load_sibling_module(
    module_name="log_io",
    file_name="log_io.py",
    caller_file=__file__,
    error_context="shared log I/O helpers",
)
_shared_read_log_tail_lines = _log_io_module.read_log_tail_lines

_ANSI_ESCAPE_RE = re.compile(r"\x1b\[[0-9;]*m")
_SESSION_KEY_RE = re.compile(r"\bsession_key\s*=\s*(?:\"|')?([-\d]+(?::[-\d]+){1,2})(?:\"|')?")
_PARTITION_MODE_RE = re.compile(
    r"\b(?:json_partition_mode|current_mode|requested_partition_mode)\s*=\s*"
    r"(?:\"|')?([A-Za-z0-9_-]+)(?:\"|')?"
)
_ENV_ASSIGNMENT_RE = re.compile(r"^\s*([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)$")
_USERNAME_TOKEN_RE = re.compile(r"\busername\s*=\s*(?:\"([^\"]*)\"|'([^']*)'|([^\s]+))")
_WEBHOOK_BIND_PORT_RE = re.compile(r":(\d{1,5})$")
_RUNTIME_LOG_TAIL_BYTES = 256 * 1024
DEFAULT_TELEGRAM_WEBHOOK_PORT = 18081


def _read_log_tail_lines(path: Path, tail_bytes: int = _RUNTIME_LOG_TAIL_BYTES) -> list[str]:
    """Read only the log tail to avoid loading large runtime logs into memory."""
    return _shared_read_log_tail_lines(path, tail_bytes=tail_bytes)


def _repo_root_from(start: Path) -> Path:
    for candidate in [start, *start.parents]:
        if (candidate / ".git").exists():
            return candidate
    return start


def _strip_inline_comment(value: str) -> str:
    in_single = False
    in_double = False
    out: list[str] = []
    for char in value:
        if char == "'" and not in_double:
            in_single = not in_single
            out.append(char)
            continue
        if char == '"' and not in_single:
            in_double = not in_double
            out.append(char)
            continue
        if char == "#" and not in_single and not in_double:
            break
        out.append(char)
    return "".join(out).strip()


def _unquote(value: str) -> str:
    v = value.strip()
    if len(v) >= 2 and ((v[0] == "'" and v[-1] == "'") or (v[0] == '"' and v[-1] == '"')):
        return v[1:-1].strip()
    return v


def _read_telegram_key_from_yaml(path: Path, key: str) -> str | None:
    if not path.exists():
        return None

    lines = path.read_text(encoding="utf-8", errors="ignore").splitlines()
    in_telegram = False
    telegram_indent = 0
    key_re = re.compile(rf"^\s*{re.escape(key)}\s*:\s*(.*)$")

    for raw in lines:
        line = raw.rstrip("\n")
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue

        indent = len(line) - len(line.lstrip(" "))
        if not in_telegram:
            if re.match(r"^\s*telegram\s*:\s*$", line):
                in_telegram = True
                telegram_indent = indent
            continue

        if indent <= telegram_indent:
            break

        match = key_re.match(line)
        if not match:
            continue
        payload = _unquote(_strip_inline_comment(match.group(1)))
        if payload in {"", "null", "None", "~"}:
            return ""
        return payload

    return None


def _settings_candidates(repo_root: Path) -> list[Path]:
    # Prefer explicit runtime override and then default PRJ spec location.
    prj_config_home = Path(os.environ.get("PRJ_CONFIG_HOME", str(repo_root / ".config")))
    user_settings = prj_config_home / "omni-dev-fusion" / "settings.yaml"
    system_settings = repo_root / "packages" / "conf" / "settings.yaml"
    return [user_settings, system_settings]


def _parse_yaml_scalar_list(raw: str) -> list[str]:
    payload = _strip_inline_comment(raw).strip()
    if payload in {"", "null", "None", "~"}:
        return []
    if payload.startswith("[") and payload.endswith("]"):
        inner = payload[1:-1].strip()
        if not inner:
            return []
        return [_unquote(item.strip()) for item in inner.split(",") if item.strip()]
    return _split_csv_entries(_unquote(payload))


def _read_telegram_acl_allow_users(path: Path) -> list[str] | None:
    if not path.exists():
        return None

    lines = path.read_text(encoding="utf-8", errors="ignore").splitlines()
    in_telegram = False
    telegram_indent = 0
    in_acl = False
    acl_indent = 0
    in_allow = False
    allow_indent = 0
    collecting_block = False
    users_key_indent = 0
    block_values: list[str] = []

    for raw in lines:
        line = raw.rstrip("\n")
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        indent = len(line) - len(line.lstrip(" "))

        if not in_telegram:
            if re.match(r"^\s*telegram\s*:\s*$", line):
                in_telegram = True
                telegram_indent = indent
            continue

        if indent <= telegram_indent:
            break

        if in_allow and collecting_block:
            if indent <= users_key_indent:
                return block_values
            block_match = re.match(r"^\s*-\s*(.*)$", line)
            if block_match:
                value = _unquote(_strip_inline_comment(block_match.group(1)))
                if value and value not in {"null", "None", "~"}:
                    block_values.append(value)
            continue

        if in_allow:
            if indent <= allow_indent:
                in_allow = False
                collecting_block = False
            else:
                users_match = re.match(r"^\s*users\s*:\s*(.*)$", line)
                if users_match:
                    users_key_indent = indent
                    payload = users_match.group(1).strip()
                    if payload == "":
                        collecting_block = True
                        block_values = []
                        continue
                    return _parse_yaml_scalar_list(payload)
            continue

        if in_acl and indent <= acl_indent:
            in_acl = False

        if in_acl:
            if re.match(r"^\s*allow\s*:\s*$", line):
                in_allow = True
                allow_indent = indent
            continue

        if re.match(r"^\s*acl\s*:\s*$", line):
            in_acl = True
            acl_indent = indent

    if collecting_block:
        return block_values
    return None


def _split_csv_entries(raw: str) -> list[str]:
    entries: list[str] = []
    for item in raw.split(","):
        token = item.strip()
        if token:
            entries.append(token)
    return entries


def _group_env_file(repo_root: Path) -> Path:
    explicit = os.environ.get("OMNI_TEST_GROUP_ENV_FILE", "").strip()
    if explicit:
        return Path(explicit)
    return repo_root / ".run" / "config" / "agent-channel-groups.env"


def _dotenv_file(repo_root: Path) -> Path:
    explicit = os.environ.get("OMNI_TEST_DOTENV_FILE", "").strip()
    if explicit:
        return Path(explicit)
    return repo_root / ".env"


def _read_env_profile(path: Path) -> dict[str, str]:
    if not path.exists():
        return {}

    values: dict[str, str] = {}
    for raw in path.read_text(encoding="utf-8", errors="ignore").splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        match = _ENV_ASSIGNMENT_RE.match(line)
        if not match:
            continue
        key = match.group(1).strip()
        payload = _unquote(_strip_inline_comment(match.group(2)))
        values[key] = payload
    return values


def group_profile_value(key: str, repo_root: Path | None = None) -> str | None:
    """Resolve value from process env first, then group profile env file."""
    in_process = os.environ.get(key, "").strip()
    if in_process:
        return in_process

    root = repo_root or _repo_root_from(Path(__file__).resolve())
    profile_values = _read_env_profile(_group_env_file(root))
    value = profile_values.get(key, "").strip()
    if not value or value in {"null", "None", "~"}:
        return None
    return value


def group_profile_int(key: str, repo_root: Path | None = None) -> int | None:
    raw = group_profile_value(key, repo_root)
    if raw is None:
        return None
    try:
        return int(raw)
    except ValueError as error:
        raise ValueError(f"{key} must be an integer, got '{raw}'.") from error


def group_profile_chat_ids(repo_root: Path | None = None) -> tuple[int, ...]:
    ordered: list[int] = []
    for key in ("OMNI_TEST_CHAT_ID", "OMNI_TEST_CHAT_B", "OMNI_TEST_CHAT_C"):
        value = group_profile_int(key, repo_root)
        if value is None:
            continue
        if value not in ordered:
            ordered.append(value)
    return tuple(ordered)


def env_or_dotenv_value(key: str, repo_root: Path | None = None) -> str | None:
    in_process = os.environ.get(key, "").strip()
    if in_process:
        return in_process

    root = repo_root or _repo_root_from(Path(__file__).resolve())
    dotenv_values = _read_env_profile(_dotenv_file(root))
    value = dotenv_values.get(key, "").strip()
    if not value or value in {"null", "None", "~"}:
        return None
    return value


def telegram_webhook_secret_token(repo_root: Path | None = None) -> str | None:
    root = repo_root or _repo_root_from(Path(__file__).resolve())
    secret = env_or_dotenv_value("TELEGRAM_WEBHOOK_SECRET", root)
    if secret:
        return secret

    for settings_path in _settings_candidates(root):
        configured = _read_telegram_key_from_yaml(settings_path, "webhook_secret_token")
        if configured is None:
            continue
        normalized = configured.strip()
        if normalized and normalized not in {"null", "None", "~"}:
            return normalized
    return None


def _parse_port(raw: str, *, source: str) -> int:
    token = raw.strip()
    if not token:
        raise ValueError(f"{source} webhook port cannot be empty.")
    try:
        port = int(token)
    except ValueError as error:
        raise ValueError(f"{source} webhook port must be an integer, got '{raw}'.") from error
    if port <= 0 or port > 65535:
        raise ValueError(f"{source} webhook port out of range: {port}.")
    return port


def _port_from_bind(bind: str) -> int | None:
    match = _WEBHOOK_BIND_PORT_RE.search(bind.strip())
    if not match:
        return None
    raw_port = match.group(1)
    try:
        return _parse_port(raw_port, source="settings")
    except ValueError:
        return None


def telegram_webhook_bind(repo_root: Path | None = None) -> str | None:
    explicit_bind = os.environ.get("WEBHOOK_BIND", "").strip()
    if explicit_bind:
        return explicit_bind

    root = repo_root or _repo_root_from(Path(__file__).resolve())
    for settings_path in _settings_candidates(root):
        configured = _read_telegram_key_from_yaml(settings_path, "webhook_bind")
        if configured is None:
            continue
        normalized = configured.strip()
        if normalized and normalized not in {"null", "None", "~"}:
            return normalized
    return None


def telegram_webhook_port(repo_root: Path | None = None) -> int:
    explicit_port = os.environ.get("WEBHOOK_PORT", "").strip()
    if explicit_port:
        return _parse_port(explicit_port, source="WEBHOOK_PORT")

    bind = telegram_webhook_bind(repo_root)
    if bind:
        parsed = _port_from_bind(bind)
        if parsed is not None:
            return parsed
    return DEFAULT_TELEGRAM_WEBHOOK_PORT


def default_telegram_webhook_url(repo_root: Path | None = None) -> str:
    return f"http://127.0.0.1:{telegram_webhook_port(repo_root)}/telegram/webhook"


def normalize_telegram_session_partition_mode(raw: str | None) -> str | None:
    if raw is None:
        return None
    token = raw.strip().lower()
    if not token:
        return None
    token = token.replace("-", "_")
    if token in {"chat", "channel"}:
        return "chat"
    if token in {"chat_user", "chatuser"}:
        return "chat_user"
    if token in {"user", "user_only", "useronly"}:
        return "user"
    if token in {"chat_thread_user", "chatthreaduser", "topic_user", "topicuser"}:
        return "chat_thread_user"
    return None


def telegram_session_partition_mode(repo_root: Path | None = None) -> str | None:
    root = repo_root or _repo_root_from(Path(__file__).resolve())
    env_mode = os.environ.get("OMNI_AGENT_TELEGRAM_SESSION_PARTITION", "").strip()
    normalized_env_mode = normalize_telegram_session_partition_mode(env_mode)
    if normalized_env_mode:
        return normalized_env_mode

    for settings_path in _settings_candidates(root):
        configured = _read_telegram_key_from_yaml(settings_path, "session_partition")
        if configured is None:
            continue
        normalized = normalize_telegram_session_partition_mode(configured)
        if normalized:
            return normalized
    return None


def session_partition_mode_from_runtime_log(log_file: Path) -> str | None:
    if not log_file.exists():
        return None

    lines = _read_log_tail_lines(log_file)
    for raw_line in reversed(lines):
        line = _ANSI_ESCAPE_RE.sub("", raw_line)
        if "Parsed message, forwarding to agent" in line:
            key_match = _SESSION_KEY_RE.search(line)
            if key_match:
                parts = key_match.group(1).split(":")
                if len(parts) == 3:
                    return "chat_thread_user"
                if len(parts) == 2:
                    return "chat_user"
                if len(parts) == 1:
                    return "chat"
        mode_match = _PARTITION_MODE_RE.search(line)
        if mode_match:
            normalized = normalize_telegram_session_partition_mode(mode_match.group(1))
            if normalized:
                return normalized

    _, _, inferred_thread = session_ids_from_runtime_log(log_file)
    if inferred_thread is not None:
        return "chat_thread_user"
    return None


def allowed_users_from_settings(repo_root: Path | None = None) -> list[str]:
    """Resolve `telegram.acl.allow.users` from merged settings (user overrides system)."""
    root = repo_root or _repo_root_from(Path(__file__).resolve())
    for settings_path in _settings_candidates(root):
        allowed_users = _read_telegram_acl_allow_users(settings_path)
        if allowed_users is not None:
            return allowed_users
    return []


def username_from_settings(repo_root: Path | None = None) -> str | None:
    for first in allowed_users_from_settings(repo_root):
        if first in {"*", "''", '""'}:
            return None
        return first
    return None


def username_from_runtime_log(log_file: Path) -> str | None:
    if not log_file.exists():
        return None
    for raw_line in reversed(_read_log_tail_lines(log_file)):
        line = _ANSI_ESCAPE_RE.sub("", raw_line)
        match = _USERNAME_TOKEN_RE.search(line)
        if not match:
            continue
        value = (match.group(1) or match.group(2) or match.group(3) or "").strip()
        if value and value not in {"*", "''", '""'}:
            return value
    return None


def session_ids_from_runtime_log(log_file: Path) -> tuple[int | None, int | None, int | None]:
    """Infer `(chat_id, user_id, thread_id)` from the latest `session_key` log token."""
    if not log_file.exists():
        return None, None, None

    last_session_key: str | None = None
    for raw_line in _read_log_tail_lines(log_file):
        line = _ANSI_ESCAPE_RE.sub("", raw_line)
        match = _SESSION_KEY_RE.search(line)
        if match:
            last_session_key = match.group(1)

    if not last_session_key:
        return None, None, None

    parts = last_session_key.split(":")
    if len(parts) == 2:
        return int(parts[0]), int(parts[1]), None
    if len(parts) == 3:
        return int(parts[0]), int(parts[2]), int(parts[1])
    return None, None, None
