"""Removal guards for deleted local skill-host test fixtures."""

from __future__ import annotations

from pathlib import Path


def _core_tests_root() -> Path:
    return Path(__file__).resolve().parents[1]


def test_local_skill_host_fixture_modules_are_deleted() -> None:
    fixtures_dir = _core_tests_root() / "fixtures"
    assert not (fixtures_dir / "mocks.py").exists()
    assert not (fixtures_dir / "core_fixtures.py").exists()
    assert not (fixtures_dir / "skills_data.py").exists()


def test_local_skill_loader_example_test_is_deleted() -> None:
    assert not (_core_tests_root() / "units" / "test_testing_layers_example.py").exists()
