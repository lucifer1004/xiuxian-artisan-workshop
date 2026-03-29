"""Unit tests for client-only EmbeddingService."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

import pytest


@pytest.fixture(autouse=True)
def _reset_embedding_singletons():
    from xiuxian_foundation.services import embedding as embedding_module

    embedding_module._service = None
    embedding_module.EmbeddingService._instance = None
    embedding_module.set_embedding_override(None)
    yield
    embedding_module._service = None
    embedding_module.EmbeddingService._instance = None
    embedding_module.set_embedding_override(None)


class TestEmbeddingServiceInitialization:
    """Initialization behavior for client-only embedding mode."""

    def setup_method(self):
        from xiuxian_foundation.services.embedding import EmbeddingService

        EmbeddingService._instance = None

    def teardown_method(self):
        from xiuxian_foundation.services.embedding import EmbeddingService

        EmbeddingService._instance = None

    def test_initialization_with_explicit_client_provider(self):
        from xiuxian_foundation.services.embedding import EmbeddingService

        with (
            patch.object(
                EmbeddingService,
                "_resolve_client_url_candidates",
                return_value=["http://127.0.0.1:3002"],
            ),
            patch.object(EmbeddingService, "_check_http_server_healthy", return_value=True),
            patch.object(EmbeddingService, "_verify_embedding_service_works", return_value=True),
            patch("xiuxian_foundation.services.embedding.get_setting") as mock_setting,
        ):
            mock_setting.side_effect = lambda key, default=None: {
                "embedding.provider": "client",
                "embedding.client_url": "http://127.0.0.1:3002",
                "embedding.dimension": 1024,
                "embedding.http_port": 3002,
            }.get(key, default)

            service = EmbeddingService()
            service.initialize()

            assert service._client_mode is True
            assert service._backend == "http"
            assert service._client_url == "http://127.0.0.1:3002"

    def test_initialization_with_legacy_fallback_provider_forces_client(self):
        from xiuxian_foundation.services.embedding import EmbeddingService

        with (
            patch.object(EmbeddingService, "_check_http_server_healthy", return_value=True),
            patch.object(EmbeddingService, "_verify_embedding_service_works", return_value=True),
            patch("xiuxian_foundation.services.embedding.get_setting") as mock_setting,
        ):
            mock_setting.side_effect = lambda key, default=None: {
                "embedding.provider": "fallback",
                "embedding.client_url": "http://127.0.0.1:3002",
                "embedding.dimension": 1024,
                "embedding.http_port": 3002,
            }.get(key, default)

            service = EmbeddingService()
            service.initialize()

            assert service._backend == "http"
            assert service._client_mode is True

    def test_initialization_with_unknown_provider_forces_client(self):
        from xiuxian_foundation.services.embedding import EmbeddingService

        with (
            patch.object(EmbeddingService, "_check_http_server_healthy", return_value=True),
            patch.object(EmbeddingService, "_verify_embedding_service_works", return_value=True),
            patch("xiuxian_foundation.services.embedding.get_setting") as mock_setting,
        ):
            mock_setting.side_effect = lambda key, default=None: {
                "embedding.provider": "legacy-provider",
                "embedding.client_url": "http://127.0.0.1:3002",
                "embedding.dimension": 1024,
                "embedding.http_port": 3002,
            }.get(key, default)

            service = EmbeddingService()
            service.initialize()

            assert service._backend == "http"
            assert service._client_mode is True

    def test_initialization_client_unreachable_sets_unavailable(self):
        from xiuxian_foundation.services.embedding import EmbeddingService

        with (
            patch.object(EmbeddingService, "_check_http_server_healthy", return_value=False),
            patch.object(EmbeddingService, "_verify_embedding_service_works", return_value=False),
            patch("xiuxian_foundation.services.embedding.get_setting") as mock_setting,
        ):
            mock_setting.side_effect = lambda key, default=None: {
                "embedding.provider": "client",
                "embedding.client_url": "http://127.0.0.1:3002",
                "embedding.dimension": 1024,
                "embedding.http_port": 3002,
            }.get(key, default)

            service = EmbeddingService()
            service.initialize()

            assert service._backend == "unavailable"
            assert service._client_mode is False
            assert service._dimension == 1024

    def test_initialization_falls_back_to_secondary_client_url_candidate(self):
        from xiuxian_foundation.services.embedding import EmbeddingService

        with (
            patch.object(
                EmbeddingService,
                "_resolve_client_url_candidates",
                return_value=["http://127.0.0.1:3002", "http://127.0.0.1:18092"],
            ),
            patch.object(
                EmbeddingService,
                "_check_http_server_healthy",
                side_effect=[False, True],
            ),
            patch.object(EmbeddingService, "_verify_embedding_service_works", return_value=True),
            patch("xiuxian_foundation.services.embedding.get_setting") as mock_setting,
        ):
            mock_setting.side_effect = lambda key, default=None: {
                "embedding.provider": "client",
                "embedding.dimension": 1024,
            }.get(key, default)

            service = EmbeddingService()
            service.initialize()

            assert service._backend == "http"
            assert service._client_mode is True
            assert service._client_url == "http://127.0.0.1:18092"


class TestEmbeddingServiceSingleton:
    def setup_method(self):
        from xiuxian_foundation.services.embedding import EmbeddingService

        EmbeddingService._instance = None

    def test_singleton_returns_same_instance(self):
        from xiuxian_foundation.services.embedding import get_embedding_service

        assert get_embedding_service() is get_embedding_service()


class TestEmbeddingServiceEmbed:
    def setup_method(self):
        from xiuxian_foundation.services.embedding import EmbeddingService

        EmbeddingService._instance = None
        service = EmbeddingService()
        service._initialized = True
        service._backend = "http"
        service._client_mode = True
        service._client_url = "http://127.0.0.1:3002"
        service._dimension = 3
        service._embed_cache_key = None
        service._embed_cache_value = None

    def teardown_method(self):
        from xiuxian_foundation.services.embedding import EmbeddingService

        EmbeddingService._instance = None

    def test_embed_single_text(self):
        from xiuxian_foundation.services.embedding import EmbeddingService

        service = EmbeddingService()
        with patch.object(service, "_embed_http", return_value=[[0.1, 0.2, 0.3]]) as mock_http:
            result = service.embed("test text")
            assert result == [[0.1, 0.2, 0.3]]
            mock_http.assert_called_once_with(["test text"])

    def test_embed_batch_texts(self):
        from xiuxian_foundation.services.embedding import EmbeddingService

        service = EmbeddingService()
        with patch.object(
            service,
            "_embed_http",
            return_value=[[0.1, 0.2, 0.3], [0.4, 0.5, 0.6]],
        ) as mock_http:
            result = service.embed_batch(["text1", "text2"])
            assert result == [[0.1, 0.2, 0.3], [0.4, 0.5, 0.6]]
            mock_http.assert_called_once_with(["text1", "text2"])

    def test_embed_raises_when_backend_unavailable(self):
        from xiuxian_foundation.services.embedding import EmbeddingService, EmbeddingUnavailableError

        service = EmbeddingService()
        service._backend = "unavailable"
        service._client_mode = False
        service._client_retried = True
        with (
            patch("xiuxian_foundation.services.embedding.get_setting", return_value=None),
            pytest.raises(EmbeddingUnavailableError),
        ):
            service.embed("hello")


class TestEmbeddingServiceHttpRaisesOnFailure:
    def setup_method(self):
        from xiuxian_foundation.services.embedding import EmbeddingService

        EmbeddingService._instance = None
        service = EmbeddingService()
        service._initialized = True
        service._backend = "http"
        service._client_mode = True
        service._client_url = "http://127.0.0.1:3002"
        service._dimension = 256
        service._embed_cache_key = None
        service._embed_cache_value = None

    def teardown_method(self):
        from xiuxian_foundation.services.embedding import EmbeddingService

        EmbeddingService._instance = None

    def test_embed_http_raises_on_http_error(self):
        from xiuxian_foundation.services.embedding import EmbeddingService, EmbeddingUnavailableError

        service = EmbeddingService()
        with patch("xiuxian_foundation.embedding_client.get_embedding_client") as mock_get_client:
            mock_client = MagicMock()
            mock_client.sync_embed_batch.side_effect = RuntimeError("connection refused")
            mock_get_client.return_value = mock_client
            with pytest.raises(EmbeddingUnavailableError) as exc_info:
                service.embed("hello")
        assert "connection refused" in str(exc_info.value)

    def test_embed_batch_raises_on_http_error(self):
        from xiuxian_foundation.services.embedding import EmbeddingService, EmbeddingUnavailableError

        service = EmbeddingService()
        with patch("xiuxian_foundation.embedding_client.get_embedding_client") as mock_get_client:
            mock_client = MagicMock()
            mock_client.sync_embed_batch.side_effect = RuntimeError("HTTP 500")
            mock_get_client.return_value = mock_client
            with pytest.raises(EmbeddingUnavailableError) as exc_info:
                service.embed_batch(["text"])
        assert "HTTP 500" in str(exc_info.value)


class TestEmbeddingOverride:
    def test_embed_batch_uses_override_when_set(self):
        from xiuxian_foundation.services.embedding import (
            get_embedding_override,
            get_embedding_service,
            set_embedding_override,
        )

        class MockOverride:
            def embed(self, text: str):
                return [[0.1] * 8]

            def embed_batch(self, texts: list[str]):
                return [[0.2] * 8 for _ in texts]

        try:
            set_embedding_override(MockOverride())
            svc = get_embedding_service()
            out = svc.embed_batch(["a", "b"])
            assert out == [[0.2] * 8, [0.2] * 8]
        finally:
            set_embedding_override(None)
        assert get_embedding_override() is None
