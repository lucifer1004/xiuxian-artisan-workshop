"""
omni.core.skills.runtime - Skill Runtime Environment

Standalone runtime implementation for Zero-Code Skill Architecture.
Manages SkillContext and skill lifecycle.

Usage:
    from omni.core.skills.runtime import get_skill_context, SkillContext
    ctx = get_skill_context(skills_dir)
    ctx.register_skill(universal_skill)
"""

from __future__ import annotations

import logging
from pathlib import Path
from typing import Any, Dict, List, Optional, TYPE_CHECKING

logger = logging.getLogger("omni.core.runtime")


class SkillContext:
    """Runtime context for managing skills.

    Provides:
    - Skill registration and retrieval
    - Skill lifecycle management
    - Command dispatch
    """

    def __init__(self, skills_dir: Path):
        """Initialize skill context.

        Args:
            skills_dir: Path to assets/skills directory
        """
        self.skills_dir = Path(skills_dir)
        self._skills: Dict[str, Any] = {}
        self._commands: Dict[str, Any] = {}

    def register_skill(self, skill: Any) -> None:
        """Register a loaded skill (UniversalScriptSkill).

        Args:
            skill: A UniversalScriptSkill instance
        """
        if hasattr(skill, "name"):
            self._skills[skill.name] = skill

            # Register all commands from the skill
            if hasattr(skill, "list_commands"):
                for cmd in skill.list_commands():
                    self._commands[cmd] = skill

            logger.debug(f"Registered skill: {skill.name} ({len(skill.list_commands())} commands)")
        else:
            logger.warning(f"Attempted to register nameless skill: {skill}")

    def get_skill(self, name: str) -> Optional[Any]:
        """Get a registered skill by name.

        Args:
            name: Skill name (e.g., "git", "filesystem")

        Returns:
            Skill instance or None
        """
        return self._skills.get(name)

    def get_command(self, full_name: str) -> Optional[Any]:
        """Get a command handler.

        Args:
            full_name: Command name (e.g., "git.git_commit")

        Returns:
            Command function or None
        """
        return self._commands.get(full_name)

    def list_skills(self) -> List[str]:
        """List registered skill names.

        Returns:
            List of skill names
        """
        return list(self._skills.keys())

    def list_commands(self) -> List[str]:
        """List all registered commands.

        Returns:
            List of command names
        """
        return list(self._commands.keys())

    def clear(self) -> None:
        """Clear all registered skills and commands."""
        self._skills.clear()
        self._commands.clear()

    @property
    def skills_count(self) -> int:
        """Get number of registered skills."""
        return len(self._skills)


class SkillRegistry:
    """Legacy skill registry (for compatibility)."""

    def __init__(self):
        self._skills: Dict[str, Any] = {}

    def register(self, name: str, skill: Any) -> None:
        self._skills[name] = skill

    def get(self, name: str) -> Optional[Any]:
        return self._skills.get(name)


class SkillDiscovery:
    """Skill discovery service."""

    def __init__(self, skills_dir: Path):
        self.skills_dir = Path(skills_dir)

    def discover(self) -> List[str]:
        """Discover available skills.

        Returns:
            List of skill names
        """
        if not self.skills_dir.exists():
            return []

        return [
            d.name for d in self.skills_dir.iterdir() if d.is_dir() and not d.name.startswith("_")
        ]


# Global context singleton
_context: Optional[SkillContext] = None


def get_skill_context(skills_dir: Path) -> SkillContext:
    """Get or create the global skill context.

    Args:
        skills_dir: Path to assets/skills directory

    Returns:
        SkillContext instance
    """
    global _context
    if _context is None:
        _context = SkillContext(skills_dir)
    return _context


def get_skill_manager(skills_dir: Path) -> SkillContext:
    """Backward compatibility alias for get_skill_context."""
    return get_skill_context(skills_dir)


def reset_context() -> None:
    """Reset the global skill context (for testing)."""
    global _context
    if _context is not None:
        _context.clear()
    _context = None


def get_registry() -> SkillRegistry:
    """Get the skill registry (for compatibility)."""
    return SkillRegistry()


async def run_command(command: str, **kwargs) -> Any:
    """Run a skill command.

    Args:
        command: Full command name (e.g., "git.git_commit")
        **kwargs: Command arguments

    Returns:
        Command result
    """
    global _context
    if _context is None:
        raise RuntimeError("SkillContext not initialized. Call get_skill_context() first.")

    handler = _context.get_command(command)
    if handler is None:
        available = _context.list_commands()
        raise ValueError(f"Command '{command}' not found. Available: {available}")

    if hasattr(handler, "__call__"):
        import inspect

        if inspect.iscoroutinefunction(handler):
            return await handler(**kwargs)
        return handler(**kwargs)

    raise TypeError(f"Command handler is not callable: {handler}")


# Convenience type aliases
SkillManager = SkillContext

# Import stubs for backward compatibility (from agent.core.skill_runtime)
if TYPE_CHECKING:
    from agent.core.skill_runtime import (
        SkillMemoryManager,
        SkillBootManager,
        SkillQueryManager,
        SkillLoadManager,
        SkillSearchManager,
        SkillLifecycle,
        SkillJITLoader,
        SkillExecutor,
        SkillCommand,
        Skill,
        ObserverMixin,
        SkillLoaderMixin,
        HotReloadMixin,
    )


__all__ = [
    # Context
    "SkillContext",
    "SkillManager",  # Alias
    "get_skill_context",
    "get_skill_manager",
    "reset_context",
    # Registry
    "SkillRegistry",
    "get_registry",
    # Discovery
    "SkillDiscovery",
    # Execution
    "run_command",
]
