from __future__ import annotations

from pathlib import Path


def test_wendao_frontend_process_nix_delegates_to_scripts() -> None:
    process_nix = Path(__file__).resolve().parents[1] / "nix/modules/process.nix"
    content = process_nix.read_text(encoding="utf-8")

    assert 'wendaoFrontendRepoUrl = "https://github.com/tao3k/wendao-frontend.git";' in content
    assert '"wendao-frontend" = {' in content
    assert "export WENDAO_FRONTEND_MANAGED=1" in content
    assert (
        'export WENDAO_FRONTEND_RUNTIME_DIR="$RUNTIME_DIR/${wendaoFrontendRuntimeDirName}"'
        in content
    )
    assert (
        'export WENDAO_FRONTEND_PIDFILE="$RUNTIME_DIR/${wendaoFrontendRuntimeDirName}/${wendaoFrontendPidFilename}"'
        in content
    )
    assert (
        'export WENDAO_FRONTEND_STDOUT_LOG="$LOG_DIR/${wendaoFrontendStdoutLogFilename}"' in content
    )
    assert (
        'export WENDAO_FRONTEND_STDERR_LOG="$LOG_DIR/${wendaoFrontendStderrLogFilename}"' in content
    )
    assert 'bash "$ROOT_DIR/scripts/channel/wendao-frontend-launch.sh"' in content
    assert "exec = {" in content
    assert "RUNTIME_DIR=\"''${PRJ_RUNTIME_DIR:-$ROOT_DIR/.run}\"" in content
    assert "export WENDAO_FRONTEND_HOST=${wendaoFrontendHost}" in content
    assert "export WENDAO_FRONTEND_PORT=${toString wendaoFrontendPort}" in content
    assert (
        'export WENDAO_FRONTEND_PIDFILE="$RUNTIME_DIR/${wendaoFrontendRuntimeDirName}/${wendaoFrontendPidFilename}"'
        in content
    )
    assert 'bash "$ROOT_DIR/scripts/channel/wendao-frontend-healthcheck.sh" >/dev/null' in content
