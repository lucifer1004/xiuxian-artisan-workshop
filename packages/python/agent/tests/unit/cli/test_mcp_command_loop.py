"""Tests for MCP command SSE loop initialization helpers."""

from __future__ import annotations

import asyncio
import threading

import pytest

from omni.agent.cli.commands.mcp import _initialize_handler_on_server_loop


def _start_loop_in_thread() -> tuple[asyncio.AbstractEventLoop, threading.Thread]:
    loop = asyncio.new_event_loop()
    ready = threading.Event()

    def _runner() -> None:
        asyncio.set_event_loop(loop)
        ready.set()
        loop.run_forever()
        loop.close()

    thread = threading.Thread(target=_runner, daemon=True)
    thread.start()
    assert ready.wait(timeout=1.0)
    return loop, thread


def _stop_loop_thread(loop: asyncio.AbstractEventLoop, thread: threading.Thread) -> None:
    loop.call_soon_threadsafe(loop.stop)
    thread.join(timeout=2.0)


def test_initialize_handler_uses_target_server_loop() -> None:
    class _Handler:
        loop_seen: asyncio.AbstractEventLoop | None = None

        async def initialize(self) -> None:
            self.loop_seen = asyncio.get_running_loop()

    loop, thread = _start_loop_in_thread()
    try:
        handler = _Handler()
        _initialize_handler_on_server_loop(handler, loop, timeout_seconds=2.0)
        assert handler.loop_seen is loop
    finally:
        _stop_loop_thread(loop, thread)


def test_initialize_handler_times_out() -> None:
    class _SlowHandler:
        async def initialize(self) -> None:
            await asyncio.sleep(10)

    loop, thread = _start_loop_in_thread()
    try:
        with pytest.raises(TimeoutError):
            _initialize_handler_on_server_loop(_SlowHandler(), loop, timeout_seconds=0.01)
    finally:
        _stop_loop_thread(loop, thread)
