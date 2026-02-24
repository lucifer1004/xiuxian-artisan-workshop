"""
omni.core.skills.runtime - Skill Runtime Environment

Standalone runtime implementation for Zero-Code Skill Architecture.
Manages SkillContext and skill lifecycle.

Usage:
    from omni.core.skills.runtime import get_skill_context, SkillContext
    ctx = get_skill_context(skills_dir)
    ctx.register_skill(universal_skill)

    # Get filtered core commands
    core_commands = ctx.get_core_commands()

    # Get all commands (including filtered)
    all_commands = ctx.list_commands()
"""

from __future__ import annotations

from pathlib import Path
from typing import Any

from omni.foundation.config.logging import get_logger

logger = get_logger("omni.core.runtime")


class SkillContext:
    """Runtime context for managing skills.

    Provides:
    - Skill registration and retrieval
    - Skill lifecycle management
    - Command dispatch (both decorated and native functions)
    - Reload callbacks for MCP notification integration
    """

    def __init__(self, skills_dir: Path):
        """Initialize skill context.

        Args:
            skills_dir: Path to assets/skills directory
        """
        self.skills_dir = Path(skills_dir)
        self._skills: dict[str, Any] = {}
        self._commands: dict[str, Any] = {}  # Decorated commands: "skill.command"
        self._native: dict[str, Any] = {}  # Native functions: "skill.function"
        self._reload_callbacks: list[callable] = []  # Callbacks after reload

    def _loader_native_aliases(self, skill: Any) -> dict[str, Any]:
        """Return native alias candidates currently owned by a skill loader."""
        loader = getattr(skill, "_tools_loader", None)
        if loader is None:
            return {}
        native = getattr(loader, "native_functions", None)
        if not isinstance(native, dict):
            return {}
        return dict(native)

    def _drop_skill_bindings(
        self, skill_name: str, native_alias_candidates: dict[str, Any] | None = None
    ) -> None:
        """Remove command/native bindings for one skill from context registries."""
        skill_prefix = f"{skill_name}."
        old_commands_to_remove = [cmd for cmd in self._commands if cmd.startswith(skill_prefix)]
        for cmd in old_commands_to_remove:
            del self._commands[cmd]

        old_native_to_remove = [key for key in self._native if key.startswith(skill_prefix)]
        for key in old_native_to_remove:
            del self._native[key]

        if native_alias_candidates:
            for alias_name, alias_func in native_alias_candidates.items():
                if self._native.get(alias_name) is alias_func:
                    del self._native[alias_name]

    def _register_loader_bindings(self, skill_name: str, loader: Any) -> None:
        """Register a loader's command/native bindings into context registries."""
        commands = getattr(loader, "commands", {})
        if isinstance(commands, dict):
            for cmd_name, handler in commands.items():
                self._commands[cmd_name] = handler

        native_functions = getattr(loader, "native_functions", {})
        if isinstance(native_functions, dict):
            for func_name, func in native_functions.items():
                self._native[f"{skill_name}.{func_name}"] = func
                self._native[func_name] = func

    def register_skill(self, skill: Any) -> None:
        """Register a loaded skill (UniversalScriptSkill).

        For hot reload: clears old skill's commands before adding new ones.

        Args:
            skill: A UniversalScriptSkill instance
        """
        if hasattr(skill, "name"):
            skill_name = skill.name
            old_skill = self._skills.get(skill_name)
            old_native_aliases = self._loader_native_aliases(old_skill) if old_skill else {}

            # Save mtime for hot reload detection
            skill_path = getattr(skill, "_path", None)
            if skill_path and skill_path.exists():
                scripts_path = skill_path / "scripts"
                if scripts_path.exists():
                    try:
                        skill._mtime = max(f.stat().st_mtime for f in scripts_path.glob("*.py"))
                    except (ValueError, OSError):
                        skill._mtime = 0
                else:
                    skill._mtime = 0
            else:
                skill._mtime = 0

            self._skills[skill.name] = skill

            # Clear old skill bindings before adding new ones.
            self._drop_skill_bindings(skill_name, old_native_aliases)

            # Register decorated commands from the skill
            if hasattr(skill, "_tools_loader") and skill._tools_loader is not None:
                self._register_loader_bindings(skill_name, skill._tools_loader)

            logger.debug(
                f"Registered skill: {skill.name} ({len(self._commands)} commands, {len(self._native)} native)"
            )
        else:
            logger.warning(f"Attempted to register nameless skill: {skill}")

    def on_reload(self, callback: callable) -> None:
        """Register a callback to be invoked when skills are reloaded.

        This is used by MCP Gateway to send notifications/tools/listChanged
        when scripts change on disk.

        Args:
            callback: Synchronous callback function (no args)
        """
        self._reload_callbacks.append(callback)
        logger.debug(f"Registered reload callback (total: {len(self._reload_callbacks)})")

    def _notify_reload(self) -> None:
        """Internal: notify all registered callbacks of a reload."""
        for callback in self._reload_callbacks:
            try:
                callback()
            except Exception as e:
                logger.warning(f"Error in reload callback: {e}")

    def get_skill(self, name: str) -> Any | None:
        """Get a registered skill by name, with hot reload support.

        Args:
            name: Skill name (e.g., "git", "filesystem")

        Returns:
            Skill instance or None
        """
        skill = self._skills.get(name)
        if skill is None:
            return None

        # Hot reload check: verify mtime and reload if modified
        skill_path = getattr(skill, "_path", None)
        if skill_path and skill_path.exists():
            scripts_path = skill_path / "scripts"
            if scripts_path.exists():
                try:
                    current_mtime = max(f.stat().st_mtime for f in scripts_path.glob("*.py"))
                    cached_mtime = getattr(skill, "_mtime", 0)
                    if current_mtime > cached_mtime:
                        logger.info(f"Hot reloading skill: {name}")

                        # Clear sys.modules cache for this skill (hot reload support)
                        import sys

                        skill_module_prefix = f"{name}."
                        modules_to_remove = [
                            k for k in sys.modules if k.startswith(skill_module_prefix)
                        ]
                        for mod in modules_to_remove:
                            del sys.modules[mod]

                        loader = getattr(skill, "_tools_loader", None)
                        if loader is None:
                            skill._mtime = current_mtime
                            return skill

                        old_context_commands = {
                            cmd_name: handler
                            for cmd_name, handler in self._commands.items()
                            if cmd_name.startswith(f"{name}.")
                        }
                        old_context_native = {
                            native_name: handler
                            for native_name, handler in self._native.items()
                            if native_name.startswith(f"{name}.")
                        }
                        old_loader_commands = dict(loader.commands)
                        old_loader_native = dict(loader.native_functions)
                        old_native_aliases = {
                            alias_name: alias_func
                            for alias_name, alias_func in old_loader_native.items()
                            if self._native.get(alias_name) is alias_func
                        }

                        self._drop_skill_bindings(name, old_loader_native)
                        reload_ok = False
                        reload_error: Exception | None = None

                        try:
                            loader.commands.clear()
                            loader.native_functions.clear()
                            loader.load_all()

                            has_scripts = any(scripts_path.glob("*.py"))
                            if old_loader_commands and not loader.commands and has_scripts:
                                raise RuntimeError(
                                    "hot reload produced empty command set while scripts still exist"
                                )

                            self._register_loader_bindings(name, loader)
                            skill._mtime = current_mtime
                            reload_ok = True
                        except Exception as exc:  # pragma: no cover - exercised by tests
                            reload_error = exc
                        finally:
                            if not reload_ok:
                                # Remove partial hot-reload state and restore previous snapshot.
                                self._drop_skill_bindings(name, dict(loader.native_functions))
                                loader.commands.clear()
                                loader.commands.update(old_loader_commands)
                                loader.native_functions.clear()
                                loader.native_functions.update(old_loader_native)
                                self._commands.update(old_context_commands)
                                self._native.update(old_context_native)
                                self._native.update(old_native_aliases)
                                logger.warning(
                                    f"Hot reload failed for skill '{name}'; "
                                    f"restored previous command set ({reload_error})"
                                )
                            else:
                                self._notify_reload()
                except (ValueError, OSError):
                    pass  # No scripts or other error

        return skill

    def get_command(self, full_name: str) -> Any | None:
        """Get a command handler (decorated commands).

        Args:
            full_name: Command name (e.g., "git.git_commit")

        Returns:
            Command function or None
        """
        return self._commands.get(full_name)

    def get_native(self, skill_name: str, func_name: str) -> Any | None:
        """Get a native function from a skill.

        Args:
            skill_name: Skill name (e.g., "git")
            func_name: Function name (e.g., "status")

        Returns:
            Native function or None
        """
        # Try "skill.function" format first
        key = f"{skill_name}.{func_name}"
        if key in self._native:
            return self._native[key]
        # Fall back to just function name
        return self._native.get(func_name)

    def list_native_functions(self, skill_name: str | None = None) -> list[str]:
        """List native function names.

        Args:
            skill_name: Optional skill name to filter by

        Returns:
            List of native function names
        """
        if skill_name:
            return [
                k for k in self._native if isinstance(k, str) and k.startswith(f"{skill_name}.")
            ]
        return list(set(self._native.keys()))

    def list_skills(self) -> list[str]:
        """List registered skill names.

        Returns:
            List of skill names
        """
        return list(self._skills.keys())

    def list_commands(self) -> list[str]:
        """List all registered commands (including filtered).

        Returns:
            List of command names
        """
        return list(self._commands.keys())

    def get_filtered_commands(self) -> list[str]:
        """List commands that should be filtered from core tools.

        Returns commands matching filter_commands config.

        Returns:
            List of filtered command names
        """
        from omni.core.config.loader import is_filtered

        return [cmd for cmd in self._commands if is_filtered(cmd)]

    def get_core_commands(self) -> list[str]:
        """List commands available in core tools (filtered excluded).

        Applies filter_commands config to exclude certain commands
        from being considered core tools.

        Returns:
            List of core command names (filter_commands excluded)
        """
        from omni.core.config.loader import is_filtered

        return [cmd for cmd in self._commands if not is_filtered(cmd)]

    def get_dynamic_commands(self) -> list[str]:
        """List commands available as dynamic tools.

        These are commands that were filtered from core tools
        but can still be loaded on demand.

        Returns:
            List of dynamic command names (filter_commands included)
        """
        from omni.core.config.loader import load_filter_commands

        filter_config = load_filter_commands()
        filter_set = set(filter_config.commands)
        return [cmd for cmd in self._commands if cmd in filter_set]

    def clear(self) -> None:
        """Clear all registered skills and commands."""
        self._skills.clear()
        self._commands.clear()

    @property
    def skills_count(self) -> int:
        """Get number of registered skills."""
        return len(self._skills)


