"""
research_entry.py - Entry point for Sharded Deep Research Workflow

Uses Qianji Engine to run Repo_Analyzer_Array.
"""

from __future__ import annotations

import asyncio
import json
import subprocess
import urllib.parse
import uuid
from pathlib import Path
from typing import Any

from xiuxian_foundation.config.logging import get_logger
from xiuxian_foundation.config.prj import get_project_root
from xiuxian_foundation.config.prj import get_skills_dir
from skills._shared.cargo_subprocess_env import prepare_cargo_subprocess_env

logger = get_logger("researcher.entry")


async def _run_subprocess(
    args: list[str],
    *,
    cwd: str,
    text: bool = True,
) -> subprocess.CompletedProcess[str]:
    """Run subprocess in a worker thread to avoid blocking the event loop."""
    env = prepare_cargo_subprocess_env()
    logger.debug(
        "Prepared cargo subprocess env",
        pyo3_python=env.get("PYO3_PYTHON", "<unset>"),
    )
    return await asyncio.to_thread(
        subprocess.run,
        args,
        cwd=str(cwd),
        capture_output=True,
        text=text,
        env=env,
    )


async def run_qianji_engine(
    project_root: str, context: dict[str, Any], session_id: str
) -> tuple[bool, dict[str, Any], str]:
    """Execute the Qianji engine with repo_analyzer.toml."""
    manifest_path = str(get_skills_dir() / "researcher" / "workflows" / "repo_analyzer.toml")

    cmd = [
        "cargo",
        "run",
        "--release",
        "--quiet",
        "-p",
        "xiuxian-qianji",
        "--features",
        "llm",
        "--bin",
        "qianji",
        "--",
        project_root,
        manifest_path,
        json.dumps(context),
        session_id,
    ]

    # LLM runtime config is resolved by Rust (`qianji.toml` + user overrides + env).
    try:
        engine_root = str(get_project_root())
    except RuntimeError:
        engine_root = "."
    proc = await _run_subprocess(cmd, cwd=engine_root)

    if proc.returncode != 0:
        logger.error(f"Qianji Engine Failed: {proc.stderr}")
        return False, {}, proc.stderr

    # Parse output after "=== Final Qianji Execution Result ==="
    parts = proc.stdout.split("=== Final Qianji Execution Result ===")
    if len(parts) > 1:
        try:
            result_json = json.loads(parts[1].strip())
            return True, result_json, ""
        except json.JSONDecodeError as e:
            return False, {}, f"JSON decode error: {e}\nOutput: {parts[1]}"

    return False, {}, "Could not find result JSON marker in engine output."


async def run_research_graph(
    repo_url: str,
    request: str = "Analyze the architecture",
    action: str = "start",
    session_id: str = "",
    approved_shards: str = "",
) -> dict[str, Any]:
    """Execute the Sharded Deep Research workflow using Qianji Engine."""
    logger.info(
        "Sharded research workflow invoked via Qianji",
        repo_url=repo_url,
        request=request,
        action=action,
    )

    repo_name = urllib.parse.urlparse(repo_url).path.strip("/").split("/")[-1]
    if repo_name.endswith(".git"):
        repo_name = repo_name[:-4]

    if action == "start":
        session_id = str(uuid.uuid4())[:8]
        repo_dir = f"/tmp/xiuxian_research_{repo_name}_{session_id}"

        context = {
            "repo_url": repo_url,
            "repo_dir": repo_dir,
            "request": request,
            "project_root": repo_dir,
        }

        success, result_json, err = await run_qianji_engine(".", context, session_id)

        if not success:
            return {"success": False, "error": f"Workflow Failed: {err}"}

        suspend_prompt = result_json.get("suspend_prompt", "")
        analysis_trace = result_json.get("analysis_trace")
        proposed_plan = (
            [item for item in analysis_trace if isinstance(item, dict)]
            if isinstance(analysis_trace, list)
            else []
        )

        return {
            "success": True,
            "session_id": session_id,
            "message": suspend_prompt,
            "proposed_plan": proposed_plan,
            "next_action": "Call action='approve' with this session_id and approved_shards (JSON string)",
            "context": result_json,
        }

    elif action == "approve":
        if not session_id:
            return {"success": False, "error": "session_id is required for approve action"}
        if not approved_shards:
            return {
                "success": False,
                "error": "approved_shards JSON is required for approve action",
            }

        context = {
            "approved_shards": approved_shards,
        }

        success, result_json, err = await run_qianji_engine(".", context, session_id)

        if not success:
            return {"success": False, "error": f"Workflow Failed: {err}"}

        return {
            "success": True,
            "session_id": session_id,
            "analysis_result": result_json.get("analysis_result", ""),
            "full_context": result_json,
        }

    return {"success": False, "error": f"Unknown action: {action}"}


__all__ = ["run_research_graph"]
