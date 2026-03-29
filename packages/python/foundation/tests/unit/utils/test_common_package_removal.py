from __future__ import annotations

from pathlib import Path


def test_common_module_is_deleted() -> None:
    common_path = Path(__file__).resolve().parents[3] / "src/xiuxian_foundation/utils/common.py"
    assert not common_path.exists()


def test_foundation_no_longer_exports_project_root_wrapper() -> None:
    import xiuxian_foundation

    assert not hasattr(xiuxian_foundation, "project_root")
