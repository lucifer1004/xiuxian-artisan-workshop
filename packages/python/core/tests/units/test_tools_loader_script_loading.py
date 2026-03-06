"""Tests for module reuse behavior in tools_loader_script_loading."""

from __future__ import annotations

from typing import TYPE_CHECKING

from omni.core.skills.tools_loader_script_loading import load_script

if TYPE_CHECKING:
    from pathlib import Path


class _NoopLogger:
    def debug(self, *_args, **_kwargs) -> None:
        return


def _write_counter_script(path: Path, *, label: str) -> None:
    path.write_text(
        f"""
import builtins

builtins._xiuxian_tools_loader_script_counter = getattr(
    builtins,
    "_xiuxian_tools_loader_script_counter",
    0,
) + 1

from omni.foundation.api.decorators import skill_command

@skill_command(name="recall", description="{label}")
def recall():
    return "ok"
""".strip(),
        encoding="utf-8",
    )


def _cleanup_skill_modules(skill_name: str) -> None:
    import sys

    prefix = f"{skill_name}."
    for module_name in list(sys.modules.keys()):
        if module_name.startswith(prefix):
            del sys.modules[module_name]


def test_load_script_reuses_module_when_enabled(tmp_path: Path) -> None:
    import builtins

    skill_name = "reuse_skill"
    scripts_path = tmp_path / skill_name / "scripts"
    scripts_path.mkdir(parents=True)
    script_path = scripts_path / "recall.py"
    _write_counter_script(script_path, label="v1")
    _cleanup_skill_modules(skill_name)
    if hasattr(builtins, "_xiuxian_tools_loader_script_counter"):
        delattr(builtins, "_xiuxian_tools_loader_script_counter")

    commands: dict[str, object] = {}
    loaded_count_1, reused_1 = load_script(
        script_path,
        f"{skill_name}.scripts",
        skill_name=skill_name,
        scripts_path=scripts_path,
        context={},
        commands=commands,
        logger=_NoopLogger(),
        allow_module_reuse=True,
    )
    loaded_count_2, reused_2 = load_script(
        script_path,
        f"{skill_name}.scripts",
        skill_name=skill_name,
        scripts_path=scripts_path,
        context={},
        commands=commands,
        logger=_NoopLogger(),
        allow_module_reuse=True,
    )

    assert loaded_count_1 == 1
    assert loaded_count_2 == 1
    assert reused_1 is False
    assert reused_2 is True
    assert builtins._xiuxian_tools_loader_script_counter == 1
    delattr(builtins, "_xiuxian_tools_loader_script_counter")


def test_load_script_reloads_when_file_changes(tmp_path: Path) -> None:
    import builtins
    import time

    skill_name = "reload_skill"
    scripts_path = tmp_path / skill_name / "scripts"
    scripts_path.mkdir(parents=True)
    script_path = scripts_path / "recall.py"
    _write_counter_script(script_path, label="v1")
    _cleanup_skill_modules(skill_name)
    if hasattr(builtins, "_xiuxian_tools_loader_script_counter"):
        delattr(builtins, "_xiuxian_tools_loader_script_counter")

    commands: dict[str, object] = {}
    _ = load_script(
        script_path,
        f"{skill_name}.scripts",
        skill_name=skill_name,
        scripts_path=scripts_path,
        context={},
        commands=commands,
        logger=_NoopLogger(),
        allow_module_reuse=True,
    )

    time.sleep(0.002)
    _write_counter_script(script_path, label="v2")
    _loaded_count_2, reused_2 = load_script(
        script_path,
        f"{skill_name}.scripts",
        skill_name=skill_name,
        scripts_path=scripts_path,
        context={},
        commands=commands,
        logger=_NoopLogger(),
        allow_module_reuse=True,
    )

    assert reused_2 is False
    assert builtins._xiuxian_tools_loader_script_counter == 2
    delattr(builtins, "_xiuxian_tools_loader_script_counter")


