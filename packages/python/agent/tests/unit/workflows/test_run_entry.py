"""Tests for run_entry decommissioned Python runtime entrypoints."""

from __future__ import annotations

import pytest

from omni.agent.workflows.run_entry import execute_task_via_kernel, execute_task_with_session


@pytest.mark.asyncio
async def test_execute_task_via_kernel_raises_decommissioned_error() -> None:
    with pytest.raises(RuntimeError, match="decommissioned"):
        await execute_task_via_kernel("test task")


@pytest.mark.asyncio
async def test_execute_task_with_session_raises_decommissioned_error() -> None:
    with pytest.raises(RuntimeError, match="decommissioned"):
        await execute_task_with_session(session_id="sid", user_message="hello")
