"""
Advanced Search Tools (Modernized)

Wraps modern Rust-based CLI tools for high-performance retrieval.
Provides superior [FIND] and [SEARCH] capabilities for the Agentic OS.
"""

import re
import shutil
import subprocess
import time
from copy import deepcopy
from functools import lru_cache
from os import walk
from pathlib import Path
from typing import Any

from xiuxian_foundation.config.logging import get_logger
from xiuxian_foundation.config.prj import get_project_root
from xiuxian_foundation.utils import json_codec as json

logger = get_logger("skill.advanced_tools.search")

_REGEX_META_PATTERN = re.compile(r"(?<!\\)[.^$*+?{}\[\]|()]")
_VIMGREP_LINE_PATTERN = re.compile(r"^(.*?):(\d+):(\d+):(.*)$")
_FILENAME_FAST_PATH_META_PATTERN = re.compile(r"[\\*?\[\]\(\)\{\}+^$|]")
_SMART_SEARCH_MAX_MATCHES = 300
_SMART_FIND_MAX_RESULTS = 100
_SMART_SEARCH_CACHE_TTL_SECONDS = 5.0
_SMART_SEARCH_RESULT_CACHE: dict[tuple[str, str], tuple[dict[str, Any], float]] = {}


@lru_cache(maxsize=8)
def _which_cached(command_name: str) -> str | None:
    """Resolve an executable once per process for lower per-call overhead."""
    return shutil.which(command_name)


def _resolve_exec(*candidates: str) -> str | None:
    """Return first available executable from candidate names."""
    for candidate in candidates:
        resolved = _which_cached(candidate)
        if resolved:
            return resolved
    return None


def _resolve_project_root(project_root: str | Path | None) -> str:
    """Resolve the project root for local CLI execution."""
    if project_root is None:
        return str(get_project_root())
    return str(Path(project_root).resolve())


def _run_command(cmd: list[str], root: str, timeout_seconds: float = 30.0) -> tuple[str, str, int]:
    """Run external command with deterministic subprocess settings."""
    process = subprocess.Popen(
        cmd,
        cwd=root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        stdin=subprocess.DEVNULL,
    )
    stdout, stderr = process.communicate(timeout=timeout_seconds)
    return stdout, stderr, process.returncode


def _run_rg_with_retry(cmd: list[str], root: str, max_retries: int = 2) -> tuple[str, str, int]:
    """Run rg with stdin handling and retry logic for transient errors."""
    for attempt in range(max_retries + 1):
        try:
            return _run_command(cmd, root, timeout_seconds=30.0)
        except Exception:
            if attempt < max_retries:
                time.sleep(0.1 * (attempt + 1))
            continue
    return "", "", 1


def _should_use_fixed_strings(pattern: str) -> bool:
    """Return True when pattern is a plain literal (safe for rg --fixed-strings)."""
    return _REGEX_META_PATTERN.search(pattern) is None


def _parse_vimgrep_line(line: str) -> dict[str, Any] | None:
    """Parse one `rg --vimgrep` output line into normalized match payload."""
    matched = _VIMGREP_LINE_PATTERN.match(line)
    if matched is None:
        return None

    file_path, line_text, _column_text, content = matched.groups()
    try:
        line_number = int(line_text)
    except ValueError:
        return None

    return {
        "file": file_path,
        "line": line_number,
        "content": content.strip(),
    }


def _smart_search_cache_key(
    *,
    root: str,
    resolved_search_root: str | None,
    pattern: str,
    file_globs: str | None,
    case_sensitive: bool,
    context_lines: int,
) -> tuple[str, str]:
    normalized_globs = str(file_globs or "").strip()
    normalized_root = str(resolved_search_root or "").strip()
    return (
        root,
        "|".join(
            (
                normalized_root,
                pattern,
                normalized_globs,
                "1" if case_sensitive else "0",
                str(max(0, int(context_lines))),
            )
        ),
    )


def _smart_search_cache_get(key: tuple[str, str]) -> dict[str, Any] | None:
    cached = _SMART_SEARCH_RESULT_CACHE.get(key)
    if cached is None:
        return None
    payload, expires_at = cached
    if time.monotonic() >= expires_at:
        _SMART_SEARCH_RESULT_CACHE.pop(key, None)
        return None
    return deepcopy(payload)


def _smart_search_cache_put(key: tuple[str, str], payload: dict[str, Any]) -> None:
    _SMART_SEARCH_RESULT_CACHE[key] = (
        deepcopy(payload),
        time.monotonic() + _SMART_SEARCH_CACHE_TTL_SECONDS,
    )


def clear_smart_search_cache() -> None:
    """Clear process-local smart_search cache."""
    _SMART_SEARCH_RESULT_CACHE.clear()


def _resolve_search_root(project_root: str, search_root: str | None) -> str | None:
    """Resolve optional scoped search root to an absolute existing path."""
    value = str(search_root or "").strip()
    if not value:
        return None

    candidate = Path(value)
    if not candidate.is_absolute():
        candidate = Path(project_root) / candidate
    resolved = candidate.resolve()
    if not resolved.exists():
        raise ValueError(f"search_root does not exist: {resolved}")
    return str(resolved)


