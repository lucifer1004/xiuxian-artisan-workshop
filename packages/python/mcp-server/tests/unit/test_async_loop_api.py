"""Guardrails for modern asyncio loop APIs in MCP transport."""

from __future__ import annotations

from omni.foundation.runtime.gitops import get_project_root


def test_stdio_transport_uses_get_running_loop() -> None:
    project_root = get_project_root()
    path = (
        project_root
        / "packages"
        / "python"
        / "mcp-server"
        / "src"
        / "omni"
        / "mcp"
        / "transport"
        / "stdio.py"
    )
    assert path.exists(), f"Expected transport module at {path}"
    content = path.read_text(encoding="utf-8")
    assert "asyncio.get_event_loop()" not in content
    assert content.count("asyncio.get_running_loop()") >= 2
