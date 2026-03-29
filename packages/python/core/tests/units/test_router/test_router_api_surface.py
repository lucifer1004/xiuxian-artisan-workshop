"""Public API surface tests for xiuxian_core.router exports."""

from __future__ import annotations


def test_router_exports_explicit_command_router_only() -> None:
    import xiuxian_core.router as router

    assert hasattr(router, "SearchCache")
    assert not hasattr(router, "SkillIndexer")
    assert not hasattr(router, "OmniRouter")
    assert not hasattr(router, "HybridSearch")
    assert not hasattr(router, "HiveRouter")
    assert not hasattr(router, "IntentSniffer")
