"""
Scale benchmarks for skills core: run_tool fast path, service entry.

Skills are the core; these benchmarks guard latency of the user-facing interface
and thinned implementation so we avoid regression and keep scale.

Run: just test-benchmarks
"""

from __future__ import annotations

import json
import time
from contextlib import asynccontextmanager
from types import SimpleNamespace
from unittest.mock import AsyncMock, MagicMock, patch

import pytest
from omni.test_kit.benchmark import (
    assert_async_latency_under_ms,
    assert_sync_latency_under_ms,
    benchmark_index_path,
    dump_skills_monitor_report,
)

# Thresholds (ms): generous for CI; tighten locally for performance work.
RUN_SKILL_FAST_PATH_MS = 500
RUN_SKILL_MONITORED_FAST_PATH_MS = 700
KNOWLEDGE_RECALL_MOCK_MS = 300
ROUTE_HYBRID_MONITORED_MS = 800
MCP_CALL_TOOL_MONITORED_MS = 800
REINDEX_STATUS_MS = 2000
SYNC_SYMBOLS_MOCK_MS = 3000


@pytest.mark.benchmark
class TestRunSkillScale:
    """Benchmark run_tool (fast path) latency."""

    @pytest.mark.asyncio
    async def test_run_skill_fast_path_latency(self):
        """run_tool fast path (mocked) stays under threshold; guards dispatch overhead."""
        from omni.core.skills.runner import run_tool

        async def one():
            await run_tool("demo.echo", {"message": "bench"})

        with (
            patch("omni.core.skills.runner._monitor_enabled", return_value=False),
            patch(
                "omni.core.skills.runner._run_fast_path",
                new_callable=AsyncMock,
                return_value={"status": "ok"},
            ),
        ):
            await assert_async_latency_under_ms(one, RUN_SKILL_FAST_PATH_MS, iterations=5)

    @pytest.mark.asyncio
    async def test_run_skill_dispatch_with_mock_under_threshold(self):
        """With mocked backend, run_tool dispatch overhead is minimal."""
        from omni.core.skills.runner import run_tool

        async def run_10():
            for _ in range(10):
                await run_tool("knowledge.recall", {"query": "x", "limit": 1})

        with (
            patch("omni.core.skills.runner._monitor_enabled", return_value=False),
            patch(
                "omni.core.skills.runner._run_fast_path",
                new_callable=AsyncMock,
                return_value={"status": "ok"},
            ),
        ):
            start = time.perf_counter()
            await run_10()
            elapsed_ms = (time.perf_counter() - start) * 1000
            avg_per_call = elapsed_ms / 10
            assert avg_per_call < KNOWLEDGE_RECALL_MOCK_MS, (
                f"run_tool dispatch avg {avg_per_call:.1f}ms exceeds {KNOWLEDGE_RECALL_MOCK_MS}ms"
            )

    @pytest.mark.asyncio
    async def test_run_tool_monitoring_exposes_phase_timings(self):
        """run_tool_with_monitor should expose fine-grained phase timings for optimization."""
        from omni.core.skills.runner import run_tool_with_monitor

        async def _fake_fast_path(
            skill_name: str,
            command_name: str,
            cmd_args: dict[str, object],
        ) -> dict[str, object]:
            from omni.foundation.runtime.skills_monitor import record_phase

            record_phase(
                "runner.fast.load",
                3.0,
                skill=skill_name,
                command=command_name,
                mocked=True,
            )
            record_phase(
                "runner.fast.execute",
                5.0,
                skill=skill_name,
                command=command_name,
                mocked=True,
                arg_count=len(cmd_args),
            )
            return {"status": "ok"}

        with (
            patch("omni.core.skills.runner._monitor_enabled", return_value=True),
            patch("omni.core.skills.runner._run_fast_path", side_effect=_fake_fast_path),
        ):
            result, monitor = await run_tool_with_monitor(
                "demo.echo",
                {"message": "bench"},
                output_json=False,
                auto_report=False,
            )

        assert result == {"status": "ok"}
        assert monitor is not None
        report = monitor.build_report()
        phases = [phase["phase"] for phase in report.phases]
        assert "runner.fast.load" in phases
        assert "runner.fast.execute" in phases
        assert report.elapsed_sec >= 0.0
        artifact = dump_skills_monitor_report(
            report,
            test_name="test_run_tool_monitoring_exposes_phase_timings",
            suite="skills",
            metadata={"tool": "demo.echo", "mode": "monitored_fast_path"},
        )
        assert artifact.exists()
        index_path = benchmark_index_path()
        assert index_path.exists()
        index_payload = json.loads(index_path.read_text(encoding="utf-8"))
        assert "skills" in index_payload["suites"]

    @pytest.mark.asyncio
    async def test_run_tool_monitored_path_overhead_under_threshold(self):
        """Monitoring-enabled unified path should remain within benchmark threshold."""
        from omni.core.skills.runner import run_tool_with_monitor

        async def one():
            result, monitor = await run_tool_with_monitor(
                "demo.echo",
                {"message": "bench"},
                output_json=False,
                auto_report=False,
            )
            assert result == {"status": "ok"}
            assert monitor is not None
            _ = monitor.build_report()
            return result

        with (
            patch("omni.core.skills.runner._monitor_enabled", return_value=True),
            patch(
                "omni.core.skills.runner._run_fast_path",
                new_callable=AsyncMock,
                return_value={"status": "ok"},
            ),
        ):
            await assert_async_latency_under_ms(
                one,
                RUN_SKILL_MONITORED_FAST_PATH_MS,
                iterations=5,
            )


