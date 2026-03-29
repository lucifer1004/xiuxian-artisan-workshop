"""Xiuxian Test Kit - Testing framework for the retained Python surface.

A comprehensive testing framework providing fixtures, decorators, and utilities
for writing robust, maintainable tests across the retained Xiuxian Python packages.

Features:
    - Auto-loaded pytest fixtures (core, git, rag, vector)
    - Fixture and payload helpers for retained transport/RAG testing
    - Data-driven testing support
    - Testing layer markers (unit, integration, cloud, etc.)
    - Assertion helpers for common patterns

Usage:
    # Enable all fixtures in conftest.py
    pytest_plugins = ["xiuxian_test_kit"]

    # Use fixtures directly in tests
    def test_something(project_root):
        ...

    # Use decorators
    @data_driven("test_cases.json")
    def test_skill(case):
        assert case.expected == case.input

    # Use assertion helpers
    from xiuxian_test_kit.asserts import assert_response_ok, assert_has_error
"""

from __future__ import annotations

__version__ = "0.1.0"

__all__: list[str] = []
