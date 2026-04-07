#!/usr/bin/env python3
"""Check Wendao gateway readiness with PID ownership and Flight-route validation."""

from __future__ import annotations

import argparse
import sys
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any, Callable

GATEWAY_PROCESS_ID_HEADER = "x-wendao-process-id"
EXPECTED_HEALTH_STATUS = 200
EXPECTED_FLIGHT_STATUS = 400

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


def _flight_probe_status(url: str, timeout_secs: float, opener: Opener) -> int | None:
    request = urllib.request.Request(url, data=b"", method="POST")
    try:
        with opener(request, timeout=timeout_secs) as response:
            return getattr(response, "status", None)
    except urllib.error.HTTPError as error:
        return error.code
    except urllib.error.URLError:
        return None


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

    flight_url = f"http://{host}:{port}/arrow.flight.protocol.FlightService/GetFlightInfo"
    flight_status = _flight_probe_status(flight_url, timeout_secs, opener)
    if flight_status != EXPECTED_FLIGHT_STATUS:
        return (
            False,
            "Flight GetFlightInfo probe did not return the expected HTTP 400 "
            f"({flight_status if flight_status is not None else 'unreachable'})",
        )

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
