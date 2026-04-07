from __future__ import annotations

import importlib.util
import sys
import urllib.error
from email.message import Message
from pathlib import Path


class _FakeResponse:
    def __init__(self, status: int, headers: dict[str, str] | None = None) -> None:
        self.status = status
        self.headers = Message()
        for key, value in (headers or {}).items():
            self.headers[key] = value

    def read(self) -> bytes:
        return b""

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc, tb) -> bool:
        return False


def _load_module():
    script_path = Path(__file__).resolve().with_name("check_wendao_gateway_health.py")
    module_name = "test_check_wendao_gateway_health_module"
    spec = importlib.util.spec_from_file_location(module_name, script_path)
    assert spec is not None
    assert spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    sys.modules[module_name] = module
    spec.loader.exec_module(module)
    return module


def test_is_gateway_healthy_accepts_matching_health_and_flight_probe(tmp_path) -> None:
    module = _load_module()
    pidfile = tmp_path / "wendao.pid"
    pidfile.write_text("4321\n", encoding="utf-8")

    def _fake_open(request_or_url, timeout: float):
        assert timeout == 2.0
        if isinstance(request_or_url, str):
            return _FakeResponse(200, {"x-wendao-process-id": "4321"})
        raise urllib.error.HTTPError(
            request_or_url.full_url,
            400,
            "Bad Request",
            hdrs=None,
            fp=None,
        )

    healthy, message = module.is_gateway_healthy(
        host="127.0.0.1",
        port=9517,
        pidfile=pidfile,
        timeout_secs=2.0,
        opener=_fake_open,
    )

    assert healthy is True
    assert message == "healthy"


def test_is_gateway_healthy_rejects_mismatched_process_id(tmp_path) -> None:
    module = _load_module()
    pidfile = tmp_path / "wendao.pid"
    pidfile.write_text("4321\n", encoding="utf-8")

    def _fake_open(request_or_url, timeout: float):
        assert timeout == 2.0
        if isinstance(request_or_url, str):
            return _FakeResponse(200, {"x-wendao-process-id": "9999"})
        return _FakeResponse(400)

    healthy, message = module.is_gateway_healthy(
        host="127.0.0.1",
        port=9517,
        pidfile=pidfile,
        timeout_secs=2.0,
        opener=_fake_open,
    )

    assert healthy is False
    assert "does not match pidfile" in message


def test_is_gateway_healthy_rejects_unexpected_flight_status(tmp_path) -> None:
    module = _load_module()
    pidfile = tmp_path / "wendao.pid"
    pidfile.write_text("4321\n", encoding="utf-8")

    def _fake_open(request_or_url, timeout: float):
        assert timeout == 2.0
        if isinstance(request_or_url, str):
            return _FakeResponse(200, {"x-wendao-process-id": "4321"})
        return _FakeResponse(503)

    healthy, message = module.is_gateway_healthy(
        host="127.0.0.1",
        port=9517,
        pidfile=pidfile,
        timeout_secs=2.0,
        opener=_fake_open,
    )

    assert healthy is False
    assert "expected HTTP 400" in message
