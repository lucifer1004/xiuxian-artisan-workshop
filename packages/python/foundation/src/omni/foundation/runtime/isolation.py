"""
isolation.py - Sidecar Execution Pattern for Heavy Skill Dependencies

This module provides a standardized way to run skill scripts in isolated
environments using uv, avoiding dependency conflicts in the main agent runtime.

Philosophy:
- Main agent environment stays clean (no heavy dependencies like crawl4ai)
- Each skill manages its own dependencies via pyproject.toml
- Communication via JSON through stdout/stderr

Usage:
    from omni.foundation.runtime.isolation import run_skill_command

    @skill_command
    def crawl_webpage(url: str):
        return run_skill_command(Path(__file__).parent, "engine.py", {"url": url})
"""

from __future__ import annotations

import atexit
import json
import os
import selectors
import subprocess
import threading
from contextlib import suppress
from dataclasses import dataclass, field
from importlib.util import find_spec
from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from pathlib import Path

_HAS_ORJSON = find_spec("orjson") is not None


def _json_loads(data: str | bytes) -> Any:
    """Fast JSON parsing using orjson if available."""
    import json

    if _HAS_ORJSON:
        import orjson as _orjson

        return _orjson.loads(data)
    return json.loads(data)


def _filter_skill_args(args: dict[str, Any]) -> dict[str, Any]:
    """Drop empty/None args before transport."""
    filtered: dict[str, Any] = {}
    for key, value in args.items():
        if value is None:
            continue
        if isinstance(value, str) and not value:
            continue
        if isinstance(value, (list, dict)) and len(value) == 0:
            continue
        filtered[key] = value
    return filtered


def _resolve_local_venv_python(skill_dir: Path) -> Path | None:
    """Return local virtualenv Python for a skill when available."""
    candidates = (
        skill_dir / ".venv" / "bin" / "python",
        skill_dir / ".venv" / "Scripts" / "python.exe",
    )
    for candidate in candidates:
        if candidate.exists() and candidate.is_file():
            return candidate
    return None


def _build_runner_command(
    skill_dir: Path, script_name: str, *, worker_mode: bool = False
) -> list[str]:
    """Build command to execute a skill script."""
    local_python = _resolve_local_venv_python(skill_dir)
    if local_python is not None:
        cmd = [str(local_python), f"scripts/{script_name}"]
    else:
        cmd = ["uv", "run", "--quiet", "python", f"scripts/{script_name}"]
    if worker_mode:
        cmd.append("--worker")
    return cmd


def _build_runner_env(skill_dir: Path) -> dict[str, str]:
    """Build environment for isolated skill execution."""
    env = os.environ.copy()
    env.setdefault("VIRTUAL_ENV", str(skill_dir / ".venv"))
    env.setdefault("UV_PROJECT_ENVIRONMENT", ".venv")
    return env


def _append_cli_args(cmd: list[str], args: dict[str, Any]) -> None:
    """Append filtered args to CLI command as --key value pairs."""
    for key, value in _filter_skill_args(args).items():
        cmd.append(f"--{key}")
        if isinstance(value, bool):
            cmd.append("true" if value else "false")
        elif isinstance(value, (list, dict)):
            cmd.append(json.dumps(value))
        else:
            cmd.append(str(value))


def _readline_with_timeout(proc: subprocess.Popen[str], timeout: int) -> str:
    """Read one stdout line with timeout from persistent worker."""
    stdout = proc.stdout
    if stdout is None:
        raise RuntimeError("Persistent worker stdout is not available")

    selector = selectors.DefaultSelector()
    try:
        selector.register(stdout, selectors.EVENT_READ)
        events = selector.select(timeout=max(0, timeout))
    finally:
        selector.close()

    if not events:
        raise TimeoutError(f"Persistent worker timed out after {timeout}s")

    line = stdout.readline()
    if not line:
        raise EOFError("Persistent worker exited without response")
    return line


