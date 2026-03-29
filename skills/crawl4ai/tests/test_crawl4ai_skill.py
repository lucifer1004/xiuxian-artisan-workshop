"""Tests for crawl4ai skill."""

import json
import sys
from pathlib import Path
from unittest.mock import MagicMock, patch

import pytest


class TestCrawl4aiCommands:
    """Tests for crawl4ai callable surface."""

    def test_crawl_url_function_exists(self):
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from scripts.crawl_url import crawl_url

        assert callable(crawl_url)
        assert not hasattr(crawl_url, "_command_metadata")

    @pytest.mark.asyncio
    async def test_crawl_url_rejects_invalid_action(self, monkeypatch):
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from scripts.crawl_url import CrawlUrl

        from xiuxian_foundation.api.decorators import normalize_tool_result

        result = await CrawlUrl(url="https://example.com", action="invalid")
        payload = json.loads(normalize_tool_result(result)["content"][0]["text"])

        assert payload["status"] == "error"
        assert payload["action"] == "invalid"
        assert payload["message"] == "action must be one of: crawl, skeleton, smart"

    @pytest.mark.asyncio
    async def test_crawl_url_normalizes_action_case_and_whitespace(self, monkeypatch):
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from scripts import crawl_url as crawl_module
        from scripts.crawl_url import CrawlUrl

        from xiuxian_foundation.api.decorators import normalize_tool_result

        captured: dict[str, object] = {}

        def _fake_run_script_command(*, script_root, script_name, args, persistent=False):
            captured["args"] = args
            captured["persistent"] = persistent
            return {"success": True, "content": "ok"}

        monkeypatch.setattr(crawl_module, "run_script_command", _fake_run_script_command)
        monkeypatch.setattr(crawl_module, "_generate_chunk_plan", MagicMock(return_value=None))

        result = await CrawlUrl(url="https://example.com", action="  CrAwL  ")
        payload = json.loads(normalize_tool_result(result)["content"][0]["text"])

        assert payload.get("success") is True
        args = captured.get("args")
        assert isinstance(args, dict)
        assert args.get("action") == "crawl"
        assert captured.get("persistent") is True


class TestCrawl4aiScriptSurface:
    def test_crawl4ai_scripts_surface_exists(self):
        skill_path = Path(__file__).parent.parent
        assert (skill_path / "scripts" / "crawl_url.py").exists()
        assert (skill_path / "scripts" / "engine.py").exists()
        assert not (skill_path / "scripts" / "utils.py").exists()

    def test_crawl4ai_commands_are_importable(self):
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from scripts.crawl_url import crawl_url

        assert callable(crawl_url)

    def test_engine_py_has_no_command_metadata(self):
        skill_path = Path(__file__).parent.parent
        sys.path.insert(0, str(skill_path))
        import scripts.engine as engine_module

        for attr_name in dir(engine_module):
            if attr_name.startswith("_"):
                continue
            attr = getattr(engine_module, attr_name)
            if callable(attr) and not attr_name.startswith("_"):
                assert not getattr(attr, "_command_metadata", False)

    def test_crawl_url_uses_isolation(self):
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from scripts import crawl_url as crawl_module

        source = Path(crawl_module.__file__).read_text(encoding="utf-8")
        assert "run_script_command" in source


class TestCrawl4aiIsolation:
    def test_get_skill_dir_returns_correct_path(self):
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from scripts.crawl_url import _get_skill_dir

        skill_dir = _get_skill_dir()
        assert skill_dir.name == "crawl4ai"
        assert (skill_dir / "pyproject.toml").exists()
        assert (skill_dir / "scripts").exists()

    def test_run_script_command_returns_dict(self):
        from skills._shared.isolation import run_script_command

        skill_path = Path(__file__).parent.parent

        with patch("subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(
                returncode=0,
                stdout='{"success": true, "content": "test", "metadata": {}}',
                stderr="",
            )

            result = run_script_command(
                script_root=skill_path,
                script_name="engine.py",
                args={"url": "https://example.com"},
            )

            assert isinstance(result, dict)
            assert "success" in result


class TestCrawl4aiSkillDiscovery:
    def test_crawl4ai_skill_dir_exists_under_retained_skills_root(self):
        from xiuxian_foundation.config.prj import get_skills_dir

        assert (get_skills_dir() / "crawl4ai").exists()
