from __future__ import annotations

from pathlib import Path


def test_di_module_is_deleted() -> None:
    di_path = Path(__file__).resolve().parents[3] / "src/xiuxian_foundation/api/di.py"
    assert not di_path.exists()