@dataclass(slots=True)
class _PersistentSkillWorker:
    """Long-lived worker process for repeated isolated calls."""

    key: str
    cmd: list[str]
    cwd: str
    env: dict[str, str]
    proc: subprocess.Popen[str] | None = None
    lock: threading.Lock = field(default_factory=threading.Lock)

    def close(self) -> None:
        """Terminate worker process if running."""
        proc = self.proc
        self.proc = None
        if proc is None:
            return
        if proc.poll() is not None:
            return
        try:
            if proc.stdin is not None:
                proc.stdin.close()
        except Exception:
            pass
        try:
            proc.terminate()
            proc.wait(timeout=0.5)
        except Exception:
            with suppress(Exception):
                proc.kill()

    def _ensure_started(self) -> subprocess.Popen[str]:
        """Start worker process if not running."""
        if self.proc is not None and self.proc.poll() is None:
            return self.proc
        self.close()
        self.proc = subprocess.Popen(
            self.cmd,
            cwd=self.cwd,
            env=self.env,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            bufsize=1,
        )
        return self.proc

    def request(self, args: dict[str, Any], timeout: int) -> dict[str, Any]:
        """Send one request to worker and parse JSON response."""
        payload = _filter_skill_args(args)
        payload_text = json.dumps(payload, ensure_ascii=False, separators=(",", ":"))

        with self.lock:
            for attempt in range(2):
                proc = self._ensure_started()
                try:
                    if proc.stdin is None:
                        raise RuntimeError("Persistent worker stdin is not available")
                    proc.stdin.write(payload_text + "\n")
                    proc.stdin.flush()
                    line = _readline_with_timeout(proc, timeout)
                    result_data = _json_loads(line)
                    if isinstance(result_data, dict):
                        out = dict(result_data)
                        out.setdefault("success", True)
                        out.setdefault("content", "")
                        out.setdefault("metadata", {})
                        return out
                    return {"success": True, "content": str(result_data), "metadata": {}}
                except TimeoutError as e:
                    self.close()
                    return {"success": False, "error": str(e)}
                except (BrokenPipeError, EOFError, OSError, RuntimeError):
                    self.close()
                    if attempt == 1:
                        return {
                            "success": False,
                            "error": "Persistent worker communication failed",
                        }
                    continue
                except (ValueError, TypeError) as e:
                    return {
                        "success": False,
                        "error": f"Failed to parse JSON output: {e!s}",
                    }
                except Exception as e:
                    return {
                        "success": False,
                        "error": f"Unexpected error: {e!s}",
                    }
        return {"success": False, "error": "Persistent worker request failed"}


_PERSISTENT_WORKERS: dict[str, _PersistentSkillWorker] = {}
_PERSISTENT_WORKERS_LOCK = threading.Lock()


def _persistent_worker_key(skill_dir: Path, script_name: str) -> str:
    """Stable key for per-skill persistent workers."""
    return f"{skill_dir.resolve()}::{script_name}"


def _get_persistent_worker(skill_dir: Path, script_name: str) -> _PersistentSkillWorker:
    """Get or create persistent worker for the skill script."""
    key = _persistent_worker_key(skill_dir, script_name)
    with _PERSISTENT_WORKERS_LOCK:
        existing = _PERSISTENT_WORKERS.get(key)
        if existing is not None:
            return existing
        worker = _PersistentSkillWorker(
            key=key,
            cmd=_build_runner_command(skill_dir, script_name, worker_mode=True),
            cwd=str(skill_dir),
            env=_build_runner_env(skill_dir),
        )
        _PERSISTENT_WORKERS[key] = worker
        return worker


def _shutdown_persistent_workers() -> None:
    """Terminate all persistent workers."""
    with _PERSISTENT_WORKERS_LOCK:
        workers = list(_PERSISTENT_WORKERS.values())
        _PERSISTENT_WORKERS.clear()
    for worker in workers:
        worker.close()


atexit.register(_shutdown_persistent_workers)


