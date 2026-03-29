from __future__ import annotations

import pytest


def test_reactive_skill_watcher_is_removed() -> None:
    from xiuxian_core.kernel.watcher import ReactiveSkillWatcher

    with pytest.raises(
        RuntimeError, match="Python hot reload and file watcher support have been removed"
    ):
        ReactiveSkillWatcher(indexer=None)


def test_kernel_reload_skill_is_removed(tmp_path) -> None:
    from xiuxian_core.kernel.engine import Kernel

    kernel = Kernel(project_root=tmp_path)

    with pytest.raises(RuntimeError, match="Python skill reload has been removed"):
        import asyncio

        asyncio.run(kernel.reload_skill("git"))
