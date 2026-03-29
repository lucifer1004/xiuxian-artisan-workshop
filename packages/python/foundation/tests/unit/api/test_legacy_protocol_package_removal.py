"""Removal tests for deleted legacy protocol compatibility surfaces."""

from __future__ import annotations

from pathlib import Path


def test_legacy_protocol_api_modules_are_absent() -> None:
    api_dir = Path(__file__).resolve().parents[3] / "src" / "xiuxian_foundation" / "api"
    expected = {
        "__init__.py",
        "agent_schema.py",
        "api_key.py",
        "decorators.py",
        "di.py",
        "execution.py",
        "handlers.py",
        "link_graph_policy_schema.py",
        "link_graph_schema.py",
        "link_graph_search_options_schema.py",
        "link_graph_stats_cache_schema.py",
        "link_graph_valkey_cache_schema.py",
        "protocols.py",
        "response_payloads.py",
        "schema.py",
        "schema_locator.py",
        "schema_provider.py",
        "tool_context.py",
        "types.py",
    }
    assert {path.name for path in api_dir.glob("*.py")} == expected
