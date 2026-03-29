"""
_template/scripts/commands.py - Skill Commands Template

No tools.py needed - this is the single source of skill commands.

Architecture:
    scripts/
    ├── __init__.py      # Module loader (importlib.util)
    └── commands.py      # Skill commands (direct definitions)

Usage:
    from skills._template.scripts import commands
    commands.example(...)

================================================================================
ODF-EP Protocol: CLI Command Description Standards
================================================================================

Format Rules:
- Each param starts with "- "
- Format: "- name: Type = default - Description"
- Optional params have "= default"
- Use Python type syntax: str, int, bool, list[str], Optional[str]

Action Verbs (First Line):
    Create, Get, Search, Update, Delete, Execute, Run, Load, Save,
    List, Show, Check, Build, Parse, Format, Validate, Generate,
    Apply, Process, Clear, Index, Ingest, Consult, Bridge, Refine,
    Summarize, Commit, Amend, Revert, Retrieve, Analyze, Suggest,
    Write, Read, Extract, Query, Filter, Detect, Navigate, Refactor

Categories:
    read   - Query/retrieve information
    write  - Modify/create content
    workflow - Multi-step operations
    search - Find/search operations
    view   - Display/visualize
================================================================================
"""

from typing import TypedDict

# =============================================================================
# Basic Skill Commands
# =============================================================================


def example(param: str) -> str:
    """Simple command - just return the result."""
    return f"Example: {param}"


def example_with_options(enabled: bool = True, value: int = 42) -> dict:
    """Command returning structured data."""
    return {
        "enabled": enabled,
        "value": value,
    }


def process_data(data: list[str], filter_empty: bool = True) -> list[str]:
    """Command with conditional logic."""
    if filter_empty:
        return [item for item in data if item.strip()]
    return data


# =============================================================================
# Error Handling Pattern
# =============================================================================


def validate_input(name: str, age: int) -> str:
    """Command demonstrating proper error handling."""
    if not name:
        raise ValueError("Name cannot be empty")

    if age < 0:
        raise ValueError("Age cannot be negative")

    return f"Valid: {name} (age {age})"


# =============================================================================
# Graph Node Pattern (for workflow skills)
# =============================================================================


class WorkflowState(TypedDict):
    """State for the example workflow."""

    input: str
    processed: str
    steps: int
    error: str | None


def node_process(state: WorkflowState) -> WorkflowState:
    """
    Process node - transform input data.

    Error handling: raise directly and let the CLI/runtime shell surface failures.
    """
    if not state.get("input"):
        raise ValueError("Input is required")

    processed = state["input"].upper()
    return {
        "input": state["input"],
        "processed": processed,
        "steps": state.get("steps", 0) + 1,
        "error": None,
    }


async def node_validate(state: WorkflowState) -> WorkflowState:
    """
    Validate processed data (async example).

    Async workflow steps remain plain callables.
    """
    if "error" in state:
        raise RuntimeError(f"Previous error: {state['error']}")

    return {
        "input": state["input"],
        "processed": state["processed"],
        "steps": state.get("steps", 0) + 1,
        "error": None,
    }


__all__ = [
    "example",
    "example_with_options",
    "node_process",
    "node_validate",
    "process_data",
    "validate_input",
]
