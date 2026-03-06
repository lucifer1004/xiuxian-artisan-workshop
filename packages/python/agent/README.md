---
type: knowledge
metadata:
  title: "xiuxian-daochang"
---

# xiuxian-daochang

Tri-MCP Agent Orchestrator - The Brain of Xiuxian Artisan Workshop.

## Overview

This package provides the orchestrator MCP server that handles skill routing, LLM session management, and the `@omni("skill.command")` single entry point.

## Architecture

- `src/agent/core/` - Core system components (orchestrator, router, skill manager)
- `src/agent/mcp_server.py` - MCP server implementation
- `src/agent/cli.py` - CLI entry points

## Dependencies

- Shared kernel and utilities
- MCP and native workflow runtimes for agent orchestration