def _is_smart_case_sensitive(pattern: str) -> bool:
    """Match fd smart-case behavior: uppercase forces case-sensitive matching."""
    return any(char.isupper() for char in pattern)


def _normalize_extension(extension: str | None) -> str | None:
    """Normalize optional extension filter for suffix matching."""
    if extension is None:
        return None
    value = extension.strip().lstrip(".").lower()
    return value or None


def _can_use_python_filename_fast_path(
    *,
    pattern: str,
    exclude: str | None,
    resolved_search_root: str | None,
) -> bool:
    """Enable Python filename fast-path only for safe literal scoped queries."""
    normalized_pattern = pattern.strip()
    if not resolved_search_root:
        return False
    if not normalized_pattern or normalized_pattern == ".":
        return False
    if exclude and exclude.strip():
        return False
    if "/" in normalized_pattern or "\\" in normalized_pattern:
        return False
    return _FILENAME_FAST_PATH_META_PATTERN.search(normalized_pattern) is None


def _python_fast_find_files(
    *,
    project_root: str,
    search_root: str,
    pattern: str,
    extension: str | None,
    max_results: int,
) -> list[str]:
    """Find files by literal filename matching without spawning external processes."""
    project_root_path = Path(project_root).resolve()
    search_root_path = Path(search_root).resolve()

    normalized_extension = _normalize_extension(extension)
    case_sensitive = _is_smart_case_sensitive(pattern)
    needle = pattern if case_sensitive else pattern.lower()
    matches: list[str] = []

    for current_root, dir_names, file_names in walk(search_root_path, topdown=True):
        dir_names[:] = sorted(name for name in dir_names if not name.startswith("."))
        for file_name in sorted(file_names):
            if file_name.startswith("."):
                continue
            if normalized_extension:
                file_extension = Path(file_name).suffix.lstrip(".").lower()
                if file_extension != normalized_extension:
                    continue

            haystack = file_name if case_sensitive else file_name.lower()
            if needle not in haystack:
                continue

            absolute_path = Path(current_root) / file_name
            try:
                relative_path = absolute_path.resolve().relative_to(project_root_path)
                matches.append(str(relative_path))
            except ValueError:
                matches.append(str(absolute_path.resolve()))

            if len(matches) >= max_results:
                return matches

    return matches


# =============================================================================
# Ripgrep (rg) - High Performance Content Search
# =============================================================================


def smart_search(
    pattern: str,
    file_globs: str | None = None,
    search_root: str | None = None,
    case_sensitive: bool = True,
    context_lines: int = 0,
    project_root: str | Path | None = None,
) -> dict[str, Any]:
    """Search using `rg --json`."""
    root = _resolve_project_root(project_root)

    rg_exec = _resolve_exec("rg")
    if not rg_exec:
        raise RuntimeError("Tool 'rg' (ripgrep) not found in path.")

    resolved_search_root = _resolve_search_root(root, search_root)
    cache_key = _smart_search_cache_key(
        root=root,
        resolved_search_root=resolved_search_root,
        pattern=pattern,
        file_globs=file_globs,
        case_sensitive=case_sensitive,
        context_lines=context_lines,
    )
    cached_payload = _smart_search_cache_get(cache_key)
    if cached_payload is not None:
        return cached_payload

    # Build ripgrep command
    use_vimgrep = context_lines <= 0
    cmd = [rg_exec, "--vimgrep", pattern] if use_vimgrep else [rg_exec, "--json", pattern]
    if _should_use_fixed_strings(pattern):
        cmd.append("--fixed-strings")
    cmd.extend(["--max-count", str(_SMART_SEARCH_MAX_MATCHES)])
    if not case_sensitive:
        cmd.append("--ignore-case")
    else:
        cmd.append("--case-sensitive")

    if context_lines > 0:
        cmd.extend(["--context", str(context_lines)])

    if file_globs:
        for glob in file_globs.split():
            cmd.extend(["-g", glob])
    if resolved_search_root:
        cmd.extend(["--", resolved_search_root])

    try:
        stdout, stderr, returncode = _run_rg_with_retry(cmd, root)

        if returncode > 1:
            raise RuntimeError(f"ripgrep error: {stderr}")

        matches = []
        file_matches = 0
        limit_reached = False

        if use_vimgrep:
            for line in stdout.splitlines():
                parsed = _parse_vimgrep_line(line)
                if parsed is None:
                    continue
                file_matches += 1
                if file_matches > _SMART_SEARCH_MAX_MATCHES:
                    limit_reached = True
                    continue
                matches.append(parsed)
        else:
            for line in stdout.splitlines():
                try:
                    data = json.loads(line)
                    if data["type"] == "match":
                        file_matches += 1
                        if file_matches > _SMART_SEARCH_MAX_MATCHES:
                            limit_reached = True
                            continue

                        matches.append(
                            {
                                "file": data["data"]["path"]["text"],
                                "line": data["data"]["line_number"],
                                "content": data["data"]["lines"]["text"].strip(),
                            }
                        )
                except (json.JSONDecodeError, KeyError):
                    continue

        if not matches:
            payload = {
                "success": False,
                "error": f"No matches found for pattern '{pattern}'",
                "tool": "ripgrep",
                "count": 0,
                "matches": [],
                "hint": "Try a different pattern or check for typos",
            }
            _smart_search_cache_put(cache_key, payload)
            return payload

        payload = {
            "success": True,
            "tool": "ripgrep",
            "count": len(matches),
            "matches": matches,
            "truncated": limit_reached,
        }
        _smart_search_cache_put(cache_key, payload)
        return payload

    except Exception as e:
        logger.error(f"Smart search failed: {e}")
        raise


