"""Helpers for addressing files inside the on-disk skills tree."""

from pathlib import Path
from typing import Literal


def _resolve_skill_root(
    *,
    skill_dir: Path | None = None,
    _caller_file: str | None = None,
) -> Path:
    """Resolve the skill root for a caller or explicit path."""
    if skill_dir is not None:
        return skill_dir
    if _caller_file:
        return Path(_caller_file).parent

    caller_frame = _get_caller_frame()
    filename = getattr(caller_frame, "f_code", None) if caller_frame else None
    if filename:
        return Path(filename.co_filename).parent
    return Path(__file__).parent.parent / "skills"


def _skill_tree_path(
    relative_path: str,
    *,
    skill_dir: Path | None = None,
    _caller_file: str | None = None,
) -> Path:
    """Build a path under a resolved skill root."""
    return _resolve_skill_root(skill_dir=skill_dir, _caller_file=_caller_file) / relative_path


def skill_asset(relative_path: str, *, skill_dir: Path | None = None) -> Path:
    """
    Get path to a file in the skill's assets/ directory.

    Args:
        relative_path: Path relative to assets/ (e.g., "guide.md", "templates/config.json")
        skill_dir: Optional skill directory

    Example:
        from xiuxian_foundation.skill_utils import skill_asset

        guide = skill_asset("guide.md")
        template = skill_asset("templates/prompt.j2")

    Returns:
        Absolute Path to the asset
    """
    return _skill_tree_path(f"assets/{relative_path}", skill_dir=skill_dir)


def skill_script(relative_path: str, *, skill_dir: Path | None = None) -> Path:
    """
    Get path to a file in the skill's scripts/ directory.

    Args:
        relative_path: Path relative to scripts/ (e.g., "workflow.py", "utils.sh")
        skill_dir: Optional skill directory

    Example:
        from xiuxian_foundation.skill_utils import skill_script

        workflow = skill_script("workflow.py")
        script = skill_script("helpers.sh")

    Returns:
        Absolute Path to the script
    """
    return _skill_tree_path(f"scripts/{relative_path}", skill_dir=skill_dir)


def skill_reference(relative_path: str, *, skill_dir: Path | None = None) -> Path:
    """
    Get path to a file in the skill's references/ directory.

    Args:
        relative_path: Path relative to references/ (e.g., "docs.md", "architecture.md")
        skill_dir: Optional skill directory

    Example:
        from xiuxian_foundation.skill_utils import skill_reference

        doc = skill_reference("documentation.md")

    Returns:
        Absolute Path to the reference
    """
    return _skill_tree_path(f"references/{relative_path}", skill_dir=skill_dir)


def skill_data(relative_path: str, *, skill_dir: Path | None = None) -> Path:
    """
    Get path to a file in the skill's data/ directory.

    Args:
        relative_path: Path relative to data/ (e.g., "config.json", "data.csv")
        skill_dir: Optional skill directory

    Example:
        from xiuxian_foundation.skill_utils import skill_data

        config = skill_data("config.json")

    Returns:
        Absolute Path to the data file
    """
    return _skill_tree_path(f"data/{relative_path}", skill_dir=skill_dir)


# =============================================================================
# Internal Utilities
# =============================================================================


def _get_caller_frame():
    """Get the caller's stack frame (for path detection)."""
    import sys

    try:
        # Walk up the stack to find the caller (skip utility functions)
        frame = sys._getframe(2)  # Start 2 levels up (current + public helper)

        # Skip our own utility functions
        frame = _skip_internal_frames(frame)

        return frame
    except Exception:
        # If frame inspection fails, return a dummy frame-like object
        return None


def _skip_internal_frames(frame):
    """Skip internal/utility function frames."""
    import sys

    while frame:
        code = getattr(frame, "f_code", None)
        if code is None:
            frame = getattr(frame, "f_back", None)
            continue

        filename = code.co_filename
        func_name = code.co_name

        # Skip if in this module or common package
        if "skill_utils" in filename:
            frame = getattr(frame, "f_back", None)
            continue

        # Skip private/internal functions
        if func_name.startswith("_"):
            frame = getattr(frame, "f_back", None)
            continue

        return frame

    return frame


# =============================================================================
# Export
# =============================================================================

__all__ = [
    "skill_asset",
    "skill_script",
    "skill_reference",
    "skill_data",
]
