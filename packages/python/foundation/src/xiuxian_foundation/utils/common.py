"""Small project path helpers for retained Python packages."""

from __future__ import annotations

from functools import lru_cache
from pathlib import Path


@lru_cache(maxsize=1)
def project_root() -> Path:
    """Get project root (where .git lives)."""
    # Import from gitops which already does this correctly
    try:
        from xiuxian_foundation.runtime.gitops import get_project_root

        return get_project_root()
    except ImportError:
        # Fallback: find .git
        path = Path(__file__).resolve()
        for parent in path.parents:
            if (parent / ".git").exists():
                return parent
        return path.parent.parent.parent


__all__ = [
    "project_root",
]
