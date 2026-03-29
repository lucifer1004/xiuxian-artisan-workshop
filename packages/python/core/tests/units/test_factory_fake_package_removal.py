"""Removal guards for deleted core test factory and fake helper surfaces."""

from __future__ import annotations

from pathlib import Path


def _core_tests_root() -> Path:
    return Path(__file__).resolve().parents[1]


def test_factory_helper_modules_are_deleted() -> None:
    factories_dir = _core_tests_root() / "factories"
    assert not (factories_dir / "__init__.py").exists()
    assert not (factories_dir / "core_factories.py").exists()
    assert not (factories_dir / "mcp_factory.py").exists()
    assert not (factories_dir / "router_factories.py").exists()
    assert not (factories_dir / "skill_metadata_factory.py").exists()


def test_fake_helper_modules_are_deleted() -> None:
    fakes_dir = _core_tests_root() / "fakes"
    assert not (fakes_dir / "__init__.py").exists()
    assert not (fakes_dir / "fake_inference.py").exists()
    assert not (fakes_dir / "fake_mcp_server.py").exists()
    assert not (fakes_dir / "fake_registry.py").exists()
    assert not (fakes_dir / "fake_vectorstore.py").exists()