def test_load_script_handles_non_package_root_module_collision(tmp_path: Path) -> None:
    import sys
    import types

    skill_name = "code"
    scripts_root = tmp_path / skill_name / "scripts"
    search_dir = scripts_root / "search"
    search_dir.mkdir(parents=True)
    commands_file = search_dir / "commands.py"
    graph_file = search_dir / "graph.py"

    graph_file.write_text(
        """
async def execute_search(query: str, session_id: str):
    return {"final_output": f"<search>{query}</search>"}
""".strip(),
        encoding="utf-8",
    )
    commands_file.write_text(
        """
from omni.foundation.api.decorators import skill_command
from .graph import execute_search

@skill_command(name="code_search", description="test")
async def code_search(query: str, session_id: str = "default") -> str:
    result = await execute_search(query, session_id)
    return str(result.get("final_output", ""))
""".strip(),
        encoding="utf-8",
    )

    original_code_module = sys.modules.get("code")
    fake_code = types.ModuleType("code")
    fake_code.marker = "stdlib-like-non-package"
    sys.modules["code"] = fake_code
    _cleanup_skill_modules(skill_name)

    try:
        commands: dict[str, object] = {}
        loaded_count, reused = load_script(
            commands_file,
            f"{skill_name}.scripts.search",
            skill_name=skill_name,
            scripts_path=scripts_root,
            context={},
            commands=commands,
            logger=_NoopLogger(),
            allow_module_reuse=True,
        )

        assert loaded_count == 1
        assert reused is False
        assert "code.code_search" in commands
        assert hasattr(sys.modules["code"], "__path__")
        assert str(scripts_root) in sys.modules["code"].__path__
    finally:
        _cleanup_skill_modules(skill_name)
        if original_code_module is None:
            sys.modules.pop("code", None)
        else:
            sys.modules["code"] = original_code_module


def test_load_script_handles_root_module_with_tuple_path(tmp_path: Path) -> None:
    import sys
    import types

    skill_name = "code"
    scripts_root = tmp_path / skill_name / "scripts"
    search_dir = scripts_root / "search"
    search_dir.mkdir(parents=True)
    commands_file = search_dir / "commands.py"
    graph_file = search_dir / "graph.py"

    graph_file.write_text(
        """
async def execute_search(query: str, session_id: str):
    return {"final_output": f"<search>{query}</search>"}
""".strip(),
        encoding="utf-8",
    )
    commands_file.write_text(
        """
from omni.foundation.api.decorators import skill_command
from .graph import execute_search

@skill_command(name="code_search", description="test")
async def code_search(query: str, session_id: str = "default") -> str:
    result = await execute_search(query, session_id)
    return str(result.get("final_output", ""))
""".strip(),
        encoding="utf-8",
    )

    original_code_module = sys.modules.get("code")
    fake_code = types.ModuleType("code")
    fake_code.__path__ = ("/tmp/non-mutable-path",)
    sys.modules["code"] = fake_code
    _cleanup_skill_modules(skill_name)

    try:
        commands: dict[str, object] = {}
        loaded_count, reused = load_script(
            commands_file,
            f"{skill_name}.scripts.search",
            skill_name=skill_name,
            scripts_path=scripts_root,
            context={},
            commands=commands,
            logger=_NoopLogger(),
            allow_module_reuse=True,
        )

        assert loaded_count == 1
        assert reused is False
        assert "code.code_search" in commands
        assert isinstance(sys.modules["code"].__path__, list)
        assert str(scripts_root) in sys.modules["code"].__path__
    finally:
        _cleanup_skill_modules(skill_name)
        if original_code_module is None:
            sys.modules.pop("code", None)
        else:
            sys.modules["code"] = original_code_module


def test_load_script_recovers_commands_module_exports_without_decorator(tmp_path: Path) -> None:
    skill_name = "fallback_skill"
    scripts_path = tmp_path / skill_name / "scripts"
    scripts_path.mkdir(parents=True)
    script_path = scripts_path / "commands.py"
    script_path.write_text(
        """
__all__ = ["echo"]

def echo(message: str = "ok") -> str:
    return message
""".strip(),
        encoding="utf-8",
    )
    _cleanup_skill_modules(skill_name)

    commands: dict[str, object] = {}
    loaded_count, reused = load_script(
        script_path,
        f"{skill_name}.scripts",
        skill_name=skill_name,
        scripts_path=scripts_path,
        context={},
        commands=commands,
        logger=_NoopLogger(),
        allow_module_reuse=False,
    )

    assert loaded_count == 1
    assert reused is False
    assert "fallback_skill.echo" in commands
    recovered = commands["fallback_skill.echo"]
    assert getattr(recovered, "_is_skill_command", False) is True
