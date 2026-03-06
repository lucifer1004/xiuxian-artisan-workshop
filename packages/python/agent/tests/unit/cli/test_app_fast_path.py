"""Tests for CLI fast path dispatch in app entrypoint."""

from __future__ import annotations

import importlib
import sys
import types

import pytest


def test_try_fast_skill_run_dispatches_json_to_json_runner(monkeypatch) -> None:
    """`skill run --json` should dispatch to JSON-only fast runner with reuse enabled."""
    app_module = importlib.import_module("omni.agent.cli.app")
    monkeypatch.setenv("OMNI_EMBED_OVERRIDE_ENABLED", "1")

    install_calls = {"count": 0}
    run_json_calls: dict[str, object] = {}

    def _install_override() -> None:
        install_calls["count"] += 1

    def _run_skills_json(commands, **kwargs) -> int:
        run_json_calls["commands"] = list(commands)
        run_json_calls["reuse_process"] = bool(kwargs.get("reuse_process", False))
        return 0

    monkeypatch.setitem(
        sys.modules,
        "omni.agent.embedding_override",
        types.SimpleNamespace(install_skill_embedding_override=_install_override),
    )
    monkeypatch.setitem(
        sys.modules,
        "omni.agent.cli.runner_json",
        types.SimpleNamespace(run_skills_json=_run_skills_json),
    )

    app_module._SKILL_EMBED_OVERRIDE_INSTALLED = False
    handled = app_module._try_fast_skill_run(
        ["skill", "run", "knowledge.search", '{"query":"x"}', "--json"]
    )

    assert handled is True
    assert run_json_calls["commands"] == ["knowledge.search", '{"query":"x"}']
    assert run_json_calls["reuse_process"] is True
    assert install_calls["count"] == 1


def test_try_fast_skill_run_help_flag_bypasses_fast_path() -> None:
    """`skill run --help` should fall back to Typer help path."""
    app_module = importlib.import_module("omni.agent.cli.app")
    handled = app_module._try_fast_skill_run(["skill", "run", "--help"])
    assert handled is False


def test_try_fast_skill_run_forwards_reuse_process_flag(monkeypatch) -> None:
    """`skill run --json --reuse-process` should forward reuse_process=True."""
    app_module = importlib.import_module("omni.agent.cli.app")
    monkeypatch.setenv("OMNI_EMBED_OVERRIDE_ENABLED", "1")

    run_json_calls: dict[str, object] = {}

    def _install_override() -> None:
        return None

    def _run_skills_json(commands, **kwargs) -> int:
        run_json_calls["commands"] = list(commands)
        run_json_calls["reuse_process"] = bool(kwargs.get("reuse_process", False))
        return 0

    monkeypatch.setitem(
        sys.modules,
        "omni.agent.embedding_override",
        types.SimpleNamespace(install_skill_embedding_override=_install_override),
    )
    monkeypatch.setitem(
        sys.modules,
        "omni.agent.cli.runner_json",
        types.SimpleNamespace(run_skills_json=_run_skills_json),
    )

    app_module._SKILL_EMBED_OVERRIDE_INSTALLED = False
    handled = app_module._try_fast_skill_run(
        ["skill", "run", "knowledge.search", '{"query":"x"}', "--json", "--reuse-process"]
    )

    assert handled is True
    assert run_json_calls["commands"] == ["knowledge.search", '{"query":"x"}']
    assert run_json_calls["reuse_process"] is True


def test_try_fast_skill_run_dispatches_plain_to_runner(monkeypatch) -> None:
    """`skill run` without --json should keep plain runner path with reuse enabled."""
    app_module = importlib.import_module("omni.agent.cli.app")
    monkeypatch.setenv("OMNI_EMBED_OVERRIDE_ENABLED", "1")

    install_calls = {"count": 0}
    run_calls: dict[str, object] = {}

    def _install_override() -> None:
        install_calls["count"] += 1

    def _run_skills(commands, *, json_output: bool = False, log_handler=None, **kwargs) -> None:
        run_calls["commands"] = list(commands)
        run_calls["json_output"] = json_output
        run_calls["has_log_handler"] = callable(log_handler)
        run_calls["reuse_process"] = bool(kwargs.get("reuse_process", False))

    monkeypatch.setitem(
        sys.modules,
        "omni.agent.embedding_override",
        types.SimpleNamespace(install_skill_embedding_override=_install_override),
    )
    monkeypatch.setitem(
        sys.modules,
        "omni.agent.cli.runner",
        types.SimpleNamespace(run_skills=_run_skills),
    )
    monkeypatch.setitem(
        sys.modules,
        "omni.agent.cli.console",
        types.SimpleNamespace(cli_log_handler=lambda _msg: None),
    )

    app_module._SKILL_EMBED_OVERRIDE_INSTALLED = False
    handled = app_module._try_fast_skill_run(["skill", "run", "knowledge.search", '{"query":"x"}'])

    assert handled is True
    assert run_calls["commands"] == ["knowledge.search", '{"query":"x"}']
    assert run_calls["json_output"] is False
    assert run_calls["reuse_process"] is True
    assert run_calls["has_log_handler"] is True
    assert install_calls["count"] == 1


