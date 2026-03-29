"""Removal guard for the deleted local vector bridge module."""

from __future__ import annotations

import importlib.util

import xiuxian_foundation.bridge as bridge_module


def test_rust_vector_module_is_removed() -> None:
    assert importlib.util.find_spec("xiuxian_foundation.bridge.rust_vector") is None


def test_bridge_root_keeps_vector_bindings_absent() -> None:
    exported = set(dir(bridge_module))
    assert "RustVectorStore" not in exported
    assert "get_vector_store" not in exported
    assert "RUST_AVAILABLE" not in exported
