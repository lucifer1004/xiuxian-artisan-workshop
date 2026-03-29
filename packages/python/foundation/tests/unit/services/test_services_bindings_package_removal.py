"""Removal guards for deleted local service-side binding helpers."""

from __future__ import annotations

import importlib.util

import xiuxian_foundation.services as services


def test_deleted_service_binding_modules_are_absent() -> None:
    assert importlib.util.find_spec("xiuxian_foundation.services.index_dimension") is None
    assert importlib.util.find_spec("xiuxian_foundation.services.router_scores") is None


def test_services_root_no_longer_exports_dimension_helpers() -> None:
    exported = set(dir(services))
    assert "EmbeddingDimensionStatus" not in exported
    assert "ensure_embedding_signature_written" not in exported
    assert "get_embedding_dimension_status" not in exported
    assert "get_embedding_signature_path" not in exported
