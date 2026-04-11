#!/usr/bin/env python3
"""Check Wendao gateway readiness with PID ownership and the health endpoint contract."""

from __future__ import annotations

import argparse
import json
import sys
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any, Callable

GATEWAY_PROCESS_ID_HEADER = "x-wendao-process-id"
EXPECTED_HEALTH_STATUS = 200
EXPECTED_HEALTH_SERVICE = "wendao-gateway"

Opener = Callable[[Any, float], Any]


def read_expected_pid(pidfile: Path) -> int:
    contents = pidfile.read_text(encoding="utf-8").strip()
    if not contents:
        raise ValueError(f"pidfile is empty: {pidfile}")
    try:
        return int(contents)
    except ValueError as error:
        raise ValueError(f"pidfile does not contain a valid process id: {pidfile}") from error


def _normalize_process_id(raw_value: str | None) -> str:
    return (raw_value or "").strip()


def _parse_health_payload(raw_payload: bytes) -> tuple[bool, str]:
    try:
        payload = json.loads(raw_payload.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        return False, f"health endpoint returned invalid json: {error}"

    if not isinstance(payload, dict):
        return False, "health endpoint returned a non-object payload"

    if payload.get("service") != EXPECTED_HEALTH_SERVICE:
        return (
            False,
            "health endpoint reported an unexpected service "
            f"({payload.get('service')!r} != {EXPECTED_HEALTH_SERVICE!r})",
        )

    if payload.get("ready") is not True:
        return False, f"health endpoint did not report ready=true: {payload!r}"

    return True, "ok"


def is_gateway_healthy(
    *,
    host: str,
    port: int,
    pidfile: Path,
    timeout_secs: float,
    opener: Opener = urllib.request.urlopen,
) -> tuple[bool, str]:
    try:
        expected_pid = read_expected_pid(pidfile)
    except OSError as error:
        return False, f"failed to read pidfile {pidfile}: {error}"
    except ValueError as error:
        return False, str(error)

    health_url = f"http://{host}:{port}/api/health"
    try:
        with opener(health_url, timeout=timeout_secs) as response:
            health_status = getattr(response, "status", None)
            actual_pid = _normalize_process_id(response.headers.get(GATEWAY_PROCESS_ID_HEADER))
            raw_payload = response.read()
    except urllib.error.HTTPError as error:
        return False, f"health endpoint returned HTTP {error.code}: {health_url}"
    except urllib.error.URLError as error:
        return False, f"health endpoint unreachable: {health_url} ({error.reason})"

    if health_status != EXPECTED_HEALTH_STATUS:
        return False, f"health endpoint returned HTTP {health_status}: {health_url}"

    if actual_pid != str(expected_pid):
        return (
            False,
            "health endpoint process id does not match pidfile "
            f"({actual_pid or 'missing'} != {expected_pid})",
        )

    payload_ok, payload_message = _parse_health_payload(raw_payload)
    if not payload_ok:
        return False, payload_message

    return True, "healthy"


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Check Wendao gateway readiness against health and Flight routes"
    )
    parser.add_argument("--host", default="127.0.0.1", help="Gateway host")
    parser.add_argument("--port", type=int, required=True, help="Gateway port")
    parser.add_argument(
        "--pidfile",
        type=Path,
        required=True,
        help="Pidfile written by the Wendao gateway launcher",
    )
    parser.add_argument(
        "--timeout-secs",
        type=float,
        default=2.0,
        help="Per-request timeout in seconds",
    )
    args = parser.parse_args()

    healthy, message = is_gateway_healthy(
        host=args.host,
        port=args.port,
        pidfile=args.pidfile,
        timeout_secs=args.timeout_secs,
    )
    if healthy:
        print(message)
        return 0
    print(f"Error: {message}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
