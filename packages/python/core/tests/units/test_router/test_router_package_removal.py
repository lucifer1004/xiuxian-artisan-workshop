from __future__ import annotations

import asyncio
import importlib

import pytest

from xiuxian_core.kernel.engine import get_kernel, reset_kernel


def test_router_package_no_longer_exports_local_router_stack() -> None:
    import xiuxian_core.router as router

    assert hasattr(router, "SearchCache")
    assert not hasattr(router, "SkillIndexer")
    assert not hasattr(router, "OmniRouter")
    assert not hasattr(router, "HybridSearch")
    assert not hasattr(router, "HiveRouter")
    assert not hasattr(router, "IntentSniffer")


def test_router_legacy_modules_are_deleted() -> None:
    for module_name in (
        "xiuxian_core.router.indexer",
        "xiuxian_core.router.main",
        "xiuxian_core.router.hive",
        "xiuxian_core.router.router",
        "xiuxian_core.router.sniffer",
        "xiuxian_core.router.hybrid_search",
        "xiuxian_core.router.query_intent",
        "xiuxian_core.router.query_normalizer",
        "xiuxian_core.router.skill_relationships",
        "xiuxian_core.router.translate",
    ):
        with pytest.raises(ModuleNotFoundError):
            importlib.import_module(module_name)


def test_kernel_router_related_properties_fail_fast() -> None:
    reset_kernel()
    kernel = get_kernel(reset=True)

    with pytest.raises(RuntimeError, match="Python local router has been removed"):
        _ = kernel.router

    with pytest.raises(RuntimeError, match="Python semantic cortex has been removed"):
        asyncio.run(kernel.build_cortex())

    with pytest.raises(RuntimeError, match="Python local sniffer has been removed"):
        _ = kernel.sniffer

    with pytest.raises(RuntimeError, match="Python local sniffer has been removed"):
        asyncio.run(kernel.load_sniffer_rules())
