"""Removal guards for deleted core test factory and fake helper surfaces."""

from __future__ import annotations

from pathlib import Path


def _core_tests_root() -> Path:
    return Path(__file__).resolve().parents[1]


def test_factory_helper_modules_are_deleted() -> None:
    factories_dir = _core_tests_root() / "factories"
    assert not any(factories_dir.glob("*.py"))


def test_fake_helper_modules_are_deleted() -> None:
    fakes_dir = _core_tests_root() / "fakes"
    assert not any(fakes_dir.glob("*.py"))
