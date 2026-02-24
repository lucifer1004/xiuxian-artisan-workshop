"""Unit tests for MCP embedding client (CLI warm-path)."""

from __future__ import annotations

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from omni.agent.cli import mcp_embed as mcp_embed_module


class TestGetCandidatePorts:
    """Tests for candidate port order (strictly from config)."""

    def test_default_order_when_no_config(self):
        with patch("omni.foundation.config.settings.get_setting", return_value=None):
            from omni.agent.cli.mcp_embed import _get_candidate_ports

            ports = _get_candidate_ports()
        assert ports == []

    def test_preferred_port_first_when_configured(self):
        def _setting(key: str):
            if key == "mcp.preferred_embed_port":
                return 3002
            if key == "embedding.client_url":
                return "http://127.0.0.1:3302"
            return None

        with patch("omni.foundation.config.settings.get_setting", side_effect=_setting):
            from omni.agent.cli.mcp_embed import _get_candidate_ports

            ports = _get_candidate_ports()
        assert ports == [3002, 3302]

    def test_invalid_preferred_uses_client_url_port(self):
        def _setting(key: str):
            if key == "mcp.preferred_embed_port":
                return 0
            if key == "embedding.client_url":
                return "http://127.0.0.1:3302"
            return None

        with patch("omni.foundation.config.settings.get_setting", side_effect=_setting):
            from omni.agent.cli.mcp_embed import _get_candidate_ports

            ports = _get_candidate_ports()
        assert ports == [3302]


class TestDetectMcpPort:
    """Tests for detect_mcp_port."""

    @pytest.mark.asyncio
    async def test_returns_embedding_http_port_when_up(self):
        with (
            patch.object(
                mcp_embed_module, "detect_embedding_http_port", new=AsyncMock(return_value=18501)
            ),
            patch.object(mcp_embed_module, "probe_mcp_embed_port", new=AsyncMock()),
        ):
            port = await mcp_embed_module.detect_mcp_port()
        assert port == 18501

    @pytest.mark.asyncio
    async def test_tries_candidates_when_embedding_http_down(self):
        with (
            patch.object(
                mcp_embed_module, "detect_embedding_http_port", new=AsyncMock(return_value=0)
            ),
            patch.object(
                mcp_embed_module,
                "probe_mcp_embed_port",
                new=AsyncMock(side_effect=[False, True, False]),
            ),
        ):
            port = await mcp_embed_module.detect_mcp_port([3002, 3001, 3000])
        assert port == 3001

    @pytest.mark.asyncio
    async def test_returns_zero_when_none_respond(self):
        with (
            patch.object(
                mcp_embed_module, "detect_embedding_http_port", new=AsyncMock(return_value=0)
            ),
            patch.object(
                mcp_embed_module, "probe_mcp_embed_port", new=AsyncMock(return_value=False)
            ),
        ):
            port = await mcp_embed_module.detect_mcp_port([3002, 3001])
        assert port == 0


class TestMcpPathSelection:
    """Tests for MCP path selection by port family."""

    def test_paths_do_not_include_legacy_message_path(self):
        paths = mcp_embed_module._mcp_paths_for_port(3302)
        assert paths == ("/messages/", "/mcp", "/")

    def test_even_legacy_port_uses_modern_paths(self):
        paths = mcp_embed_module._mcp_paths_for_port(3001)
        assert paths == ("/messages/", "/mcp", "/")


