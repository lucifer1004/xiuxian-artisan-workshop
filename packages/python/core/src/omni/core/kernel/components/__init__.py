"""
kernel/components/ - Unified Kernel Components

Provides unified implementations for:
- registry.py: Unified skill registry (skill_runtime + skill_registry merged)
- skill_plugin.py: Skill Plugin interface
- skill_loader.py: Skill script loader
- mcp_tool.py: MCP tool adapter

These components replace duplicate code in skill_registry.
"""

from __future__ import annotations

# Re-export unified registry
from .registry import UnifiedRegistry

# Re-export skill plugin interface
from .skill_plugin import ISkillPlugin, SkillPluginWrapper

# Re-export skill loader
from .skill_loader import load_skill_scripts, extract_tool_schema

# Re-export MCP tool adapter
from .mcp_tool import MCPToolAdapter

__all__ = [
    "UnifiedRegistry",
    "ISkillPlugin",
    "SkillPluginWrapper",
    "load_skill_scripts",
    "extract_tool_schema",
    "MCPToolAdapter",
]
