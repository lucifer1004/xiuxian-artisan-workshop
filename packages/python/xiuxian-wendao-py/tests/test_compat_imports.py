from __future__ import annotations

from importlib.util import find_spec


def test_compat_package_is_removed() -> None:
    assert find_spec("xiuxian_wendao_py.compat") is None