class TestMakeMcpEmbedFunc:
    """Tests for make_mcp_embed_func fallback."""

    @pytest.mark.asyncio
    async def test_fallback_to_local_when_mcp_returns_none(self):
        with (
            patch.object(mcp_embed_module, "embed_via_mcp", new=AsyncMock(return_value=None)),
            patch.object(mcp_embed_module, "embed_via_mcp_http", new=AsyncMock(return_value=None)),
            patch.object(mcp_embed_module, "embed_via_http", new=AsyncMock(return_value=None)),
        ):
            from omni.agent.cli.mcp_embed import make_mcp_embed_func

            embed_func = make_mcp_embed_func(3002)
            mock_svc = MagicMock()
            mock_svc.embed_batch.return_value = [[0.1] * 8]
            with patch(
                "omni.foundation.services.embedding.get_embedding_service", return_value=mock_svc
            ):
                result = await embed_func(["hello"])
        assert result == [[0.1] * 8]
        mock_svc.embed_batch.assert_called_once_with(["hello"])

    @pytest.mark.asyncio
    async def test_uses_mcp_when_embed_via_mcp_returns_vectors(self):
        with (
            patch.object(mcp_embed_module, "embed_via_mcp_http", new=AsyncMock(return_value=None)),
            patch.object(
                mcp_embed_module,
                "embed_via_mcp",
                new=AsyncMock(return_value=[[0.2] * 8]),
            ),
        ):
            from omni.agent.cli.mcp_embed import make_mcp_embed_func

            embed_func = make_mcp_embed_func(3002)
            result = await embed_func(["hi"])
        assert result == [[0.2] * 8]

    @pytest.mark.asyncio
    async def test_prefers_mcp_http_endpoint_before_tool_call(self):
        with (
            patch.object(
                mcp_embed_module,
                "embed_via_mcp_http",
                new=AsyncMock(return_value=[[0.4] * 8]),
            ) as mock_http_path,
            patch.object(
                mcp_embed_module,
                "embed_via_mcp",
                new=AsyncMock(return_value=[[0.2] * 8]),
            ) as mock_tool_call,
        ):
            from omni.agent.cli.mcp_embed import make_mcp_embed_func

            embed_func = make_mcp_embed_func(3002)
            result = await embed_func(["hi"])
        assert result == [[0.4] * 8]
        mock_http_path.assert_awaited_once()
        mock_tool_call.assert_not_called()

    @pytest.mark.asyncio
    async def test_modern_port_tool_fallback_does_not_hit_legacy_message_path(self):
        called_paths: list[str] = []

        async def _fake_embed_via_mcp(*_args, **kwargs):
            called_paths.append(kwargs["path"])
            return None

        with (
            patch.object(mcp_embed_module, "embed_via_mcp_http", new=AsyncMock(return_value=None)),
            patch.object(mcp_embed_module, "embed_via_mcp", side_effect=_fake_embed_via_mcp),
        ):
            from omni.agent.cli.mcp_embed import make_mcp_embed_func

            embed_func = make_mcp_embed_func(3302)
            mock_svc = MagicMock()
            mock_svc.embed_batch.return_value = [[0.3] * 8]
            with patch(
                "omni.foundation.services.embedding.get_embedding_service", return_value=mock_svc
            ):
                result = await embed_func(["hello"])
        assert result == [[0.3] * 8]
        assert "/message" not in called_paths


class TestProbeMcpEmbedPort:
    @pytest.mark.asyncio
    async def test_probe_prefers_healthcheck_before_embed_probe(self):
        with (
            patch.object(mcp_embed_module, "_mcp_health_ok", new=AsyncMock(return_value=True)),
            patch.object(
                mcp_embed_module,
                "embed_via_mcp_http",
                new=AsyncMock(return_value=[[0.1] * 8]),
            ) as mock_http_path,
            patch.object(
                mcp_embed_module,
                "embed_via_mcp",
                new=AsyncMock(return_value=None),
            ) as mock_tool_call,
        ):
            ok = await mcp_embed_module.probe_mcp_embed_port(3002)
        assert ok is True
        mock_http_path.assert_not_called()
        mock_tool_call.assert_not_called()

    @pytest.mark.asyncio
    async def test_probe_falls_back_to_embed_probe_when_healthcheck_fails(self):
        with (
            patch.object(mcp_embed_module, "_mcp_health_ok", new=AsyncMock(return_value=False)),
            patch.object(
                mcp_embed_module,
                "embed_via_mcp_http",
                new=AsyncMock(return_value=[[0.1] * 8]),
            ) as mock_http_path,
            patch.object(
                mcp_embed_module,
                "embed_via_mcp",
                new=AsyncMock(return_value=None),
            ) as mock_tool_call,
        ):
            ok = await mcp_embed_module.probe_mcp_embed_port(3002)
        assert ok is True
        mock_http_path.assert_awaited_once()
        mock_tool_call.assert_not_called()


class TestEmbedViaMcp:
    @pytest.mark.asyncio
    async def test_embed_via_mcp_prefers_direct_embed_for_modern_messages_path(self):
        """`/messages/` on modern ports should try direct /embed path before tools/call."""

        class _UnusedClient:
            async def post(self, *_args, **_kwargs):  # pragma: no cover - should not run
                raise AssertionError("tools/call path should not run when direct embed succeeds")

        with (
            patch.object(
                mcp_embed_module,
                "embed_via_mcp_http",
                new=AsyncMock(return_value=[[0.7] * 8]),
            ) as mock_direct,
            patch.object(mcp_embed_module, "_get_shared_http_client", return_value=_UnusedClient()),
        ):
            vectors = await mcp_embed_module.embed_via_mcp(
                ["hello"],
                port=3002,
                path="/messages/",
            )

        assert vectors == [[0.7] * 8]
        mock_direct.assert_awaited_once()