def test_try_fast_skill_run_json_no_reuse_flag_disables_daemon(monkeypatch) -> None:
    """`skill run --json --no-reuse-process` should forward reuse_process=False."""
    app_module = importlib.import_module("omni.agent.cli.app")
    monkeypatch.setenv("OMNI_EMBED_OVERRIDE_ENABLED", "1")

    run_json_calls: dict[str, object] = {}

    def _install_override() -> None:
        return None

    def _run_skills_json(commands, **kwargs) -> int:
        run_json_calls["commands"] = list(commands)
        run_json_calls["reuse_process"] = bool(kwargs.get("reuse_process", False))
        return 0

    monkeypatch.setitem(
        sys.modules,
        "omni.agent.embedding_override",
        types.SimpleNamespace(install_skill_embedding_override=_install_override),
    )
    monkeypatch.setitem(
        sys.modules,
        "omni.agent.cli.runner_json",
        types.SimpleNamespace(run_skills_json=_run_skills_json),
    )

    app_module._SKILL_EMBED_OVERRIDE_INSTALLED = False
    handled = app_module._try_fast_skill_run(
        ["skill", "run", "knowledge.search", "--json", "--no-reuse-process"]
    )

    assert handled is True
    assert run_json_calls["commands"] == ["knowledge.search"]
    assert run_json_calls["reuse_process"] is False


def test_try_fast_skill_run_plain_forwards_reuse_process_flag(monkeypatch) -> None:
    """Plain fast path should forward reuse_process when `--reuse-process` is set."""
    app_module = importlib.import_module("omni.agent.cli.app")
    monkeypatch.setenv("OMNI_EMBED_OVERRIDE_ENABLED", "1")

    run_calls: dict[str, object] = {}

    def _install_override() -> None:
        return None

    def _run_skills(commands, *, json_output: bool = False, log_handler=None, **kwargs) -> None:
        run_calls["commands"] = list(commands)
        run_calls["json_output"] = json_output
        run_calls["reuse_process"] = bool(kwargs.get("reuse_process", False))
        run_calls["has_log_handler"] = callable(log_handler)

    monkeypatch.setitem(
        sys.modules,
        "omni.agent.embedding_override",
        types.SimpleNamespace(install_skill_embedding_override=_install_override),
    )
    monkeypatch.setitem(
        sys.modules,
        "omni.agent.cli.runner",
        types.SimpleNamespace(run_skills=_run_skills),
    )
    monkeypatch.setitem(
        sys.modules,
        "omni.agent.cli.console",
        types.SimpleNamespace(cli_log_handler=lambda _msg: None),
    )

    app_module._SKILL_EMBED_OVERRIDE_INSTALLED = False
    handled = app_module._try_fast_skill_run(
        ["skill", "run", "knowledge.search", "--reuse-process"]
    )

    assert handled is True
    assert run_calls["commands"] == ["knowledge.search"]
    assert run_calls["json_output"] is False
    assert run_calls["reuse_process"] is True
    assert run_calls["has_log_handler"] is True


