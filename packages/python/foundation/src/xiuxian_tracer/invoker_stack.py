"""
invoker_stack.py - Helpers for building standard ToolInvoker stacks.
"""

from __future__ import annotations

from typing import Any

from .composite_invoker import CompositeToolInvoker
from .node_factory import MappingToolInvoker, NoOpToolInvoker, ToolInvoker
from .tool_invoker import ToolClientInvoker


def create_default_invoker_stack(
    *,
    tool_client: Any | None = None,
    mapping: dict[str, Any] | None = None,
    default_invoker: ToolInvoker | None = None,
) -> CompositeToolInvoker:
    """Build the default invoker stack in priority order.

    Order:
    1. ToolClientInvoker (if `tool_client` is provided)
    2. MappingToolInvoker (if mapping is provided)
    3. default_invoker (or NoOpToolInvoker)
    """
    chain: list[ToolInvoker] = []
    if tool_client is not None:
        chain.append(ToolClientInvoker(tool_client))
    if mapping:
        chain.append(MappingToolInvoker(mapping))

    return CompositeToolInvoker(
        chain,
        default_invoker=default_invoker or NoOpToolInvoker(),
    )


__all__ = [
    "create_default_invoker_stack",
]
