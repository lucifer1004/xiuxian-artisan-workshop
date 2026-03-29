"""Tests for xiuxian_core.context module."""

from __future__ import annotations

import pytest

from xiuxian_core.context import (
    ContextOrchestrator,
    ContextProvider,
    ContextResult,
    SystemPersonaProvider,
    create_executor_orchestrator,
    create_planner_orchestrator,
)


class TestContextResult:
    """Test ContextResult dataclass."""

    def test_creation(self):
        """Test basic creation."""
        result = ContextResult(
            content="<test>content</test>",
            token_count=10,
            name="test_provider",
            priority=5,
        )
        assert result.content == "<test>content</test>"
        assert result.token_count == 10
        assert result.name == "test_provider"
        assert result.priority == 5

    def test_equality(self):
        """Test equality comparison."""
        r1 = ContextResult(content="a", token_count=1, name="n", priority=0)
        r2 = ContextResult(content="a", token_count=1, name="n", priority=0)
        r3 = ContextResult(content="b", token_count=1, name="n", priority=0)
        assert r1 == r2
        assert r1 != r3


class TestSystemPersonaProvider:
    """Test SystemPersonaProvider."""

    def test_default_role(self):
        """Test default architect role."""
        provider = SystemPersonaProvider()
        assert provider.role == "architect"

    def test_custom_role(self):
        """Test custom role assignment."""
        provider = SystemPersonaProvider(role="developer")
        assert provider.role == "developer"

    def test_unknown_role(self):
        """Test unknown role uses fallback."""
        provider = SystemPersonaProvider(role="unknown_role")
        assert provider.role == "unknown_role"

    @pytest.mark.asyncio
    async def test_provide_returns_content(self):
        """Test provide returns ContextResult with content."""
        provider = SystemPersonaProvider(role="architect")
        result = await provider.provide({}, 1000)

        assert isinstance(result, ContextResult)
        assert result.token_count > 0
        assert "architect" in result.content
        assert result.priority == 0
        assert result.name == "persona"

    @pytest.mark.asyncio
    async def test_provide_ignores_budget(self):
        """Test that persona ignores budget (always included)."""
        provider = SystemPersonaProvider()
        result = await provider.provide({}, 1)

        # Persona should be included regardless of tiny budget
        assert result.token_count > 0

    @pytest.mark.asyncio
    async def test_all_personas_exist(self):
        """Test that all default personas are accessible."""
        for role in ["architect", "developer", "researcher"]:
            provider = SystemPersonaProvider(role=role)
            result = await provider.provide({}, 1000)
            assert "<role>You are a" in result.content

    @pytest.mark.asyncio
    async def test_system_core_prompt_uses_relative_path_from_project_root(
        self, tmp_path, monkeypatch
    ):
        """Relative prompts.system_core should resolve from project root."""
        prompt_path = tmp_path / "custom_prompts" / "system_core.md"
        prompt_path.parent.mkdir(parents=True)
        prompt_path.write_text("SYSTEM CORE RELATIVE", encoding="utf-8")

        monkeypatch.setattr(
            "xiuxian_foundation.config.get_setting",
            lambda key, default=None: "custom_prompts/system_core.md"
            if key == "prompts.system_core"
            else default,
        )
        monkeypatch.setattr(
            "xiuxian_foundation.config.get_config_paths",
            lambda: type("P", (), {"project_root": tmp_path})(),
        )

        provider = SystemPersonaProvider(role="architect")
        result = await provider.provide({}, 1000)
        assert "SYSTEM CORE RELATIVE" in result.content

    @pytest.mark.asyncio
    async def test_system_core_prompt_supports_absolute_path(self, tmp_path, monkeypatch):
        """Absolute prompts.system_core should be used directly."""
        prompt_path = tmp_path / "absolute_system_core.md"
        prompt_path.write_text("SYSTEM CORE ABS", encoding="utf-8")

        monkeypatch.setattr(
            "xiuxian_foundation.config.get_setting",
            lambda key, default=None: str(prompt_path) if key == "prompts.system_core" else default,
        )
        monkeypatch.setattr(
            "xiuxian_foundation.config.get_config_paths",
            lambda: type("P", (), {"project_root": tmp_path})(),
        )

        provider = SystemPersonaProvider(role="architect")
        result = await provider.provide({}, 1000)
        assert "SYSTEM CORE ABS" in result.content


