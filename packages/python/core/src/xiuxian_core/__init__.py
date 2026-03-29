"""
agent/core/kernel/ - Kernel Abstraction Layer (Backward Compatibility Wrapper)

DEPRECATED: This module is kept for backward compatibility.
Please migrate to `xiuxian_core.kernel` for new code.

Microkernel architecture for agent core:

kernel.py           - Core Kernel class, single entry point
lifecycle.py        - State machine (init -> ready -> running -> shutdown)
components/         - Unified components (registry, orchestrator, loader)

This layer provides:
- Single entry point for agent initialization
- Unified lifecycle management
- Component isolation for clean architecture
"""

from __future__ import annotations

# Re-export from xiuxian_core for backward compatibility
from xiuxian_core.kernel import Kernel, get_kernel, LifecycleState, LifecycleManager

__all__ = [
    "Kernel",
    "get_kernel",
    "LifecycleState",
    "LifecycleManager",
]
