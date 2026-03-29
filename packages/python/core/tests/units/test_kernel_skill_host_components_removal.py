from __future__ import annotations

from pathlib import Path


def test_kernel_skill_host_components_are_removed() -> None:
    components_dir = (
        Path(__file__).resolve().parents[2] / "src" / "xiuxian_core" / "kernel" / "components"
    )
    py_files = sorted(path.name for path in components_dir.glob("*.py"))
    assert py_files == ["__init__.py"]