class _StaticProvider(ContextProvider):
    """Minimal provider stub for orchestrator tests."""

    def __init__(self, name: str, content: str, token_count: int, priority: int) -> None:
        self._result = ContextResult(
            content=content,
            token_count=token_count,
            name=name,
            priority=priority,
        )

    async def provide(self, state: dict[str, object], budget: int) -> ContextResult | None:
        return self._result


class TestContextOrchestrator:
    """Test ContextOrchestrator."""

    def test_creation_with_providers(self):
        """Test orchestrator creation with providers."""
        orchestrator = ContextOrchestrator(
            [
                SystemPersonaProvider(),
            ]
        )
        assert orchestrator is not None

    def test_default_parameters(self):
        """Test default max_tokens and output_reserve."""
        orchestrator = ContextOrchestrator([])
        assert orchestrator._max_input_tokens == 128000 - 4096

    def test_custom_parameters(self):
        """Test custom max_tokens and output_reserve."""
        orchestrator = ContextOrchestrator(
            [],
            max_tokens=64000,
            output_reserve=2048,
        )
        assert orchestrator._max_input_tokens == 64000 - 2048

    @pytest.mark.asyncio
    async def test_empty_providers(self):
        """Test with no providers returns empty string."""
        orchestrator = ContextOrchestrator([])
        result = await orchestrator.build_context({})
        assert result == ""

    @pytest.mark.asyncio
    async def test_single_provider(self):
        """Test with single provider."""
        orchestrator = ContextOrchestrator(
            [
                SystemPersonaProvider(role="developer"),
            ]
        )
        result = await orchestrator.build_context({})
        assert "developer" in result

    @pytest.mark.asyncio
    async def test_multiple_providers_parallel(self):
        """Test multiple providers are executed."""
        orchestrator = ContextOrchestrator(
            [
                SystemPersonaProvider(role="architect"),
                _StaticProvider(
                    name="secondary",
                    content="<secondary>extra</secondary>",
                    token_count=2,
                    priority=50,
                ),
            ]
        )
        result = await orchestrator.build_context({})
        assert "architect" in result
        assert "<secondary>extra</secondary>" in result

    @pytest.mark.asyncio
    async def test_priority_ordering(self):
        """Test that context results are sorted by priority after assembly."""
        orchestrator = ContextOrchestrator(
            [
                _StaticProvider("later", "<later/>", 1, 20),
                _StaticProvider("first", "<first/>", 1, 5),
            ]
        )
        context = await orchestrator.build_context({})

        assert context.index("<first/>") < context.index("<later/>")


class TestFactoryFunctions:
    """Test factory functions."""

    def test_create_planner_orchestrator(self):
        """Test planner orchestrator creation."""
        orchestrator = create_planner_orchestrator()
        assert isinstance(orchestrator, ContextOrchestrator)
        assert len(orchestrator._providers) == 1

    def test_create_executor_orchestrator(self):
        """Test executor orchestrator creation."""
        orchestrator = create_executor_orchestrator()
        assert isinstance(orchestrator, ContextOrchestrator)
        assert len(orchestrator._providers) == 1

    @pytest.mark.asyncio
    async def test_planner_has_all_providers(self):
        """Planner orchestrator should keep only persona in Rust-authoritative mode."""
        orchestrator = create_planner_orchestrator()
        provider_types = [type(p).__name__ for p in orchestrator._providers]
        assert "SystemPersonaProvider" in provider_types
        assert "AvailableToolsProvider" not in provider_types
        assert "ActiveSkillProvider" not in provider_types
        assert "EpisodicMemoryProvider" not in provider_types

    @pytest.mark.asyncio
    async def test_executor_has_core_providers(self):
        """Executor orchestrator should keep only persona in Rust-authoritative mode."""
        orchestrator = create_executor_orchestrator()
        provider_types = [type(p).__name__ for p in orchestrator._providers]
        assert "SystemPersonaProvider" in provider_types
        assert "ActiveSkillProvider" not in provider_types
        assert provider_types.count("AvailableToolsProvider") == 0


class TestContextIntegration:
    """Integration tests for the context module."""

    @pytest.mark.asyncio
    async def test_build_context_with_state(self):
        """Test building context with realistic state in persona-only mode."""
        state = {
            "active_skill": "git",
            "current_task": "Analyze repository status",
            "messages": [{"content": "Show me the git status"}],
        }

        orchestrator = create_planner_orchestrator()
        context = await orchestrator.build_context(state)

        assert isinstance(context, str)
        # Context should contain system persona
        assert len(context) > 0
