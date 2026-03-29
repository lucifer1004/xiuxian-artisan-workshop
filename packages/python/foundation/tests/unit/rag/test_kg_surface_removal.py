"""Removal checks for deleted local KG snapshot helpers."""

from __future__ import annotations

import importlib

import pytest


def test_kg_recall_module_removed() -> None:
    with pytest.raises(ModuleNotFoundError):
        importlib.import_module("xiuxian_rag.fusion.kg_recall")


def test_fusion_config_no_longer_exposes_kg_snapshot_helpers() -> None:
    from xiuxian_rag.fusion import _config

    assert not hasattr(_config, "_load_kg")
    assert not hasattr(_config, "_save_kg")
