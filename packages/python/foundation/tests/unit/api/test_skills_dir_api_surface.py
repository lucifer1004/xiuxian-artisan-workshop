"""API surface tests for retained skill directory helpers."""

from __future__ import annotations

from importlib.util import find_spec

from xiuxian_foundation.config.dirs import get_skills_dir


def test_skills_dir_uses_function_api() -> None:
    path = get_skills_dir()
    assert path.name == "skills"


def test_legacy_skills_module_is_removed() -> None:
    assert find_spec("xiuxian_foundation.config.skills") is None
