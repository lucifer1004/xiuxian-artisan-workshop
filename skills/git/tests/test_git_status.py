"""Import smoke tests for git skill scripts."""

from __future__ import annotations

import sys
from pathlib import Path

SKILLS_ROOT = Path(__file__).parent.parent.parent
if str(SKILLS_ROOT) not in sys.path:
    sys.path.insert(0, str(SKILLS_ROOT))


class TestGitScripts:
    """Verify script modules and key entrypoints exist."""

    def test_commit_script_imports(self) -> None:
        from git.scripts import commit

        assert hasattr(commit, "commit")

    def test_prepare_script_imports(self) -> None:
        from git.scripts import prepare

        assert hasattr(prepare, "stage_and_scan")

    def test_render_script_imports(self) -> None:
        from git.scripts import rendering

        assert hasattr(rendering, "render_commit_message")

    def test_smart_commit_workflow_surface_is_absent(self) -> None:
        git_skill_root = Path(__file__).resolve().parents[1]

        assert not (git_skill_root / "scripts" / "commit_state.py").exists()
        assert not (git_skill_root / "scripts" / "smart_commit_graphflow").exists()
        assert not (git_skill_root / "workflows" / "smart_commit.toml").exists()
