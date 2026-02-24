"""
Async execution helpers for sync call sites.
"""

from __future__ import annotations

import asyncio
from concurrent.futures import ThreadPoolExecutor
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from collections.abc import Coroutine


def run_async_blocking[T](coro: Coroutine[Any, Any, T]) -> T:
    """Run a coroutine from sync code and return its result.

    Fast path:
    - If no event loop is running in this thread, execute directly via asyncio.run()
      to avoid per-call thread-pool overhead in CLI hot paths.

    Compatibility path:
    - If a loop is already running (e.g. tests or embedded runtimes), execute in
      a dedicated worker thread with its own event loop.
    """
    try:
        asyncio.get_running_loop()
    except RuntimeError:
        return asyncio.run(coro)

    with ThreadPoolExecutor(max_workers=1) as executor:
        future = executor.submit(_run_coro_in_thread, coro)
        return future.result()


def _run_coro_in_thread[T](coro: Coroutine[Any, Any, T]) -> T:
    """Run a coroutine in this thread with a new event loop. Used by run_async_blocking."""
    return asyncio.run(coro)


__all__ = [
    "run_async_blocking",
]
