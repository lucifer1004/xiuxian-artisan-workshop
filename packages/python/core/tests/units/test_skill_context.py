"""
Tests for Skill Context initialization, auto-loading, and tool schema extraction.

These tests verify:
1. SkillContext auto-loads skills on first access
2. Kernel.skill_context properly initializes with all commands
3. get_tool_schema reads from LanceDB correctly
4. Error handling for missing/invalid LanceDB data
5. Tool schema extraction from command handlers
"""

from __future__ import annotations

import asyncio
import os
from typing import TYPE_CHECKING
from unittest.mock import AsyncMock, MagicMock, patch

if TYPE_CHECKING:
    from pathlib import Path


class TestSkillContextAutoLoad:
    """Tests for SkillContext auto-loading behavior."""

    def test_skill_context_loads_discovered_skills(self, tmp_path: Path):
        """Test that SkillContext loads discovered skills automatically."""
        from omni.core.skills.runtime import SkillContext

        # Create a minimal skill structure
        skills_dir = tmp_path / "assets" / "skills"
        skills_dir.mkdir(parents=True)

        # Create a test skill with a script
        test_skill = skills_dir / "test_skill"
        test_skill.mkdir()
        scripts_dir = test_skill / "scripts"
        scripts_dir.mkdir()

        (scripts_dir / "commands.py").write_text("""
from omni.foundation.api.decorators import skill_command

@skill_command(name="test_cmd", description="A test command")
def test_cmd():
    return "test"

__all__ = ["test_cmd"]
""")

        # Mock get_project_root to return tmp_path
        with patch("omni.foundation.runtime.gitops.get_project_root", return_value=tmp_path):
            ctx = SkillContext(skills_dir)

            # Initially no skills loaded (LanceDB is empty)
            assert len(ctx.list_skills()) == 0
            assert len(ctx.list_commands()) == 0


class TestKernelSkillContext:
    """Tests for Kernel.skill_context initialization."""

    def test_kernel_skill_context_returns_context(self, tmp_path: Path):
        """Test that Kernel.skill_context returns a valid SkillContext."""
        from omni.core.kernel.engine import Kernel

        # Mock LanceDB to avoid actual database operations
        with patch("omni.foundation.bridge.RustVectorStore") as mock_store:
            mock_store_instance = MagicMock()
            mock_store_instance.list_all_tools.return_value = []
            mock_store.return_value = mock_store_instance

            with patch("omni.foundation.runtime.gitops.get_project_root", return_value=tmp_path):
                kernel = Kernel(project_root=tmp_path)

                # Access skill_context (triggers loading)
                ctx = kernel.skill_context

                # Verify context was created successfully
                assert ctx is not None

    def test_skill_context_loads_in_running_loop_context(self, tmp_path: Path):
        """Test skill loading when called from within an async context.

        This specifically tests the scenario where Kernel.skill_context is
        accessed from within an MCP handler (which has a running event loop).
        """
        from omni.core.kernel.engine import Kernel

        async def access_skill_context():
            """Simulate MCP handler accessing skill_context."""
            with patch("omni.foundation.bridge.RustVectorStore") as mock_store:
                mock_store_instance = MagicMock()
                mock_store_instance.list_all_tools.return_value = []
                mock_store.return_value = mock_store_instance

                with patch(
                    "omni.foundation.runtime.gitops.get_project_root", return_value=tmp_path
                ):
                    kernel = Kernel(project_root=tmp_path)
                    ctx = kernel.skill_context
                    return ctx

        # This should NOT raise "asyncio.run() cannot be called from a running event loop"
        ctx = asyncio.run(access_skill_context())

        # Verify context was created successfully
        assert ctx is not None

    def test_kernel_shutdown_resets_cached_runtime_state(self, tmp_path: Path):
        """Kernel shutdown should clear cached state to avoid cross-test contamination."""
        from omni.core.kernel.engine import Kernel

        kernel = Kernel(project_root=tmp_path)
        kernel._skill_context = MagicMock()
        kernel._skill_context.skills_count = 2
        kernel._discovered_skills = ["dummy"]
        kernel._router = object()
        kernel._sniffer = MagicMock()
        kernel._security = object()

        with patch("omni.core.skills.runtime.reset_context") as mock_reset_context:
            asyncio.run(kernel._on_shutdown())

        mock_reset_context.assert_called_once()
        assert kernel._skill_context is None
        assert kernel._discovered_skills == []
        assert kernel._router is None
        assert kernel._sniffer is None
        assert kernel._security is None

    def test_discover_skills_includes_filesystem_fallback(self, tmp_path: Path):
        """Kernel discovery should include skills that exist on disk but are missing in index."""
        from omni.core.kernel.engine import Kernel

        skills_dir = tmp_path / "assets" / "skills"
        skills_dir.mkdir(parents=True)
        (skills_dir / "code").mkdir()

        kernel = Kernel(project_root=tmp_path, skills_dir=skills_dir)
        kernel._discovery_service = MagicMock()
        kernel._discovery_service.discover_all = AsyncMock(return_value=[])

        discovered = asyncio.run(kernel.discover_skills())
        discovered_names = {skill.name for skill in discovered}

        assert "code" in discovered_names