@pytest.mark.benchmark
class TestServiceEntryScale:
    """Benchmark thinned service entry points (reindex_status, sync paths)."""

    def test_reindex_status_latency(self):
        """reindex_status returns within threshold (mocked stores)."""
        from omni.agent.services.reindex import reindex_status

        mock_store = MagicMock()
        mock_store.list_all_tools = MagicMock(return_value=[])

        def run():
            with (
                patch(
                    "omni.agent.services.reindex.get_database_paths",
                    return_value={"skills": "/s", "knowledge": "/k"},
                ),
                patch("omni.foundation.bridge.RustVectorStore", return_value=mock_store),
                patch(
                    "omni.core.knowledge.librarian.Librarian",
                    return_value=MagicMock(is_ready=False),
                ),
            ):
                result = reindex_status()
            assert "skills.lance" in result or "knowledge.lance" in result
            return result

        assert_sync_latency_under_ms(run, REINDEX_STATUS_MS, iterations=1)

    @pytest.mark.asyncio
    async def test_sync_symbols_mocked_latency(self):
        """sync_symbols (mocked indexer) completes under threshold."""
        from omni.agent.services.sync import sync_symbols

        async def run():
            with (
                patch("omni.agent.services.sync.sync_log"),
                patch("omni.foundation.runtime.gitops.get_project_root", return_value="/tmp"),
                patch(
                    "omni.core.knowledge.symbol_indexer.SymbolIndexer",
                    return_value=MagicMock(
                        build=MagicMock(return_value={"unique_symbols": 0, "indexed_files": 0})
                    ),
                ),
            ):
                result = await sync_symbols(clear=False, verbose=False)
            assert result.get("status") in ("success", "error")
            return result

        await assert_async_latency_under_ms(run, SYNC_SYMBOLS_MOCK_MS, iterations=1)


