from __future__ import annotations

from pathlib import Path


def test_valkey_process_nix_bootstraps_dirs_and_waits_for_readiness() -> None:
    process_nix = Path(__file__).resolve().parents[1] / "nix/modules/process.nix"
    content = process_nix.read_text(encoding="utf-8")

    assert "mkdir -p ${valkeyRuntimeDir} ${valkeyDataDir}" in content
    assert "rm -f ${valkeyPidFile}" in content
    assert "--pidfile ${valkeyPidFile}" in content
    assert "exec valkey-server .config/xiuxian-artisan-workshop/valkey.conf --tcp-backlog 128" in content
    assert "valkey-cli -u ${valkeyUrl} info server" in content
    assert "process_id" in content
    assert "valkey-cli -u ${valkeyUrl} ping" in content
    assert "initial_delay_seconds = 5;" in content
    assert "timeout_seconds = 2;" in content
    assert "failure_threshold = 30;" in content
