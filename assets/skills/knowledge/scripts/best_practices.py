"""
Knowledge Skill - Best Practices (The Architect)

Responsibilities:
- Bridge the gap between Documentation (Theory) and Codebase (Practice).
- Provide "Gold Standard" examples by searching both docs and actual usage.

Commands:
- get_best_practice: Retrieve documentation AND code examples for a topic.
"""

import json
import shutil
import subprocess
from concurrent.futures import ThreadPoolExecutor
from functools import lru_cache
from pathlib import Path
from typing import Any

from omni.foundation.api.decorators import skill_command
from omni.foundation.config.logging import get_logger
from omni.foundation.config.paths import ConfigPaths

logger = get_logger("skill.knowledge.best_practices")

_DOC_TARGETS: tuple[str, ...] = ("docs", "assets/references", "README.md")
_CODE_TARGETS: tuple[str, ...] = ("packages", "assets/skills")
_RG_EXECUTOR = ThreadPoolExecutor(max_workers=2, thread_name_prefix="knowledge-rg")


@lru_cache(maxsize=1)
def _resolve_rg_exec() -> str | None:
    """Resolve ripgrep executable once per process."""
    return shutil.which("rg")


def _existing_targets(root: Path, candidates: tuple[str, ...]) -> list[str]:
    """Return existing target paths for ripgrep."""
    return [str(path) for path in (root / item for item in candidates) if path.exists()]


def _parse_rg_json_output(stdout: str) -> list[dict[str, Any]]:
    """Parse ripgrep JSON output into structured results."""
    results = []
    for line in stdout.splitlines():
        try:
            data = json.loads(line)
            if data["type"] == "match":
                file_path = data["data"]["path"]["text"]
                # Skip irrelevant directories
                if any(
                    x in file_path for x in ["egg-info", "__pycache__", ".git", ".venv", "venv"]
                ):
                    continue
                results.append(
                    {
                        "file": file_path,
                        "line": data["data"]["line_number"],
                        "content": data["data"]["lines"]["text"].strip(),
                    }
                )
        except (json.JSONDecodeError, KeyError):
            continue
    return results


def _run_ripgrep(
    query: str, root: Path, targets: list[str], file_types: list[str]
) -> list[dict[str, Any]]:
    """Execute ripgrep search and return parsed results."""
    if not targets:
        return []

    rg_exec = _resolve_rg_exec()
    if not rg_exec:
        return []

    cmd = [rg_exec, "--json", "-i", query] + ["-t" + ft for ft in file_types] + targets

    try:
        process = subprocess.Popen(
            cmd, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True
        )
        stdout, stderr = process.communicate()
        if process.returncode > 1:
            logger.warning(f"ripgrep error: {stderr}")
            return []
        return _parse_rg_json_output(stdout)
    except Exception as e:
        logger.warning(f"ripgrep execution failed: {e}")
        return []


@skill_command(
    name="get_best_practice",
    description="Retrieve documentation AND code examples for a specific topic. The Architect's tool for bridging theory and practice.",
    autowire=True,
)
def get_best_practice(
    topic: str,
    paths: ConfigPaths | None = None,
) -> dict[str, Any]:
    """
    Bridge Documentation (Theory) and Codebase (Practice).

    This command provides a comprehensive view of how a concept is
    defined in documentation AND how it's actually used in the codebase.

    Args:
        topic: The concept/pattern to search for (e.g., "@skill_command", "async def").
        paths: ConfigPaths instance (auto-injected).

    Returns:
        dict with:
        - success: bool
        - topic: str
        - theory: dict with count and snippets (from docs)
        - practice: dict with count and examples (from code)
    """
    if paths is None:
        paths = ConfigPaths()

    root = paths.project_root

    rg_exec = _resolve_rg_exec()
    if not rg_exec:
        raise RuntimeError("ripgrep (rg) not found in PATH.")

    doc_targets = _existing_targets(root, _DOC_TARGETS)
    code_targets = _existing_targets(root, _CODE_TARGETS)

    # --- Step 1/2: Search theory and practice in parallel ---
    theory_future = _RG_EXECUTOR.submit(_run_ripgrep, topic, root, doc_targets, ["md", "markdown"])
    practice_future = _RG_EXECUTOR.submit(_run_ripgrep, topic, root, code_targets, ["py", "rust"])
    theory_results = theory_future.result()
    practice_results = practice_future.result()

    # Filter out test files from practice results
    practice_results = [
        r for r in practice_results if "/tests/" not in r["file"] and "test_" not in r["file"]
    ]

    # --- Step 3: Synthesis ---
    return {
        "success": True,
        "topic": topic,
        "theory": {
            "count": len(theory_results),
            "snippets": theory_results[:3],  # Top 3 doc hits
        },
        "practice": {
            "count": len(practice_results),
            "examples": practice_results[:5],  # Top 5 code usages
        },
    }


__all__ = ["get_best_practice"]
