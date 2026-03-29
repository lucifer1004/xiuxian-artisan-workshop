"""Removal tests for legacy MCP schema compatibility surfaces."""

from __future__ import annotations

import importlib
import importlib.util


def test_mcp_api_modules_removed() -> None:
    importlib.invalidate_caches()
    assert importlib.util.find_spec("xiuxian_foundation.api.mcp_schema") is None
    assert importlib.util.find_spec("xiuxian_foundation.api.mcp_core_compat") is None
