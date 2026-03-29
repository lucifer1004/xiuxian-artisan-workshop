"""
Skills tree path utility tests.

Tests for retained helpers in ``xiuxian_foundation.utils.skills``.
"""

from pathlib import Path


class TestSkillsTreePathUtilities:
    """Test retained helpers for the on-disk skills tree."""

    def test_skill_asset_exists(self):
        """Test that skill_asset function exists."""
        from xiuxian_foundation.utils.skills import skill_asset

        assert callable(skill_asset)

    def test_skill_script_path_builder_exists(self):
        """Test that the scripts-path helper exists."""
        from xiuxian_foundation.utils.skills import skill_script

        assert callable(skill_script)

    def test_skill_reference_exists(self):
        """Test that skill_reference function exists."""
        from xiuxian_foundation.utils.skills import skill_reference

        assert callable(skill_reference)

    def test_skill_data_exists(self):
        """Test that skill_data function exists."""
        from xiuxian_foundation.utils.skills import skill_data

        assert callable(skill_data)


class TestSkillsTreePathBuilding:
    """Test skills-tree path building with explicit skill_dir."""

    def test_skill_asset_with_explicit_dir(self, tmp_path: Path):
        """Test skill_asset with explicit skill_dir."""
        from xiuxian_foundation.utils.skills import skill_asset

        test_skill_dir = tmp_path / "skills" / "git"
        result = skill_asset("guide.md", skill_dir=test_skill_dir)

        assert str(result) == str(test_skill_dir / "assets/guide.md")

    def test_skill_script_path_builder_with_explicit_dir(self, tmp_path: Path):
        """Test scripts-path helper with explicit skill_dir."""
        from xiuxian_foundation.utils.skills import skill_script

        test_skill_dir = tmp_path / "skills" / "git"
        result = skill_script("workflow.py", skill_dir=test_skill_dir)

        assert str(result) == str(test_skill_dir / "scripts/workflow.py")

    def test_skill_reference_with_explicit_dir(self, tmp_path: Path):
        """Test skill_reference with explicit skill_dir."""
        from xiuxian_foundation.utils.skills import skill_reference

        test_skill_dir = tmp_path / "skills" / "git"
        result = skill_reference("docs.md", skill_dir=test_skill_dir)

        assert str(result) == str(test_skill_dir / "references/docs.md")

    def test_skill_data_with_explicit_dir(self, tmp_path: Path):
        """Test skill_data with explicit skill_dir."""
        from xiuxian_foundation.utils.skills import skill_data

        test_skill_dir = tmp_path / "skills" / "git"
        result = skill_data("config.json", skill_dir=test_skill_dir)

        assert str(result) == str(test_skill_dir / "data/config.json")
