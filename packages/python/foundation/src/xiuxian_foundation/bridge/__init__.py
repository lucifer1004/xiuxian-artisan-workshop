"""
Bridge Module - Rust Bindings Isolation Layer

This module provides a clean interface between Python and the remaining
bridge-side helpers.

Architecture:
- types.py: Python-side data structures (dataclasses)
- interfaces.py: Protocol definitions (ABC/Protocol)
- rust_*.py: bridge implementations (split for modularity)

Modules:
- rust_analyzer.py: Code analysis utilities

Usage:
    from xiuxian_foundation.bridge import SearchResult
    from xiuxian_foundation.bridge.interfaces import VectorStoreProvider
"""

from __future__ import annotations

# Lazy exports - avoid importing at module level to prevent recursion
_lazy_types = None
_lazy_interfaces = None


def __getattr__(name: str):
    """Lazy load bridge submodules."""
    global _lazy_types, _lazy_interfaces

    # Types
    if name in (
        "SearchResult",
        "FileContent",
        "VectorMetadata",
        "CodeSymbol",
        "ScanResult",
        "SkillStructure",
        "IngestResult",
    ):
        if _lazy_types is None:
            from . import types

            _lazy_types = types
        return getattr(_lazy_types, name)

    # Interfaces
    if name in (
        "VectorStoreProvider",
        "CodeAnalysisProvider",
        "FileScannerProvider",
        "SkillScannerProvider",
    ):
        if _lazy_interfaces is None:
            from . import interfaces

            _lazy_interfaces = interfaces
        return getattr(_lazy_interfaces, name)

    if name in ("ToolRecordValidationError", "validate_tool_record", "validate_tool_records"):
        from .tool_record_validation import (
            ToolRecordValidationError,
            validate_tool_record,
            validate_tool_records,
        )

        return (
            ToolRecordValidationError
            if name == "ToolRecordValidationError"
            else (validate_tool_record if name == "validate_tool_record" else validate_tool_records)
        )

    if name in (
        "RustCodeAnalyzer",
        "get_code_analyzer",
    ):
        from .rust_analyzer import RustCodeAnalyzer, get_code_analyzer

        return RustCodeAnalyzer if name == "RustCodeAnalyzer" else get_code_analyzer

    raise AttributeError(f"module {__name__!r} has no attribute {name!r}")


def __dir__():
    """List available attributes for autocomplete."""
    return [
        # Validation
        "ToolRecordValidationError",
        "validate_tool_record",
        "validate_tool_records",
        # Types
        "SearchResult",
        "FileContent",
        "VectorMetadata",
        "CodeSymbol",
        "ScanResult",
        "SkillStructure",
        "IngestResult",
        # Interfaces
        "VectorStoreProvider",
        "CodeAnalysisProvider",
        "FileScannerProvider",
        "SkillScannerProvider",
        # Implementations
        "RustCodeAnalyzer",
        # Factories
        "get_code_analyzer",
    ]