# =============================================================================
# fd-find - Fast File Location and Discovery
# =============================================================================


def smart_find(
    pattern: str = ".",
    extension: str | None = None,
    exclude: str | None = None,
    search_root: str | None = None,
    project_root: str | Path | None = None,
    # Search mode: "filename" (default, uses fd) or "content" (uses rg)
    search_mode: str = "filename",
) -> dict[str, Any]:
    """Find files using 'fd' (by filename) or 'rg --files-with-matches' (by content)."""
    root = _resolve_project_root(project_root)
    resolved_search_root = _resolve_search_root(root, search_root)

    # Mode 1: Content Search (Delegates to ripgrep)
    if search_mode == "content":
        rg_exec = _resolve_exec("rg")
        if not rg_exec:
            raise RuntimeError("Tool 'rg' (ripgrep) not found.")

        cmd = [rg_exec, "--files-with-matches", "--max-count", "1", pattern]
        if _should_use_fixed_strings(pattern):
            cmd.append("--fixed-strings")
        if extension:
            cmd.extend(["--type", extension.replace(".", "")])
        if exclude:
            for excl in exclude.split():
                cmd.extend(["-g", f"!{excl}"])
        if resolved_search_root:
            cmd.extend(["--", resolved_search_root])

        try:
            stdout, stderr, returncode = _run_command(cmd, root, timeout_seconds=30.0)
            if returncode > 1:
                raise RuntimeError(f"ripgrep error: {stderr}")
            files = [line for line in stdout.splitlines() if line.strip()]
            if not files:
                return {
                    "success": False,
                    "error": f"No files found matching pattern '{pattern}'",
                    "search_mode": "content",
                    "count": 0,
                    "files": [],
                    "hint": "Try a different pattern or check for typos",
                }
            return {
                "success": True,
                "tool": "ripgrep",
                "search_mode": "content",
                "count": len(files),
                "files": files[:100],
            }
        except Exception as e:
            raise RuntimeError(f"Content search failed: {e}") from e

    # Mode 2: Filename Search (Uses fd)
    if _can_use_python_filename_fast_path(
        pattern=pattern,
        exclude=exclude,
        resolved_search_root=resolved_search_root,
    ):
        files = _python_fast_find_files(
            project_root=root,
            search_root=resolved_search_root,
            pattern=pattern,
            extension=extension,
            max_results=_SMART_FIND_MAX_RESULTS,
        )
        if not files:
            return {
                "success": False,
                "error": f"No files found matching pattern '{pattern}'",
                "search_mode": "filename",
                "count": 0,
                "files": [],
                "hint": "Try a different pattern or check for typos",
            }
        return {
            "success": True,
            "tool": "python",
            "search_mode": "filename",
            "count": len(files),
            "files": files[:100],
        }

    fd_exec = _resolve_exec("fd", "fdfind")
    if not fd_exec:
        raise RuntimeError("Tool 'fd' not found in system path.")

    cmd = [fd_exec, "--type", "f", "--max-results", str(_SMART_FIND_MAX_RESULTS)]  # files only
    if extension:
        cmd.extend(["--extension", extension])
    if exclude:
        cmd.extend(["--exclude", exclude])
    if resolved_search_root:
        cmd.extend(["--search-path", resolved_search_root])

    cmd.append(pattern)

    try:
        stdout, stderr, returncode = _run_command(cmd, root, timeout_seconds=30.0)
        if returncode != 0:
            raise RuntimeError(f"fd error: {stderr}")
        files = [line for line in stdout.splitlines() if line.strip()]
        if not files:
            return {
                "success": False,
                "error": f"No files found matching pattern '{pattern}'",
                "search_mode": "filename",
                "count": 0,
                "files": [],
                "hint": "Try a different pattern or check for typos",
            }
        return {
            "success": True,
            "tool": "fd",
            "search_mode": "filename",
            "count": len(files),
            "files": files[:100],
        }
    except Exception as e:
        raise RuntimeError(f"Filename search failed: {e}") from e


__all__ = ["smart_find", "smart_search"]