def test_try_fast_skill_run_plain_no_reuse_flag_disables_daemon(monkeypatch) -> None:
    """Plain fast path should forward reuse_process=False when explicitly disabled."""
    app_module = importlib.import_module("omni.agent.cli.app")
    monkeypatch.setenv("OMNI_EMBED_OVERRIDE_ENABLED", "1")

    run_calls: dict[str, object] = {}

    def _install_override() -> None:
        return None

    def _run_skills(commands, *, json_output: bool = False, log_handler=None, **kwargs) -> None:
        run_calls["commands"] = list(commands)
        run_calls["json_output"] = json_output
        run_calls["reuse_process"] = bool(kwargs.get("reuse_process", False))
        run_calls["has_log_handler"] = callable(log_handler)

    monkeypatch.setitem(
        sys.modules,
        "omni.agent.embedding_override",
        types.SimpleNamespace(install_skill_embedding_override=_install_override),
    )
    monkeypatch.setitem(
        sys.modules,
        "omni.agent.cli.runner",
        types.SimpleNamespace(run_skills=_run_skills),
    )
    monkeypatch.setitem(
        sys.modules,
        "omni.agent.cli.console",
        types.SimpleNamespace(cli_log_handler=lambda _msg: None),
    )

    app_module._SKILL_EMBED_OVERRIDE_INSTALLED = False
    handled = app_module._try_fast_skill_run(
        ["skill", "run", "knowledge.search", "--no-reuse-process"]
    )

    assert handled is True
    assert run_calls["commands"] == ["knowledge.search"]
    assert run_calls["json_output"] is False
    assert run_calls["reuse_process"] is False
    assert run_calls["has_log_handler"] is True


def test_entry_point_skips_typer_when_fast_path_handles(monkeypatch) -> None:
    """When fast path handles argv, entry_point should return before Typer app()."""
    app_module = importlib.import_module("omni.agent.cli.app")

    state = {"app_called": 0, "register_called": 0}

    def _fake_bootstrap(_conf: str | None, _verbose: bool) -> None:
        return None

    def _fake_try_fast(_argv: list[str]) -> bool:
        return True

    def _fake_register(_command: str | None) -> None:
        state["register_called"] += 1

    def _fake_app() -> None:
        state["app_called"] += 1

    monkeypatch.setattr(app_module, "_bootstrap_configuration", _fake_bootstrap)
    monkeypatch.setattr(app_module, "_try_fast_skill_run", _fake_try_fast)
    monkeypatch.setattr(app_module, "_register_commands_for", _fake_register)
    monkeypatch.setattr(app_module, "app", _fake_app)
    monkeypatch.setattr(sys, "argv", ["omni", "skill", "run", "knowledge.search", "{}"])

    app_module.entry_point()

    assert state["register_called"] == 0
    assert state["app_called"] == 0


@pytest.mark.parametrize(
    ("raw", "expected"),
    [
        ("1", True),
        ("true", True),
        ("yes", True),
        ("on", True),
        ("0", False),
        ("false", False),
        ("no", False),
        ("off", False),
    ],
)
def test_embedding_override_enabled_prefers_explicit_env(
    monkeypatch, raw: str, expected: bool
) -> None:
    """Explicit OMNI_EMBED_OVERRIDE_ENABLED should override provider-derived behavior."""
    app_module = importlib.import_module("omni.agent.cli.app")
    monkeypatch.setenv("OMNI_EMBED_OVERRIDE_ENABLED", raw)
    monkeypatch.setattr(
        app_module,
        "get_settings",
        lambda: types.SimpleNamespace(get=lambda _key, _default=None: "legacy-provider"),
    )
    assert app_module._embedding_override_enabled() is expected


@pytest.mark.parametrize(
    ("provider", "expected"),
    [
        ("", True),
        ("client", True),
        ("fallback", False),
        ("legacy-provider", False),
    ],
)
def test_embedding_override_enabled_derives_from_provider(
    monkeypatch, provider: str, expected: bool
) -> None:
    """When env pin is absent, provider decides whether override is enabled."""
    app_module = importlib.import_module("omni.agent.cli.app")
    monkeypatch.delenv("OMNI_EMBED_OVERRIDE_ENABLED", raising=False)
    monkeypatch.setattr(
        app_module,
        "get_settings",
        lambda: types.SimpleNamespace(get=lambda _key, _default=None: provider),
    )
    assert app_module._embedding_override_enabled() is expected


def test_embedding_override_enabled_falls_back_true_on_settings_error(monkeypatch) -> None:
    """Fail-open preserves legacy override behavior when settings lookup fails."""
    app_module = importlib.import_module("omni.agent.cli.app")
    monkeypatch.delenv("OMNI_EMBED_OVERRIDE_ENABLED", raising=False)

    class _Boom:
        def get(self, *_args, **_kwargs):
            raise RuntimeError("boom")

    monkeypatch.setattr(app_module, "get_settings", lambda: _Boom())
    assert app_module._embedding_override_enabled() is True
