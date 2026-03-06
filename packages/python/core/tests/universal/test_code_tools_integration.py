"""Universal integration tests for code search skill loading and execution."""

from __future__ import annotations

import pytest

from omni.core.skills.universal import UniversalScriptSkill
from omni.foundation.config.skills import SKILLS_DIR


@pytest.fixture
async def code_skill():
    """Load code skill directly to avoid kernel-level watcher/runtime side effects."""
    skill = UniversalScriptSkill(
        skill_name="code",
        skill_path=SKILLS_DIR() / "code",
    )
    await skill.load(context={"allow_module_reuse": False})
    return skill


def _extract_command_module_globals(command) -> dict | None:
    """Recover original command module globals from decorated function wrappers."""
    command_fn = getattr(command, "__wrapped__", command)
    command_globals = getattr(command_fn, "__globals__", None)

    if isinstance(command_globals, dict) and "_get_code_search_executor" in command_globals:
        return command_globals

    for cell in getattr(command_fn, "__closure__", ()) or ():
        candidate = getattr(cell, "cell_contents", None)
        candidate_globals = getattr(candidate, "__globals__", None)
        if callable(candidate) and isinstance(candidate_globals, dict):
            if "_get_code_search_executor" in candidate_globals:
                return candidate_globals
    return None


@pytest.mark.asyncio
async def test_code_search_integration(code_skill):
    """Verify command discovery and lightweight structural execution."""
    commands = code_skill.list_commands()
    assert "code.code_search" in commands

    result = await code_skill.execute(
        "code.code_search",
        query="class NonExistentClassXYZ123",
    )
    text = result if isinstance(result, str) else str(result)
    assert "<search_interaction" in text or "<search_results" in text or "SEARCH:" in text

    result_with_session = await code_skill.execute(
        "code.code_search",
        query="def code_search",
        session_id="test_session",
    )
    assert result_with_session


@pytest.mark.asyncio
async def test_modular_relative_imports_integration(code_skill):
    """Verify relative imports in code search commands resolve correctly."""
    command = code_skill.get_command("code.code_search")
    assert command is not None

    command_globals = _extract_command_module_globals(command)
    if not isinstance(command_globals, dict):
        pytest.skip("code_search command globals are unavailable for modular import assertion")

    get_executor = command_globals.get("_get_code_search_executor")
    assert callable(get_executor), "code_search must expose _get_code_search_executor"

    command_globals["_CODE_SEARCH_EXECUTOR"] = None
    executor = get_executor()
    assert callable(executor), "Relative import '.graph.execute_search' should resolve"

    async def _stub_executor(query: str, session_id: str) -> dict:
        del query, session_id
        return {"final_output": "<search_interaction><status>ok</status></search_interaction>"}

    command_globals["_CODE_SEARCH_EXECUTOR"] = _stub_executor
    clear_cache = command_globals.get("clear_code_search_cache")
    if callable(clear_cache):
        clear_cache()

    result = await code_skill.execute(
        "code.code_search",
        query="def code_search",
        session_id="modular_import_test",
    )
    text = result if isinstance(result, str) else str(result)
    assert "<search_interaction" in text