def run_skill_command(
    skill_dir: Path,
    script_name: str,
    args: dict[str, Any],
    timeout: int = 60,
    persistent: bool = False,
) -> dict[str, Any]:
    """Run a skill script in an isolated uv environment.

    This function:
    1. Uses the skill's local pyproject.toml for dependency resolution
    2. Executes the script in a subprocess with proper isolation
    3. Captures and parses JSON output from stdout

    Args:
        skill_dir: Path to the skill root directory (contains pyproject.toml)
        script_name: Name of the script to run (e.g., "engine.py")
        args: Dictionary of arguments to pass to the script
        timeout: Maximum execution time in seconds (default 60)
        persistent: Reuse a long-lived worker process for repeated calls.

    Returns:
        Dictionary with 'success' key and either 'result' or 'error'

    Example:
        result = run_skill_command(
            Path(__file__).parent,
            "engine.py",
            {"url": "https://example.com", "fit_markdown": True}
        )
        if result["success"]:
            print(result["result"]["markdown"])
    """

    script_path = skill_dir / "scripts" / script_name

    if not script_path.exists():
        return {
            "success": False,
            "error": f"Script not found: {script_path}",
        }

    if persistent:
        worker = _get_persistent_worker(skill_dir, script_name)
        return worker.request(args, timeout)

    cmd = _build_runner_command(skill_dir, script_name, worker_mode=False)
    env = _build_runner_env(skill_dir)

    _append_cli_args(cmd, args)

    stdout = ""  # Initialize for exception handlers

    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=timeout,
            cwd=str(skill_dir),
            env=env,
        )

        # Parse stdout - engine.py outputs clean JSON
        stdout = result.stdout.strip()

        if result.returncode != 0 and not stdout:
            return {
                "success": False,
                "error": f"Script failed (exit code {result.returncode})",
                "stderr": result.stderr.strip() if result.stderr else None,
            }

        if not stdout:
            return {"success": True, "content": "", "metadata": {}}

        try:
            result_data = _json_loads(stdout)
            if isinstance(result_data, dict):
                out = dict(result_data)
                out.setdefault("success", result.returncode == 0)
                out.setdefault("content", "")
                out.setdefault("metadata", {})
                return out
            return {"success": result.returncode == 0, "content": stdout, "metadata": {}}
        except (ValueError, TypeError):
            # Fallback: treat entire stdout as content
            return {"success": True, "content": stdout, "metadata": {}}

    except subprocess.TimeoutExpired:
        return {
            "success": False,
            "error": f"Script timed out after {timeout}s",
        }
    except subprocess.CalledProcessError as e:
        return {
            "success": False,
            "error": f"Script failed (exit code {e.returncode})",
            "stderr": e.stderr.strip() if e.stderr else None,
        }
    except (ValueError, TypeError) as e:
        return {
            "success": False,
            "error": f"Failed to parse JSON output: {e!s}",
            "stdout": stdout[:500] if stdout else None,
        }
    except Exception as e:
        return {
            "success": False,
            "error": f"Unexpected error: {e!s}",
        }


def run_skill_command_async(
    skill_dir: Path,
    script_name: str,
    args: dict[str, Any],
    timeout: int = 60,
    persistent: bool = False,
) -> dict[str, Any]:
    """
    Async wrapper for run_skill_command.

    Note: subprocess.run is synchronous by nature. This wrapper exists
    for API compatibility with async code patterns.

    Args:
        skill_dir: Path to the skill root directory
        script_name: Name of the script to run
        args: Arguments to pass to the script
        timeout: Maximum execution time in seconds
        persistent: Reuse a long-lived worker process for repeated calls.

    Returns:
        Dictionary with 'success' key and either 'result' or 'error'
    """
    return run_skill_command(skill_dir, script_name, args, timeout, persistent)


def check_skill_dependencies(skill_dir: Path) -> dict[str, Any]:
    """
    Check if a skill's dependencies are installed.

    Runs 'uv sync --dry-run' to verify the environment without installing.

    Args:
        skill_dir: Path to the skill root directory

    Returns:
        Dictionary with 'ready' status and any messages
    """
    pyproject_path = skill_dir / "pyproject.toml"

    if not pyproject_path.exists():
        return {"ready": False, "error": "No pyproject.toml found"}

    try:
        result = subprocess.run(
            ["uv", "sync", "--dry-run", "--directory", str(skill_dir)],
            capture_output=True,
            text=True,
            timeout=30,
        )

        if result.returncode == 0:
            return {"ready": True, "message": "Dependencies satisfied"}
        else:
            return {
                "ready": False,
                "error": result.stderr.strip() or "Dependency resolution failed",
            }

    except subprocess.TimeoutExpired:
        return {"ready": False, "error": "Dependency check timed out"}
    except FileNotFoundError:
        return {"ready": False, "error": "uv not found in PATH"}


__all__ = [
    "check_skill_dependencies",
    "run_skill_command",
    "run_skill_command_async",
]