@pytest.mark.benchmark
class TestRouteAndMcpScale:
    """Benchmark monitored route and MCP tool execution paths."""

    @pytest.mark.asyncio
    async def test_route_hybrid_monitoring_emits_artifact(self):
        """Route hybrid path should expose monitor timings and persist benchmark artifact."""
        from omni.core.router.main import OmniRouter
        from omni.foundation.runtime.skills_monitor import record_phase, skills_monitor_scope

        class _FakeHybridSearch:
            def __init__(self, _storage_path: str | None = None):
                pass

            async def search(
                self,
                query: str,
                limit: int = 5,
                min_score: float = 0.0,
            ) -> list[dict[str, object]]:
                record_phase(
                    "router.hybrid.search",
                    4.0,
                    query_len=len(query),
                    limit=limit,
                    min_score=min_score,
                )
                return [
                    {
                        "skill_name": "advanced_tools",
                        "command": "advanced_tools.smart_find",
                        "score": 0.81,
                        "final_score": 0.89,
                        "confidence": "high",
                        "vector_score": 0.74,
                        "keyword_score": 0.93,
                        "category": "workflow",
                    }
                ]

        with patch("omni.core.router.main.HybridSearch", _FakeHybridSearch):
            router = OmniRouter(storage_path=":memory:")
            started = time.perf_counter()
            async with skills_monitor_scope("router.route_hybrid", auto_report=False) as monitor:
                results = await router.route_hybrid(
                    "find python files",
                    limit=3,
                    threshold=0.2,
                    use_cache=False,
                )
            elapsed_ms = (time.perf_counter() - started) * 1000

        assert elapsed_ms < ROUTE_HYBRID_MONITORED_MS
        assert len(results) == 1
        assert results[0].skill_name == "advanced_tools"

        report = monitor.build_report()
        phases = [phase["phase"] for phase in report.phases]
        assert "router.hybrid.search" in phases

        artifact = dump_skills_monitor_report(
            report,
            test_name="test_route_hybrid_monitoring_emits_artifact",
            suite="route",
            metadata={"path": "route_hybrid", "query": "find python files"},
        )
        assert artifact.exists()
        index_path = benchmark_index_path()
        assert index_path.exists()
        index_payload = json.loads(index_path.read_text(encoding="utf-8"))
        assert "route" in index_payload["suites"]

    @pytest.mark.asyncio
    async def test_mcp_call_tool_monitoring_emits_artifact(self):
        """MCP call_tool path should expose monitor timings and persist benchmark artifact."""
        from omni.agent.server import AgentMCPHandler
        from omni.foundation.runtime.skills_monitor import skills_monitor_scope

        class _DemoSkill:
            async def execute(self, command_name: str, **kwargs: object) -> dict[str, object]:
                return {
                    "status": "ok",
                    "command": command_name,
                    "args": kwargs,
                }

        @asynccontextmanager
        async def _noop_memory_scope(_name: str):
            yield

        handler = AgentMCPHandler()
        handler._kernel = SimpleNamespace(
            is_ready=True,
            skill_context=SimpleNamespace(
                get_skill=lambda skill_name: _DemoSkill() if skill_name == "demo" else None,
            ),
        )

        request = {
            "id": 7,
            "params": {"name": "demo.echo", "arguments": {"message": "bench"}},
        }

        with (
            patch("omni.agent.mcp_server.memory_monitor.amemory_monitor_scope", _noop_memory_scope),
            patch("omni.foundation.api.tool_context.run_with_execution_timeout", lambda coro: coro),
        ):
            started = time.perf_counter()
            async with skills_monitor_scope("mcp.call_tool", auto_report=False) as monitor:
                response = await handler._handle_call_tool(request)
            elapsed_ms = (time.perf_counter() - started) * 1000

        assert elapsed_ms < MCP_CALL_TOOL_MONITORED_MS
        text_payload = response["result"]["content"][0]["text"]
        payload = json.loads(text_payload)
        assert payload["status"] == "ok"
        assert payload["command"] == "echo"

        report = monitor.build_report()
        phases = [phase["phase"] for phase in report.phases]
        assert "runner.kernel.direct.execute" in phases

        artifact = dump_skills_monitor_report(
            report,
            test_name="test_mcp_call_tool_monitoring_emits_artifact",
            suite="mcp",
            metadata={"path": "mcp.call_tool", "tool": "demo.echo"},
        )
        assert artifact.exists()
        index_path = benchmark_index_path()
        assert index_path.exists()
        index_payload = json.loads(index_path.read_text(encoding="utf-8"))
        assert "mcp" in index_payload["suites"]