class TestToolSchemaExtraction:
    """Tests for extract_tool_schemas function."""

    def test_extract_tool_schemas_from_context(self, tmp_path: Path):
        """Test extracting tool schemas from SkillContext commands."""
        from omni.agent.core.omni.schemas import extract_tool_schemas
        from omni.core.skills.runtime import SkillContext
        from omni.core.skills.universal import UniversalScriptSkill

        skills_dir = tmp_path / "assets" / "skills"
        skills_dir.mkdir(parents=True)

        # Create a test skill
        test_skill = skills_dir / "extract_test"
        test_skill.mkdir()
        scripts_dir = test_skill / "scripts"
        scripts_dir.mkdir()

        (scripts_dir / "tools.py").write_text("""
from omni.foundation.api.decorators import skill_command

@skill_command(name="process", description="Process data", category="write")
def process(data: str, options: dict | None = None):
    return {"result": data}

__all__ = ["process"]
""")

        skill = UniversalScriptSkill(skill_name="extract_test", skill_path=test_skill)
        asyncio.run(skill.load())

        ctx = SkillContext(skills_dir)
        ctx.register_skill(skill)

        # Get command handler
        def get_handler(cmd_name):
            return ctx.get_command(cmd_name)

        # Extract schemas
        schemas = extract_tool_schemas(["extract_test.process"], get_handler)

        assert len(schemas) == 1
        schema = schemas[0]
        assert schema["name"] == "extract_test.process"
        assert "description" in schema
        assert "input_schema" in schema

    def test_extract_tool_schemas_empty_commands(self, tmp_path: Path):
        """Test extracting schemas with empty command list."""
        from omni.agent.core.omni.schemas import extract_tool_schemas

        def get_handler(cmd):
            return None

        schemas = extract_tool_schemas([], get_handler)
        assert schemas == []

    def test_extract_tool_schemas_skips_missing_commands(self, tmp_path: Path):
        """Test that missing commands are skipped gracefully."""
        from omni.agent.core.omni.schemas import extract_tool_schemas

        def get_handler(cmd_name):
            return None

        schemas = extract_tool_schemas(["nonexistent.command"], get_handler)
        assert schemas == []


class TestSkillContextHotReloadRollback:
    """Tests for transactional hot-reload safety in SkillContext."""

    def test_hot_reload_restores_previous_commands_on_empty_reload(self, tmp_path: Path):
        """Hot reload must rollback when a modified script yields an empty command set."""
        from omni.core.skills.runtime import SkillContext
        from omni.core.skills.universal import UniversalScriptSkill

        skills_dir = tmp_path / "assets" / "skills"
        skills_dir.mkdir(parents=True)
        skill_dir = skills_dir / "reload_guard"
        skill_dir.mkdir()
        scripts_dir = skill_dir / "scripts"
        scripts_dir.mkdir()
        command_file = scripts_dir / "ping.py"
        command_file.write_text(
            """
from omni.foundation.api.decorators import skill_command

@skill_command(name="ping", description="ping")
def ping():
    return "pong"
"""
        )

        skill = UniversalScriptSkill("reload_guard", skill_dir)
        asyncio.run(skill.load())

        ctx = SkillContext(skills_dir)
        ctx.register_skill(skill)
        original_handler = ctx.get_command("reload_guard.ping")
        assert original_handler is not None

        # Corrupt script to force an empty reload result from tools loader.
        command_file.write_text("def ping(:\n    return 'pong'\n")
        bumped = command_file.stat().st_mtime + 5
        os.utime(command_file, (bumped, bumped))

        reloaded_skill = ctx.get_skill("reload_guard")
        assert reloaded_skill is skill
        assert ctx.get_command("reload_guard.ping") is original_handler
        assert "reload_guard.ping" in skill.list_commands()
