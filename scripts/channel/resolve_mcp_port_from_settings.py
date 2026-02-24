#!/usr/bin/env python3
"""Resolve MCP port from project settings with embedding URL fallback."""

from __future__ import annotations

from urllib.parse import urlparse

from omni.foundation.config.settings import get_setting


def _normalize_port(value: object) -> int | None:
    if isinstance(value, bool):
        return None
    if isinstance(value, int):
        normalized = value
    elif isinstance(value, str):
        text = value.strip()
        if not text:
            return None
        try:
            normalized = int(text)
        except ValueError:
            return None
    else:
        return None

    if 1 <= normalized <= 65535:
        return normalized
    return None


def resolve_mcp_port() -> int | None:
    preferred = _normalize_port(get_setting("mcp.preferred_embed_port"))
    if preferred is not None:
        return preferred

    client_url = get_setting("embedding.client_url")
    if isinstance(client_url, str) and client_url.strip():
        parsed = urlparse(client_url.strip())
        return _normalize_port(parsed.port)

    return None


def main() -> int:
    port = resolve_mcp_port()
    print("" if port is None else str(port), end="")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
