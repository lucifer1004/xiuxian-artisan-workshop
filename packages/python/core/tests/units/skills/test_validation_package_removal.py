"""Removal guard for deleted local skills validation module."""

from __future__ import annotations

import importlib.util


def test_skills_validation_module_is_removed() -> None:
    assert importlib.util.find_spec("xiuxian_core.skills.validation") is None