class SkillDiscovery:
    """Skill discovery service."""

    def __init__(self, skills_dir: Path):
        self.skills_dir = Path(skills_dir)

    def discover(self) -> list[str]:
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
_context: SkillContext | None = None


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


def reset_context() -> None:
    """Reset the global skill context (for testing)."""
    global _context
    if _context is not None:
        _context.clear()
    _context = None


async def run_command(command: str, **kwargs) -> Any:
    """Run a skill command (decorated or native).

    Args:
        command: Full command name (e.g., "git.git_commit" or "git.status")
        **kwargs: Command arguments

    Returns:
        Command result
    """
    global _context
    if _context is None:
        raise RuntimeError("SkillContext not initialized. Call get_skill_context() first.")

    # First try decorated command
    handler = _context.get_command(command)
    if handler is None and "." in command:
        # Try native function: parse "skill.func" -> skill="git", func="status"
        skill_name, func_name = command.split(".", 1)
        handler = _context.get_native(skill_name, func_name)

    if handler is None:
        available = _context.list_commands()
        raise ValueError(f"Command '{command}' not found. Available: {available}")

    if callable(handler):
        import inspect

        if inspect.iscoroutinefunction(handler):
            return await handler(**kwargs)
        return handler(**kwargs)

    raise TypeError(f"Command handler is not callable: {handler}")


# Convenience type aliases
SkillManager = SkillContext

# Deprecated type stubs - removed (agent.core.skill_runtime no longer exists)


__all__ = [
    # Context
    "SkillContext",
    "SkillManager",  # Alias
    "get_skill_context",
    "reset_context",
    # Execution
    "run_command",
]
