"""
xiuxian_core.context - Cognitive Pipeline

Modular context providers for assembling LLM prompts.

Modules:
- base: Abstract base classes
- providers: Concrete context providers
- orchestrator: Context assembly engine
- prompts: Prompt injection utilities

Usage:
    from xiuxian_core.context import ContextOrchestrator, SystemPersonaProvider
    from xiuxian_core.prompts import inject_prompt, load_prompt

    orchestrator = ContextOrchestrator([
        SystemPersonaProvider(role="Architect"),
    ])

    # Inject prompt
    content = load_prompt("assets/prompts/custom.md", category="knowledge")
"""

# Prompt injection utilities
from ..prompts import PROMPT_TAGS, inject_prompt, load_prompt, merge_prompts
from .base import ContextProvider, ContextResult
from .orchestrator import (
    ContextOrchestrator,
    create_executor_orchestrator,
    create_planner_orchestrator,
)
from .providers import SystemPersonaProvider

__all__ = [
    "ContextOrchestrator",
    "ContextProvider",
    "ContextResult",
    "SystemPersonaProvider",
    "create_executor_orchestrator",
    "create_planner_orchestrator",
    # Prompts
    "inject_prompt",
    "load_prompt",
    "merge_prompts",
    "PROMPT_TAGS",
]
