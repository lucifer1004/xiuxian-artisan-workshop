"""Protocol constants shared by agent MCP transports/handlers."""

from __future__ import annotations

MCP_PROTOCOL_VERSION = "2025-06-18"


def is_supported_mcp_protocol_version(version: str) -> bool:
    """Return True when version matches the only supported MCP protocol."""
    return version == MCP_PROTOCOL_VERSION
