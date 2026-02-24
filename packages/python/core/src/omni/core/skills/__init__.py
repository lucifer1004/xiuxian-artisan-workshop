"""
omni.core.skills - Skills System (lazy exports).

Avoid eager package imports so fast CLI paths can import ``omni.core.skills.runner``
without loading the full skills subsystem.
"""

from __future__ import annotations

import importlib
from typing import Any

_LAZY_EXPORTS: dict[str, tuple[str, str]] = {
    # Discovery
    "DiscoveredSkill": (".discovery", "DiscoveredSkill"),
    "SkillDiscoveryService": (".discovery", "SkillDiscoveryService"),
    "is_rust_available": (".discovery", "is_rust_available"),
    # Extensions
    "ExtensionWrapper": (".extensions", "ExtensionWrapper"),
    "SkillExtensionLoader": (".extensions", "SkillExtensionLoader"),
    "get_extension_loader": (".extensions", "get_extension_loader"),
    # Memory
    "SkillMemory": (".memory", "SkillMemory"),
    "get_skill_memory": (".memory", "get_skill_memory"),
    # Context Hydration
    "SkillIndexLoader": (".index_loader", "SkillIndexLoader"),
    "FileCache": (".file_cache", "FileCache"),
    "RefParser": (".ref_parser", "RefParser"),
    "ContextHydrator": (".hydrator", "ContextHydrator"),
    # Registry
    "SkillRegistry": (".registry", "SkillRegistry"),
    "get_skill_registry": (".registry", "get_skill_registry"),
    "HolographicRegistry": (".registry", "HolographicRegistry"),
    "ToolMetadata": (".registry", "ToolMetadata"),
    "LazyTool": (".registry", "LazyTool"),
    # Runtime
    "SkillContext": (".runtime", "SkillContext"),
    "SkillManager": (".runtime", "SkillManager"),
    "get_skill_context": (".runtime", "get_skill_context"),
    "reset_context": (".runtime", "reset_context"),
    "run_command": (".runtime", "run_command"),
    # Tools loader
    "ToolsLoader": (".tools_loader", "ToolsLoader"),
    "create_tools_loader": (".tools_loader", "create_tools_loader"),
    "_skill_command_registry": (".tools_loader", "_skill_command_registry"),
    # Universal skill
    "UniversalScriptSkill": (".universal", "UniversalScriptSkill"),
    "UniversalSkillFactory": (".universal", "UniversalSkillFactory"),
    "create_skill_from_assets": (".universal", "create_skill_from_assets"),
    "create_universal_skill": (".universal", "create_universal_skill"),
    # Runner
    "FastPathUnavailable": (".runner", "FastPathUnavailable"),
    "run_skill": (".runner", "run_skill"),
    "run_skill_with_monitor": (".runner", "run_skill_with_monitor"),
    # Indexer
    "SkillIndexer": (".indexer", "SkillIndexer"),
}


def __getattr__(name: str) -> Any:
    export = _LAZY_EXPORTS.get(name)
    if export is None:
        raise AttributeError(f"module {__name__!r} has no attribute {name!r}")
    module_name, attr_name = export
    module = importlib.import_module(module_name, package=__name__)
    value = getattr(module, attr_name)
    globals()[name] = value
    return value


__all__ = [
    "ContextHydrator",
    "DiscoveredSkill",
    "ExtensionWrapper",
    "FastPathUnavailable",
    "FileCache",
    "HolographicRegistry",
    "LazyTool",
    "RefParser",
    "SkillContext",
    "SkillDiscoveryService",
    "SkillExtensionLoader",
    "SkillIndexLoader",
    "SkillIndexer",
    "SkillManager",
    "SkillMemory",
    "SkillRegistry",
    "ToolMetadata",
    "ToolsLoader",
    "UniversalScriptSkill",
    "UniversalSkillFactory",
    "_skill_command_registry",
    "create_skill_from_assets",
    "create_tools_loader",
    "create_universal_skill",
    "get_extension_loader",
    "get_skill_context",
    "get_skill_memory",
    "get_skill_registry",
    "is_rust_available",
    "reset_context",
    "run_command",
    "run_skill",
    "run_skill_with_monitor",
]
