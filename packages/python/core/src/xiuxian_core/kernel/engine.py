"""
kernel/engine.py - Core Agent Engine

Trinity Architecture - Core Layer

Single entry point for agent core, providing:
- Unified lifecycle management
- Component registry
- Dependency injection
- Clean separation between core and domain modules

Logging: Uses Foundation layer (xiuxian_foundation.config.logging)
"""

from __future__ import annotations

import asyncio
import time
from pathlib import Path
from typing import TYPE_CHECKING, Any

from xiuxian_foundation.config.logging import configure_logging, get_logger

from .lifecycle import LifecycleManager, LifecycleState
from .reactor import EventTopic, get_reactor

if TYPE_CHECKING:
    from xiuxian_core.security import SecurityValidator

# Ensure logging is configured before getting logger
configure_logging(level="INFO")
logger = get_logger("xiuxian_core.kernel")

# Global kernel singleton
_kernel_instance: Kernel | None = None


class Kernel:
    """Kernel - single entry point for agent core.

    Responsibilities:
    - Lifecycle management (init -> ready -> running -> shutdown)
    - Component registry for dependency injection
    - Clean separation between core and domain modules
    - Bridge to existing skill_runtime system
    - Rust-powered skill discovery integration
    - Hot Reload for skill development
    - Security Enforcement (Permission Gatekeeper)

    Usage:
        kernel = get_kernel()
        await kernel.initialize()
        await kernel.start()
        # Secure execution:
        await kernel.execute_tool("git.status", {}, caller="researcher")
        await kernel.shutdown()
    """

    __slots__ = (
        "_background_tasks",  # Track background tasks for cleanup
        "_components",
        "_cortex_enabled",  # Legacy startup flag kept for constructor compatibility
        "_discovered_skills",
        "_lifecycle",
        "_project_root",
        "_reactor",  # Event-driven reactor for reactive architecture
        "_router",
        "_security",  # Security Validator (Permission Gatekeeper)
        "_sniffer",  # Intent Sniffer for context detection
    )

    def __init__(
        self,
        *,
        project_root: Path | None = None,
        skills_dir: Path | None = None,
        enable_cortex: bool = True,
    ) -> None:
        """Initialize kernel with optional paths.

        Args:
            project_root: Project root directory (auto-detected if None)
            skills_dir: Skills directory (defaults to project_root/assets/skills)
            enable_cortex: Legacy startup flag retained for constructor compatibility.
        """
        self._cortex_enabled = enable_cortex
        self._lifecycle = LifecycleManager(
            on_ready=self._on_ready,
            on_running=self._on_running,
            on_shutdown=self._on_shutdown,
        )
        self._components: dict[str, Any] = {}
        self._discovered_skills: list[Any] = []
        self._router = None
        self._sniffer: Any | None = None
        self._security = None  # Security Validator (Permission Gatekeeper) - lazy init
        self._reactor = None  # Event-driven reactor - initialized in _on_ready
        self._background_tasks: set[asyncio.Task] = set()

        # Resolve paths
        from xiuxian_foundation.runtime.gitops import get_project_root

        self._project_root = project_root or get_project_root()
        _ = skills_dir

    # =========================================================================
    # Lifecycle
    # =========================================================================

    @property
    def state(self) -> LifecycleState:
        """Get current lifecycle state."""
        return self._lifecycle.state

    @property
    def is_ready(self) -> bool:
        """Check if kernel is ready (initialized and operational).

        Returns True for both READY and RUNNING states, as the kernel
        is fully operational after start() is called.
        """
        return self._lifecycle.is_ready() or self._lifecycle.is_running()

    @property
    def is_running(self) -> bool:
        """Check if kernel is running."""
        return self._lifecycle.is_running()

    async def initialize(self) -> None:
        """Initialize kernel and all components."""
        await self._lifecycle.initialize()

    async def start(self) -> None:
        """Start kernel (transition to running state)."""
        await self._lifecycle.start()

    async def shutdown(self) -> None:
        """Shutdown kernel and cleanup all components."""
        await self._lifecycle.shutdown()

    # =========================================================================
    # Components
    # =========================================================================

    def register_component(self, name: str, component: Any) -> None:
        """Register a component by name.

        Args:
            name: Component name
            component: Component instance

        Raises:
            ValueError: If component already registered
        """
        if name in self._components:
            raise ValueError(f"Component '{name}' already registered")
        self._components[name] = component

    def get_component(self, name: str) -> Any:
        """Get a registered component.

        Args:
            name: Component name

        Returns:
            Component instance

        Raises:
            KeyError: If component not found
        """
        return self._components[name]

    def has_component(self, name: str) -> bool:
        """Check if a component is registered."""
        return name in self._components

    # =========================================================================
    # Security (Permission Gatekeeper)
    # =========================================================================

    @property
    def security(self) -> SecurityValidator:
        """Get the Security Validator (Permission Gatekeeper).

        Uses Rust-powered PermissionGatekeeper for high-performance checks.
        Lazy initialization to avoid startup overhead.
        """
        if self._security is None:
            from xiuxian_core.security import SecurityValidator

            self._security = SecurityValidator()
        return self._security

    async def execute_tool(
        self,
        tool_name: str,
        args: dict[str, Any],
        caller: str | None = None,
    ) -> Any:
        """
        Execute a tool with mandatory security checks.

        This is the primary entry point for all skill-to-skill and agent-to-skill calls.
        All tool invocations must pass through this method for proper permission enforcement.

        Args:
            tool_name: Full tool name in format "skill.command" (e.g., "git.status")
            args: Tool arguments as a dictionary
            caller: Name of the calling skill (e.g., "researcher"). None = Root/User (full access)

        Returns:
            Tool execution result (CommandResult)

        Raises:
            SecurityError: If permission is denied
            ValueError: If tool name format is invalid or tool not found
        """
        # 1. Validate Tool Identifier
        if "." not in tool_name:
            raise ValueError(
                f"Invalid tool name format '{tool_name}'. Expected 'skill.command' format."
            )

        _ = (args, caller, tool_name)
        raise RuntimeError(
            "Python local tool execution has been removed. Use Rust/Wendao over "
            "Arrow Flight for command dispatch and execution."
        )

    # =========================================================================
    # Skill Discovery
    # =========================================================================

    @property
    def discovered_skills(self) -> list[Any]:
        """Get list of discovered skills."""
        return self._discovered_skills

    async def discover_skills(self) -> list[Any]:
        """Python no longer performs local skill discovery."""
        if not self._discovered_skills:
            logger.info(
                "Python local skill discovery removed; Rust/Wendao owns discovery over Arrow Flight"
            )
        return self._discovered_skills

    def load_universal_skill(self, skill_name: str) -> Any:
        _ = skill_name
        raise RuntimeError(
            "Skill materialization is Rust-owned. Use Rust/Wendao over Arrow Flight."
        )

    async def load_all_universal_skills(self) -> list[Any]:
        logger.info("Rust/Wendao owns skill materialization")
        return []

    # =========================================================================
    # Skill Runtime Integration
    # =========================================================================

    @property
    def router(self):
        raise RuntimeError(
            "Routing and command selection are Rust-owned. Use Rust/Wendao over Arrow Flight."
        )

    # =========================================================================
    # Intent Sniffer (The Nose) - Context Detection
    # =========================================================================

    @property
    def sniffer(self) -> Any:
        raise RuntimeError(
            "Routing context signals are Rust-owned. Use Rust/Wendao over Arrow Flight."
        )

    async def load_sniffer_rules(self) -> int:
        raise RuntimeError(
            "Routing context signals are Rust-owned. Use Rust/Wendao over Arrow Flight."
        )

    async def build_cortex(self) -> None:
        raise RuntimeError("Routing indexes are Rust-owned. Use Rust/Wendao over Arrow Flight.")

    # =========================================================================
    # Event-Driven Reactor (Reactive Architecture)
    # =========================================================================

    @property
    def reactor(self):
        """Get the event-driven reactor (lazy initialization).

        The Reactor consumes events from the Rust Event Bus and dispatches
        to registered Python handlers. This enables reactive architecture:
        - Cortex auto-indexing on file changes
        - Sniffer context updates
        """
        if self._reactor is None:
            self._reactor = get_reactor()
        return self._reactor

    async def _on_file_changed_cortex(self, event: dict) -> None:
        _ = event
        return None

    # =========================================================================
    # Paths
    # =========================================================================

    @property
    def project_root(self) -> Path:
        """Get project root directory."""
        return self._project_root

    async def reload_skill(self, skill_name: str) -> None:
        _ = skill_name
        raise RuntimeError("Reload orchestration is Rust-owned. Use Rust/Wendao over Arrow Flight.")

    async def _safe_build_cortex(self) -> None:
        return None

    async def _notify_clients_tool_list_changed(self) -> None:
        """No-op after Python-side tool-runtime removal."""
        return None

    # =========================================================================
    # Lifecycle Callbacks
    # =========================================================================

    async def _on_ready(self) -> None:
        """Called when kernel reaches READY state."""
        import time as _time

        t0 = _time.time()
        logger.info("🟢 Kernel initializing...")

        logger.info("📦 Rust/Wendao owns skill loading")
        skills_loaded = 0
        t1 = _time.time()
        t2 = t1

        logger.info("🧠 Rust/Wendao owns routing indexes")
        logger.info("👃 Routing context signals are Rust-owned")
        logger.info("🔗 Python event reactor remains disabled")
        t3 = _time.time()
        t4 = t3

        t5 = _time.time()

        t6 = _time.time()

        # Timing summary
        total_time = t6 - t0
        logger.info(
            f"[TIMING] Kernel startup: {total_time:.2f}s "
            f"(skills: {t1 - t0:.2f}s, load: {t2 - t1:.2f}s, "
            f"router_removed: {t3 - t2:.2f}s, sniffer: {t4 - t3:.2f}s, "
            f"reactor: {t5 - t4:.2f}s)"
        )

        # Summary of active services
        logger.info("━" * 60)
        logger.info("🚀 Kernel Services Active:")
        logger.info(f"   • Skills:    {skills_loaded} loaded (Python local runtime removed)")
        logger.info("   • Router:    Python local semantic router removed")
        logger.info("   • Sniffer:   disabled")
        logger.info("   • Security:  Permission Gatekeeper active (Zero Trust)")
        logger.info("   • Reactor:   disabled")
        logger.info("━" * 60)

    async def _on_running(self) -> None:
        """Called when kernel reaches RUNNING state."""
        pass

    async def _on_shutdown(self) -> None:
        """Called when kernel starts shutting down - graceful cleanup."""
        logger.info("🛑 Kernel shutting down...")

        # Step 0: Cancel background tasks
        if self._background_tasks:
            logger.debug(f"Cancelling {len(self._background_tasks)} background tasks")
            for task in self._background_tasks:
                task.cancel()

            # Wait for tasks to finish cancelling
            await asyncio.gather(*self._background_tasks, return_exceptions=True)
            self._background_tasks.clear()

        # Step 1: Unregister sniffer from reactor (cleanup handlers)
        if self._sniffer is not None:
            self._sniffer.unregister_from_reactor()
            logger.debug("Sniffer unregistered from reactor")

        # Step 2: Stop reactor first (no more event processing)
        if self._reactor is not None and self._reactor.is_running:
            await self._reactor.stop()
            self._reactor = None
            logger.debug("Event reactor stopped")

        # Step 3: Save any persistent state (vector index, caches)
        if hasattr(self, "_router") and self._router is not None:
            if hasattr(self._router, "_semantic") and hasattr(self._router._semantic, "_indexer"):
                indexer = self._router._semantic._indexer
                stats = await indexer.get_stats()
                if stats.get("entries_indexed", 0) > 0:
                    logger.info(f"💾 Index contains {stats['entries_indexed']} entries (in-memory)")

        self._discovered_skills.clear()

        # Step 4: Cleanup components
        self._components.clear()
        self._router = None
        self._sniffer = None
        self._security = None

        logger.info("👋 Kernel shutdown complete")


def get_kernel(*, enable_cortex: bool | None = None, reset: bool = False) -> Kernel:
    """Get the global kernel instance (singleton).

    Args:
        enable_cortex: Override cortex setting. If None, uses existing or default (True).
            Set to False for CLI commands that don't need semantic routing.
        reset: If True, recreate the kernel instance.
    """
    global _kernel_instance
    if _kernel_instance is None or reset:
        _kernel_instance = Kernel(
            enable_cortex=enable_cortex if enable_cortex is not None else True
        )
    elif enable_cortex is not None:
        # Update cortex setting if kernel already exists
        _kernel_instance._cortex_enabled = enable_cortex
    return _kernel_instance


def reset_kernel() -> None:
    """Reset the global kernel instance (for testing)."""
    global _kernel_instance
    _kernel_instance = None
