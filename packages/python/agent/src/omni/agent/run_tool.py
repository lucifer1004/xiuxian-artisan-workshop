"""
run_tool.py - Entry point for running a single skill command (CLI or one-off).

Usage:
    echo '{"query": "architecture", "limit": 10}' | python -m omni.agent.run_tool knowledge recall

Input: JSON object with command arguments on stdin.
Output: JSON-serialized result on stdout. On failure, error message on stderr and exit(1).
"""

from __future__ import annotations

import asyncio
import json
import sys
from contextlib import suppress


def _main() -> int:
    if len(sys.argv) != 3:
        sys.stderr.write(
            "Usage: python -m omni.agent.run_tool <skill_name> <command_name>\n"
            "Reads JSON object of arguments from stdin.\n"
        )
        return 1

    skill_name = sys.argv[1]
    command_name = sys.argv[2]
    tool_name = f"{skill_name}.{command_name}"

    try:
        raw = sys.stdin.read()
        args = json.loads(raw) if raw.strip() else {}
    except json.JSONDecodeError as e:
        sys.stderr.write(f"Invalid JSON on stdin: {e}\n")
        return 1

    try:
        from omni.core.skills.runner import run_tool

        result = asyncio.run(run_tool(tool_name, args))
        # Serialize for JSON; allow non-dict result (e.g. str)
        if isinstance(result, (dict, list, str, int, float, bool, type(None))):
            out = result
        else:
            out = {"value": str(result), "_serialized": True}
        print(json.dumps(out, default=str, ensure_ascii=False))
        return 0
    except Exception as e:
        sys.stderr.write(f"{type(e).__name__}: {e}\n")
        with suppress(Exception):
            print(json.dumps({"success": False, "error": str(e)}, ensure_ascii=False))
        return 1


if __name__ == "__main__":
    sys.exit(_main())
