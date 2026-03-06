"""XiuxianCellRunner - Python Interface to Rust XiuxianCell Executor.

Trinity Architecture - Core Layer

High-level Python wrapper for the Rust XiuxianCell executor.
Serves as the central nervous system for all OS interactions.

All methods return ToolResponse for unified MCP format.
"""

from __future__ import annotations

import logging
from concurrent.futures import ThreadPoolExecutor
from contextlib import suppress
from enum import Enum
from typing import Any

from pydantic import BaseModel, Field

from omni.core.errors import CoreErrorCode
from omni.core.responses import ToolResponse
from omni.foundation.utils import json_codec as json

logger = logging.getLogger(__name__)


class ActionType(str, Enum):
    """Command action classification."""

    OBSERVE = "observe"  # Read-only operations (ls, open, ps, cat)
    MUTATE = "mutate"  # Side-effect operations (cp, mv, rm, save)


class SysQueryResult(BaseModel):
    """Result from sys_query (code extraction). Used internally."""

    success: bool = Field(default=False, description="Whether extraction succeeded")
    items: list[dict[str, Any]] = Field(default_factory=list, description="Extracted code elements")
    count: int = Field(default=0, description="Number of items extracted")
    error: str | None = Field(default=None, description="Error message if failed")


class XiuxianCellRunner:
    """High-level Python wrapper for the Rust XiuxianCell executor.

    This runner provides:
    - Safe execution of Nushell commands with AST-based security analysis
    - Automatic JSON output parsing for structured data
    - Intent classification (observe vs mutate)
    - Fallback mode when Rust bridge is unavailable

    Example:
        >>> runner = XiuxianCellRunner()
        >>> result = await runner.run("ls -la", ActionType.OBSERVE)
        >>> if result.status == "success":
        ...     print(result.data)
    """

    def __init__(
        self,
        nu_path: str = "nu",
        enable_shellcheck: bool = True,
    ) -> None:
        """Initialize the XiuxianCell runner.

        Args:
            nu_path: Path to Nushell binary (default: "nu")
            enable_shellcheck: Enable ShellCheck validation (default: True)
        """
        self._rust_bridge: Any | None = None
        self._executor: ThreadPoolExecutor | None = None
        self._init_rust_bridge(nu_path, enable_shellcheck)

    def _init_rust_bridge(self, nu_path: str, enable_shellcheck: bool) -> None:
        """Initialize the Rust bridge binding."""
        try:
            # Import from compiled Rust extension
            from xiuxian_core_rs import PyOmniCell

            self._rust_bridge = PyOmniCell(nu_path=nu_path, enable_shellcheck=enable_shellcheck)
            # Dedicated worker thread reduces default-executor scheduling jitter for frequent
            # short-lived XiuxianCell calls.
            self._executor = ThreadPoolExecutor(max_workers=1, thread_name_prefix="xiuxiancell")
            logger.info("XiuxianCell Rust bridge initialized successfully")

        except ImportError:
            logger.warning(
                "Rust bridge not found. XiuxianCell running in degraded mode. "
                "Run `uv sync --reinstall-package xiuxian-core-rs` to enable."
            )
            self._rust_bridge = None
            self._executor = None

    def close(self) -> None:
        """Release runner resources."""
        if self._executor is not None:
            self._executor.shutdown(wait=False, cancel_futures=False)
            self._executor = None

    def __del__(self) -> None:
        """Best-effort resource cleanup."""
        with suppress(Exception):
            self.close()

    def classify(self, command: str) -> ActionType:
        """Classify command intent using Rust AST analyzer.

        Args:
            command: The command to classify

        Returns:
            ActionType.OBSERVE for read-only, ActionType.MUTATE for side-effects
        """
        if self._rust_bridge is not None:
            try:
                result = self._rust_bridge.classify(command)
                return ActionType(result)
            except Exception as e:
                logger.warning(f"Rust classification failed: {e}")

        # Fallback: Keyword-based classification
        return self._classify_by_keywords(command)

    async def run(
        self,
        command: str,
        action: ActionType | None = None,
        ensure_structured: bool = True,
    ) -> ToolResponse:
        """Execute a command via the Rust XiuxianCell kernel.

        Args:
            command: The Nushell command to execute
            action: Explicit intent (observe/mutate), auto-detected if None
            ensure_structured: Force JSON output for structured data

        Returns:
            ToolResponse with execution result
        """
        # Auto-classify if action not specified
        if action is None:
            action = self.classify(command)

        # Fast-path safety check in Python
        if action == ActionType.MUTATE:
            safety_result = self._check_mutation_safety(command)
            if not safety_result["safe"]:
                return ToolResponse.blocked(
                    reason=safety_result["reason"],
                    metadata={"command": command, "security_check": "mutation_safety"},
                )

        if self._rust_bridge is not None:
            return await self._run_via_rust(command, action, ensure_structured)

        # Degraded mode: Fallback to subprocess
        return await self._run_fallback(command, action)

    async def _run_via_rust(
        self,
        command: str,
        action: ActionType,
        ensure_structured: bool,
    ) -> ToolResponse:
        """Execute via Rust bridge (async wrapper for sync call with timeout)."""
        import asyncio

        try:
            # Run sync Rust bridge call in thread pool with timeout
            loop = asyncio.get_running_loop()
            execute_with_action = getattr(self._rust_bridge, "execute_with_action", None)
            if callable(execute_with_action):

                def execute_fn() -> Any:
                    return execute_with_action(command, action.value, ensure_structured)
            else:

                def execute_fn() -> Any:
                    return self._rust_bridge.execute(command, ensure_structured)

            try:
                raw_json = await asyncio.wait_for(
                    loop.run_in_executor(self._executor, execute_fn),
                    timeout=30.0,  # 30 second timeout
                )
            except TimeoutError:
                return ToolResponse.error(
                    message=f"Command timed out after 30 seconds: {command[:100]}...",
                    code=CoreErrorCode.TOOL_TIMEOUT.value,
                    metadata={
                        "command": command,
                        "error_type": "timeout",
                        "runner": "xiuxian-cell-rust",
                    },
                )

            # Parse the JSON string into Python objects
            data = json.loads(raw_json)

            # Clean up null results for mutations
            if action == ActionType.MUTATE:
                if data is None or data == "null":
                    data = {
                        "status": "success",
                        "message": "Mutation completed successfully",
                        "command": command[:100] + "..." if len(command) > 100 else command,
                    }
                # If data is a dict with null values, clean them up
                elif isinstance(data, dict):
                    cleaned = {k: v for k, v in data.items() if v is not None}
                    if cleaned != data:
                        data = cleaned

            return ToolResponse.success(
                data=data,
                metadata={
                    "runner": "xiuxian-cell-rust",
                    "mode": action.value,
                    "command": command,
                },
            )

        except json.JSONDecodeError as e:
            logger.error(f"JSON parse error: {e}")
            return ToolResponse.error(
                message=f"JSON parse error: {e}",
                code=CoreErrorCode.CELL_JSON_DECODE_ERROR.value,
                metadata={
                    "command": command,
                    "error_type": "json_decode",
                    "runner": "xiuxian-cell-rust",
                },
            )
        except Exception as e:
            logger.error(f"XiuxianCell execution failed: {e}")
            return ToolResponse.error(
                message=str(e),
                code=CoreErrorCode.CELL_EXECUTION_ERROR.value,
                metadata={
                    "command": command,
                    "error_type": type(e).__name__,
                    "runner": "xiuxian-cell-rust",
                },
            )

    async def _run_fallback(
        self,
        command: str,
        action: ActionType,
    ) -> ToolResponse:
        """Fallback execution via subprocess when Rust bridge unavailable."""
        import asyncio

        try:
            # Build command with JSON output
            final_cmd = f"{command} | to json --raw"

            # Execute via shell
            process = await asyncio.create_subprocess_shell(
                final_cmd,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
            )
            stdout, stderr = await process.communicate()

            if process.returncode != 0:
                return ToolResponse.error(
                    message=f"Command failed: {stderr.decode()}",
                    code=CoreErrorCode.CELL_SUBPROCESS_ERROR.value,
                    metadata={
                        "command": command,
                        "error_type": "subprocess",
                        "stderr": stderr.decode(),
                        "runner": "xiuxian-cell-fallback",
                    },
                )

            output = stdout.decode().strip()
            if not output:
                return ToolResponse.success(
                    data={"status": "mutation_complete"},
                    metadata={
                        "runner": "xiuxian-cell-fallback",
                        "mode": action.value,
                        "command": command,
                    },
                )

            # Try to parse as JSON
            try:
                data = json.loads(output)
            except json.JSONDecodeError:
                data = output

            return ToolResponse.success(
                data=data,
                metadata={
                    "runner": "xiuxian-cell-fallback",
                    "mode": action.value,
                    "command": command,
                },
            )

        except Exception as e:
            return ToolResponse.error(
                message=str(e),
                code=CoreErrorCode.CELL_EXECUTION_ERROR.value,
                metadata={
                    "command": command,
                    "error_type": type(e).__name__,
                    "runner": "xiuxian-cell-fallback",
                },
            )

    def _check_mutation_safety(self, command: str) -> dict[str, Any]:
        """Python-layer heuristic safety check (complements Rust AST analysis)."""
        cmd_lower = command.lower()

        # Block obvious dangers
        dangerous_patterns = [
            ("rm -rf /", "Root deletion not allowed"),
            ("mkfs", "Filesystem formatting not allowed"),
            (":(){ :|:& };:", "Fork bomb not allowed"),
        ]

        for pattern, reason in dangerous_patterns:
            if pattern in cmd_lower:
                return {"safe": False, "reason": reason}

        return {"safe": True, "reason": ""}

    def _classify_by_keywords(self, command: str) -> ActionType:
        """Fallback keyword-based classification."""
        cmd_lower = command.lower().strip()

        mutation_keywords = [
            "rm",
            "mv",
            "cp",
            "save",
            "touch",
            "mkdir",
            "chmod",
            "chown",
            "echo",
            "print",
            "write",
            "tee",
            "sed",
            "awk",
            "dd",
        ]

        for keyword in mutation_keywords:
            if cmd_lower.startswith(keyword):
                return ActionType.MUTATE

        return ActionType.OBSERVE

    async def sys_query(
        self,
        query: dict[str, Any],
        action: ActionType = ActionType.OBSERVE,
    ) -> ToolResponse:
        """Extract code elements from a file using AST patterns.

        Provides high-precision context extraction for large codebases.

        Args:
            query: Query specification with keys:
                - path: File path to extract from (required)
                - pattern: ast-grep pattern (e.g., "def $NAME") (required)
                - language: Programming language (optional, auto-detected from extension)
                - captures: List of capture names to include (optional)
            action: ActionType (only OBSERVE supported currently)

        Returns:
            ToolResponse with sys_query result

        Example:
            >>> result = await runner.sys_query({
            ...     "path": "src/main.py",
            ...     "pattern": "def $NAME",
            ...     "language": "python",
            ...     "captures": ["NAME"]
            ... })
            >>> print(result.data["items"][0]["captures"]["NAME"])
            'hello'
        """
        if action != ActionType.OBSERVE:
            return ToolResponse.error(
                message="sys_query only supports OBSERVE action type",
                code=CoreErrorCode.INVALID_ARGUMENT.value,
                metadata={"query": query, "action": action.value},
            )

        path = query.get("path")
        pattern = query.get("pattern")

        if not path or not pattern:
            return ToolResponse.error(
                message="Both 'path' and 'pattern' are required",
                code=CoreErrorCode.MISSING_REQUIRED.value,
                metadata={"query": query},
            )

        # Read file content
        content = await self._read_file(path)
        if content is None:
            return ToolResponse.error(
                message=f"Failed to read file: {path}",
                code=CoreErrorCode.STORAGE_READ_ERROR.value,
                metadata={"path": path},
            )

        # Get optional parameters
        language = query.get("language", "")
        captures = query.get("captures")

        # Call Rust extraction function
        try:
            from xiuxian_core_rs import py_extract_items

            raw_json = py_extract_items(
                content=content,
                pattern=pattern,
                language=language,
                captures=captures,
            )

            items = json.loads(raw_json)

            return ToolResponse.success(
                data={
                    "items": items,
                    "count": len(items),
                    "path": path,
                    "pattern": pattern,
                },
                metadata={
                    "query_type": "sys_query",
                    "language": language,
                    "captures": captures,
                },
            )

        except ImportError:
            logger.warning("Rust bridge not available for sys_query")
            return ToolResponse.error(
                message="Rust bridge not available. Run `uv sync --reinstall-package xiuxian-core-rs`",
                code=CoreErrorCode.DEPENDENCY_MISSING.value,
                metadata={"path": path, "pattern": pattern},
            )
        except json.JSONDecodeError as e:
            logger.error(f"JSON parse error in sys_query: {e}")
            return ToolResponse.error(
                message=f"JSON parse error: {e}",
                code=CoreErrorCode.CELL_JSON_DECODE_ERROR.value,
                metadata={"path": path, "pattern": pattern},
            )
        except Exception as e:
            logger.error(f"sys_query failed: {e}")
            return ToolResponse.error(
                message=str(e),
                code=CoreErrorCode.TOOL_EXECUTION_FAILED.value,
                metadata={"path": path, "pattern": pattern, "error_type": type(e).__name__},
            )

    async def _read_file(self, path: str) -> str | None:
        """Read file content using Rust bridge or fallback."""
        try:
            from xiuxian_core_rs import read_file_safe

            result = read_file_safe(path, max_bytes=1024 * 1024)  # 1MB limit
            if result and result.startswith("Error:"):
                logger.warning(f"read_file_safe failed: {result}")
                # Fallback to async file read
                return await self._read_file_async(path)
            return result

        except ImportError:
            return await self._read_file_async(path)
        except TypeError:
            # Fallback if signature mismatch
            return await self._read_file_async(path)
        except Exception as e:
            logger.error(f"Error reading file {path}: {e}")
            return await self._read_file_async(path)

    async def _read_file_async(self, path: str) -> str | None:
        """Async file read fallback using asyncio."""
        import asyncio

        try:
            loop = asyncio.get_running_loop()
            content = await loop.run_in_executor(None, lambda: self._read_file_sync(path))
            return content
        except Exception as e:
            logger.error(f"Async file read failed for {path}: {e}")
            return None

    def _read_file_sync(self, path: str) -> str | None:
        """Synchronous file read."""
        try:
            with open(path, encoding="utf-8") as f:
                content = f.read()
                # Truncate if too large (1MB limit)
                if len(content) > 1024 * 1024:
                    logger.warning(f"File {path} exceeds 1MB limit, truncating")
                    content = content[: 1024 * 1024]
                return content
        except Exception as e:
            logger.error(f"Sync file read failed for {path}: {e}")
            return None


# Module-level singleton for convenience
_default_runner: XiuxianCellRunner | None = None


def get_runner() -> XiuxianCellRunner:
    """Get the default XiuxianCellRunner instance."""
    global _default_runner
    if _default_runner is None:
        _default_runner = XiuxianCellRunner()
    return _default_runner


async def run_command(
    command: str,
    action: ActionType | None = None,
) -> ToolResponse:
    """Convenience function to run a command.

    Args:
        command: The command to execute
        action: Optional action type hint

    Returns:
        ToolResponse with execution results
    """
    runner = get_runner()
    return await runner.run(command, action)


async def sys_query(
    query: dict[str, Any],
    action: ActionType = ActionType.OBSERVE,
) -> ToolResponse:
    """Convenience function to extract code elements.

    Args:
        query: Query specification with path, pattern, language, captures
        action: ActionType (only OBSERVE supported)

    Returns:
        ToolResponse with extraction results
    """
    runner = get_runner()
    return await runner.sys_query(query, action)
