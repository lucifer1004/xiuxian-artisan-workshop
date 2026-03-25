from __future__ import annotations

from pathlib import Path


def test_wendao_gateway_process_nix_checks_http_status_and_process_id_header() -> None:
    process_nix = Path(__file__).resolve().parents[1] / "nix/modules/process.nix"
    content = process_nix.read_text(encoding="utf-8")

    assert 'gatewayRuntimeDir = ".run/wendao-gateway";' in content
    assert 'gatewayPidFile = "${gatewayRuntimeDir}/wendao.pid";' in content
    assert "PIDFILE=${gatewayPidFile}" in content
    assert 'curl -sS --max-time 2 -D - -o /dev/null "http://127.0.0.1:$PORT/api/health"' in content
    assert 'HTTP_STATUS="$(printf' in content
    assert 'tolower($1) == "x-wendao-process-id"' in content
    assert '[ "$HTTP_STATUS" != "200" ]' in content
