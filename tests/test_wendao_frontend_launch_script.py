from __future__ import annotations

from pathlib import Path


def test_wendao_frontend_launch_script_bootstraps_checkout_and_runs_rspack() -> None:
    script = Path(__file__).resolve().parents[1] / "scripts/channel/wendao-frontend-launch.sh"
    content = script.read_text(encoding="utf-8")

    assert (
        'managed_materialize_git_repo "$FRONTEND_DIR" "$REPO_URL" "" "wendao-frontend checkout"'
        in content
    )
    assert (
        'RUNTIME_DIR="${WENDAO_FRONTEND_RUNTIME_DIR:-$PROJECT_RUNTIME_ROOT/wendao-frontend}"'
        in content
    )
    assert 'PIDFILE="${WENDAO_FRONTEND_PIDFILE:-$RUNTIME_DIR/wendao-frontend.pid}"' in content
    assert "npm ci" in content
    assert 'managed_cleanup_pidfile_process "$PIDFILE" wendao-frontend "rspack-node"' in content
    assert 'managed_cleanup_listener "$PORT" wendao-frontend "rspack-node"' in content
    assert 'exec ./node_modules/.bin/rspack dev --host "$HOST" --port "$PORT"' in content
    assert 'managed_write_pidfile "$PIDFILE" "$WENDAO_FRONTEND_CHILD_PID"' in content
