"""Self-contained runtime compatibility helpers for the `xiuxian_*` namespace."""

from __future__ import annotations

import os
import subprocess
from pathlib import Path


def get_project_root() -> Path:
    """Resolve the project root without importing legacy runtime packages."""
    prj_root = os.environ.get("PRJ_ROOT")
    if prj_root:
        return Path(prj_root).expanduser().resolve()

    try:
        result = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"],
            capture_output=True,
            text=True,
            timeout=5,
            check=False,
        )
    except Exception as exc:
        raise RuntimeError("Cannot resolve project root") from exc

    if result.returncode == 0 and result.stdout.strip():
        return Path(result.stdout.strip()).resolve()

    cwd = Path.cwd().resolve()
    if (cwd / ".git").exists():
        return cwd

    raise RuntimeError("Cannot resolve project root")


def clear_project_root_cache() -> None:
    """Compatibility no-op for legacy tests that reset cached git root state."""
    return None


__all__ = ["clear_project_root_cache", "get_project_root"]
