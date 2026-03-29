"""Tests for the removed Python ProjectMemory backend."""

from __future__ import annotations

from pathlib import Path

import pytest

from xiuxian_foundation.services.memory.base import (
    MEMORY_DIR,
    ProjectMemory,
    format_decision,
    init_memory_dir,
    parse_decision,
)


def test_format_decision_with_all_fields() -> None:
    decision = {
        "title": "Test Decision",
        "problem": "Problem statement",
        "solution": "Solution body",
        "rationale": "Rationale body",
        "status": "accepted",
        "author": "Claude",
        "date": "2026-01-30T10:00:00",
    }
    formatted = format_decision(decision)
    assert "# Decision: Test Decision" in formatted
    assert "Problem statement" in formatted
    assert "Solution body" in formatted
    assert "accepted" in formatted


def test_parse_decision_roundtrip() -> None:
    content = """# Decision: Test Decision
Date: 2026-01-30T10:00:00
Author: Claude

## Problem
Test problem statement

## Solution
Test solution

## Rationale
Test rationale

## Status
accepted
"""
    parsed = parse_decision(content)
    assert parsed["title"] == "Test Decision"
    assert parsed["status"] == "accepted"
    assert parsed["problem"] == "Test problem statement"


def test_init_memory_dir_still_creates_local_artifact_dirs(tmp_path: Path) -> None:
    assert init_memory_dir(tmp_path) is True
    assert (tmp_path / "decisions").exists()
    assert (tmp_path / "tasks").exists()
    assert (tmp_path / "context").exists()
    assert (tmp_path / "active_context").exists()


def test_project_memory_reports_removed_backend(tmp_path: Path) -> None:
    with pytest.raises(RuntimeError, match="Arrow Flight"):
        ProjectMemory(dir_path=tmp_path)


def test_memory_dir_constant_is_pathlike() -> None:
    assert isinstance(MEMORY_DIR, Path)
