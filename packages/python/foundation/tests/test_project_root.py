"""Tests for the retained project root helper."""

from __future__ import annotations

from pathlib import Path


class TestGetProjectRoot:
    def test_returns_path_object(self):
        from xiuxian_foundation.config.prj import get_project_root

        result = get_project_root()
        assert isinstance(result, Path)

    def test_returns_existing_directory(self):
        from xiuxian_foundation.config.prj import get_project_root

        result = get_project_root()
        assert result.exists()
        assert result.is_dir()

    def test_returns_absolute_path(self):
        from xiuxian_foundation.config.prj import get_project_root

        result = get_project_root()
        assert result.is_absolute()

    def test_contains_git_directory(self):
        from xiuxian_foundation.config.prj import get_project_root

        result = get_project_root()
        assert (result / ".git").exists()
