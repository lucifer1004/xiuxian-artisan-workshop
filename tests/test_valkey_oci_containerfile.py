from __future__ import annotations

from pathlib import Path


def test_valkey_oci_containerfile_binds_portable_launch_contract() -> None:
    containerfile = Path(__file__).resolve().parents[1] / "oci/valkey/Containerfile"
    content = containerfile.read_text(encoding="utf-8")

    assert "ARG VALKEY_IMAGE=valkey/valkey:9.0-alpine" in content
    assert "RUN apk add --no-cache bash" in content
    assert "COPY scripts/channel/valkey-common.sh /usr/local/bin/valkey-common.sh" in content
    assert "COPY scripts/channel/valkey-runtime.sh /usr/local/bin/valkey-runtime.sh" in content
    assert "COPY scripts/channel/valkey-launch.sh /usr/local/bin/valkey-launch.sh" in content
    assert (
        "COPY scripts/channel/valkey-healthcheck.sh /usr/local/bin/valkey-healthcheck.sh" in content
    )
    assert "WORKDIR /data/valkey" in content
    assert "VALKEY_BIND=0.0.0.0" in content
    assert "VALKEY_DATA_DIR=/data/valkey" in content
    assert "VALKEY_PIDFILE=/run/valkey/valkey.pid" in content
    assert "VALKEY_PROTECTED_MODE=no" in content
    assert "VALKEY_DAEMONIZE=no" in content
    assert 'ENTRYPOINT ["/usr/local/bin/valkey-launch.sh"]' in content
    assert (
        'HEALTHCHECK --interval=5s --timeout=3s --start-period=15s --retries=10 CMD ["/usr/local/bin/valkey-healthcheck.sh"]'
        in content
    )
