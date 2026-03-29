"""Sidecar execution helpers for local CLI skill scripts."""

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


def _filter_script_args(args: dict[str, Any]) -> dict[str, Any]:
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


def _resolve_local_venv_python(script_root: Path) -> Path | None:
    """Return local virtualenv Python for a script package when available."""
    candidates = (
        script_root / ".venv" / "bin" / "python",
        script_root / ".venv" / "Scripts" / "python.exe",
    )
    for candidate in candidates:
        if candidate.exists() and candidate.is_file():
            return candidate
    return None


def _build_runner_command(
    script_root: Path, script_name: str, *, worker_mode: bool = False
) -> list[str]:
    """Build command to execute an isolated script entrypoint."""
    local_python = _resolve_local_venv_python(script_root)
    if local_python is not None:
        cmd = [str(local_python), f"scripts/{script_name}"]
    else:
        cmd = ["uv", "run", "--quiet", "python", f"scripts/{script_name}"]
    if worker_mode:
        cmd.append("--worker")
    return cmd


def _build_runner_env(script_root: Path) -> dict[str, str]:
    """Build environment for isolated script execution."""
    env = os.environ.copy()
    env.setdefault("VIRTUAL_ENV", str(script_root / ".venv"))
    env.setdefault("UV_PROJECT_ENVIRONMENT", ".venv")
    return env


def _append_cli_args(cmd: list[str], args: dict[str, Any]) -> None:
    """Append filtered args to CLI command as --key value pairs."""
    for key, value in _filter_script_args(args).items():
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
class _PersistentScriptWorker:
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
        payload = _filter_script_args(args)
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
                    return {"success": False, "error": f"Failed to parse JSON output: {e!s}"}
                except Exception as e:
                    return {"success": False, "error": f"Unexpected error: {e!s}"}
        return {"success": False, "error": "Persistent worker request failed"}


_PERSISTENT_WORKERS: dict[str, _PersistentScriptWorker] = {}
_PERSISTENT_WORKERS_LOCK = threading.Lock()


def _persistent_worker_key(script_root: Path, script_name: str) -> str:
    """Stable key for per-script persistent workers."""
    return f"{script_root.resolve()}::{script_name}"


def _get_persistent_worker(script_root: Path, script_name: str) -> _PersistentScriptWorker:
    """Get or create persistent worker for the script entrypoint."""
    key = _persistent_worker_key(script_root, script_name)
    with _PERSISTENT_WORKERS_LOCK:
        existing = _PERSISTENT_WORKERS.get(key)
        if existing is not None:
            return existing
        worker = _PersistentScriptWorker(
            key=key,
            cmd=_build_runner_command(script_root, script_name, worker_mode=True),
            cwd=str(script_root),
            env=_build_runner_env(script_root),
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


def run_script_command(
    script_root: Path,
    script_name: str,
    args: dict[str, Any],
    timeout: int = 60,
    persistent: bool = False,
) -> dict[str, Any]:
    """Run a script entrypoint in an isolated uv environment."""
    script_path = script_root / "scripts" / script_name

    if not script_path.exists():
        return {
            "success": False,
            "error": f"Script not found: {script_path}",
        }

    if persistent:
        worker = _get_persistent_worker(script_root, script_name)
        return worker.request(args, timeout)

    cmd = _build_runner_command(script_root, script_name, worker_mode=False)
    env = _build_runner_env(script_root)
    _append_cli_args(cmd, args)

    stdout = ""
    try:
        result = subprocess.run(
            cmd,
            capture_output=True,
            text=True,
            timeout=timeout,
            cwd=str(script_root),
            env=env,
        )
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
            return {"success": True, "content": stdout, "metadata": {}}
    except subprocess.TimeoutExpired:
        return {"success": False, "error": f"Script timed out after {timeout}s"}
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
        return {"success": False, "error": f"Unexpected error: {e!s}"}


__all__ = ["run_script_command"]
