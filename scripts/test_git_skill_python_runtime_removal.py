from __future__ import annotations

from pathlib import Path


PROJECT_ROOT = Path(__file__).resolve().parents[1]
GIT_SKILL_ROOT = PROJECT_ROOT / "skills" / "git"


def test_git_skill_no_longer_ships_python_runtime() -> None:
    assert not any(GIT_SKILL_ROOT.rglob("*.py"))
    assert not any((GIT_SKILL_ROOT / "templates").rglob("*.j2"))
    assert not any((GIT_SKILL_ROOT / "tests").rglob("*.py"))
    assert not any((GIT_SKILL_ROOT / "scripts").rglob("*.py"))
    assert not any((GIT_SKILL_ROOT / "extensions" / "rust_bridge").rglob("*.py"))


def test_git_skill_docs_do_not_claim_local_runtime_scripts() -> None:
    skill_doc = (GIT_SKILL_ROOT / "SKILL.md").read_text(encoding="utf-8")
    readme = (GIT_SKILL_ROOT / "README.md").read_text(encoding="utf-8")

    forbidden = (
        "skills/git/scripts/",
        "git.commit",
        "git.stage_all",
        "Tool Runtime Calls",
        "Python local git runtime",
    )

    for pattern in forbidden:
        assert pattern not in skill_doc
        assert pattern not in readme
