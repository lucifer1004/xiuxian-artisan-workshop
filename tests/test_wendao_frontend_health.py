from __future__ import annotations

import importlib.util
from pathlib import Path

MODULE_PATH = (
    Path(__file__).resolve().parents[1] / "scripts/channel/check_wendao_frontend_health.py"
)
MODULE_SPEC = importlib.util.spec_from_file_location("check_wendao_frontend_health", MODULE_PATH)
assert MODULE_SPEC is not None and MODULE_SPEC.loader is not None
MODULE = importlib.util.module_from_spec(MODULE_SPEC)
MODULE_SPEC.loader.exec_module(MODULE)
is_frontend_healthy = MODULE.is_frontend_healthy


class _Response:
    def __init__(self, status: int) -> None:
        self.status = status

    def read(self, _: int) -> bytes:
        return b"o"

    def __enter__(self) -> "_Response":
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        return None


def test_is_frontend_healthy_requires_live_pid_and_http_200(tmp_path: Path) -> None:
    pidfile = tmp_path / "wendao-frontend.pid"
    pidfile.write_text("4242\n", encoding="utf-8")

    healthy, message = is_frontend_healthy(
        host="127.0.0.1",
        port=9518,
        pidfile=pidfile,
        timeout_secs=2.0,
        opener=lambda url, timeout: _Response(200),
        pid_exists=lambda pid: pid == 4242,
        listener_pid_for_port=lambda port: 4242 if port == 9518 else None,
        process_command_for_pid=lambda pid: "rspack-node" if pid == 4242 else "",
    )

    assert healthy is True
    assert message == "healthy"


def test_is_frontend_healthy_rejects_dead_pid(tmp_path: Path) -> None:
    pidfile = tmp_path / "wendao-frontend.pid"
    pidfile.write_text("4242\n", encoding="utf-8")

    healthy, message = is_frontend_healthy(
        host="127.0.0.1",
        port=9518,
        pidfile=pidfile,
        timeout_secs=2.0,
        opener=lambda url, timeout: _Response(200),
        pid_exists=lambda pid: False,
        listener_pid_for_port=lambda port: None,
        process_command_for_pid=lambda pid: "",
    )

    assert healthy is False
    assert "not alive" in message


def test_is_frontend_healthy_accepts_live_listener_when_pidfile_missing(tmp_path: Path) -> None:
    pidfile = tmp_path / "wendao-frontend.pid"

    healthy, message = is_frontend_healthy(
        host="127.0.0.1",
        port=9518,
        pidfile=pidfile,
        timeout_secs=2.0,
        opener=lambda url, timeout: _Response(200),
        pid_exists=lambda pid: False,
        listener_pid_for_port=lambda port: 5151 if port == 9518 else None,
        process_command_for_pid=lambda pid: "rspack-node" if pid == 5151 else "",
    )

    assert healthy is True
    assert message == "healthy"


def test_is_frontend_healthy_rejects_unexpected_listener_owner(tmp_path: Path) -> None:
    pidfile = tmp_path / "wendao-frontend.pid"

    healthy, message = is_frontend_healthy(
        host="127.0.0.1",
        port=9518,
        pidfile=pidfile,
        timeout_secs=2.0,
        opener=lambda url, timeout: _Response(200),
        pid_exists=lambda pid: False,
        listener_pid_for_port=lambda port: 6161 if port == 9518 else None,
        process_command_for_pid=lambda pid: "python -m http.server" if pid == 6161 else "",
    )

    assert healthy is False
    assert "unexpected process" in message
