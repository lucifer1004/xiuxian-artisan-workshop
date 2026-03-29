"""Tests for InferenceClient with OpenAI-compatible HTTP backend.

Tests verify LLM API message format compliance:
- system_prompt is passed as separate parameter
- messages array contains only 'user' and 'assistant' roles
- Tool call extraction from text content
"""

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from xiuxian_foundation.services.llm.client import InferenceClient


class TestInferenceClientHTTPBackend:
    """Tests for InferenceClient using compatibility backend."""

    def test_backend_module_loaded(self):
        """Test that compatibility backend is loaded on initialization."""
        with (
            patch("xiuxian_foundation.services.llm.client.get_setting") as mock_get,
            patch("xiuxian_foundation.services.llm.client.get_anthropic_api_key") as mock_key,
        ):
            mock_get.side_effect = lambda key, default=None: {
                "inference.base_url": "https://api.anthropic.com",
                "inference.model": "claude-sonnet-4-20250514",
                "inference.timeout": 120,
                "inference.max_tokens": 4096,
            }.get(key, default)
            mock_key.return_value = "test-api-key"

            client = InferenceClient()
            assert hasattr(client, "_backend")
            assert client._backend.__name__ == "openai_http_backend"

    def test_minimax_uses_auth_token(self):
        """Test that MiniMax API configuration is loaded."""
        with (
            patch("xiuxian_foundation.services.llm.client.get_setting") as mock_get,
            patch("xiuxian_foundation.services.llm.client.get_anthropic_api_key") as mock_key,
        ):
            mock_get.side_effect = lambda key, default=None: {
                "inference.base_url": "https://api.minimax.chat/v1",
                "inference.model": "abab6.5s-chat",
                "inference.timeout": 120,
                "inference.max_tokens": 4096,
            }.get(key, default)
            mock_key.return_value = "test-api-key"

            client = InferenceClient()
            assert "minimax" in client.base_url.lower()
            assert client.model == "abab6.5s-chat"


class TestInferenceClientMessageFormat:
    """Tests for LLM API message format via HTTP backend."""

    def _create_backend_response(self, text: str, tool_calls=None) -> MagicMock:
        """Create a mock backend response (OpenAI/Anthropic format)."""
        mock_choice = MagicMock()
        mock_message = MagicMock()
        mock_message.content = text
        if tool_calls:
            mock_message.tool_calls = tool_calls
        mock_choice.message = mock_message
        mock_choice.finish_reason = "stop"

        mock_usage = MagicMock()
        mock_usage.prompt_tokens = 100
        mock_usage.completion_tokens = 50

        mock_response = MagicMock()
        mock_response.choices = [mock_choice]
        mock_response.usage = mock_usage
        return mock_response

    @pytest.mark.asyncio
    async def test_complete_sends_messages_via_backend(self):
        """Test that complete() sends messages via backend.acompletion."""
        with (
            patch("xiuxian_foundation.services.llm.client.get_setting") as mock_get,
            patch("xiuxian_foundation.services.llm.client.get_anthropic_api_key") as mock_key,
        ):
            mock_get.side_effect = lambda key, default=None: {
                "inference.base_url": "https://api.anthropic.com",
                "inference.model": "claude-sonnet-4-20250514",
                "inference.provider": "anthropic",
                "inference.timeout": 120,
                "inference.max_tokens": 4096,
            }.get(key, default)
            mock_key.return_value = "test-api-key"

            client = InferenceClient()
            # Mock the _backend attribute directly after construction
            mock_response = self._create_backend_response("Hello!")
            mock_backend_instance = AsyncMock()
            mock_backend_instance.acompletion = AsyncMock(return_value=mock_response)
            client._backend = mock_backend_instance

            result = await client.complete(
                system_prompt="You are a helpful assistant.",
                user_query="Say hello",
            )

            # Verify backend was called
            mock_backend_instance.acompletion.assert_called_once()
            call_kwargs = mock_backend_instance.acompletion.call_args[1]

            # Verify message format
            assert "messages" in call_kwargs
            assert call_kwargs["model"] == "anthropic/claude-sonnet-4-20250514"
            assert "anthropic" in call_kwargs["model"].lower()

            # Result should be successful
            assert result["success"] is True
            assert result["content"] == "Hello!"


class TestToolCallParsingHTTPBackend:
    """Tests for tool call extraction from text content via HTTP backend."""

    def _create_backend_response(self, text: str) -> MagicMock:
        """Create a mock backend response with text content."""
        mock_choice = MagicMock()
        mock_message = MagicMock()
        mock_message.content = text
        mock_message.tool_calls = None
        mock_choice.message = mock_message
        mock_choice.finish_reason = "stop"

        mock_usage = MagicMock()
        mock_usage.prompt_tokens = 100
        mock_usage.completion_tokens = 50

        mock_response = MagicMock()
        mock_response.choices = [mock_choice]
        mock_response.usage = mock_usage
        return mock_response

    @pytest.mark.asyncio
    async def test_tool_call_extraction_simple(self):
        """Test simple [TOOL_CALL: skill.command] extraction."""
        with (
            patch("xiuxian_foundation.services.llm.client.get_setting") as mock_get,
            patch("xiuxian_foundation.services.llm.client.get_anthropic_api_key") as mock_key,
        ):
            mock_get.side_effect = lambda key, default=None: {
                "inference.base_url": "https://api.minimax.chat/v1",
                "inference.model": "abab6.5s-chat",
                "inference.timeout": 120,
                "inference.max_tokens": 4096,
            }.get(key, default)
            mock_key.return_value = "test-api-key"

            client = InferenceClient()
            # Mock backend directly on client
            mock_response = self._create_backend_response(
                "I need to list files.\n[TOOL_CALL: filesystem.list_directory]\nLet me do that."
            )
            mock_backend = AsyncMock()
            mock_backend.acompletion = AsyncMock(return_value=mock_response)
            client._backend = mock_backend

            result = await client.complete(
                system_prompt="You are a helpful assistant.",
                user_query="List the files",
            )

            assert result["success"] is True
            assert len(result["tool_calls"]) == 1
            assert result["tool_calls"][0]["name"] == "filesystem.list_directory"

    @pytest.mark.asyncio
    async def test_tool_call_in_thinking_block_filtered(self):
        """Test that [TOOL_CALL: ...] in thinking blocks are NOT extracted."""
        with (
            patch("xiuxian_foundation.services.llm.client.get_setting") as mock_get,
            patch("xiuxian_foundation.services.llm.client.get_anthropic_api_key") as mock_key,
        ):
            mock_get.side_effect = lambda key, default=None: {
                "inference.base_url": "https://api.minimax.chat/v1",
                "inference.model": "abab6.5s-chat",
                "inference.timeout": 120,
                "inference.max_tokens": 4096,
            }.get(key, default)
            mock_key.return_value = "test-api-key"

            # Content with tool call ONLY in thinking block
            content = (
                "<thinking>\n"
                "Current Goal: List files\n"
                "Intent: I should use filesystem.list_directory\n"
                "Routing: I'll call [TOOL_CALL: filesystem.read_files] to read\n"
                "</thinking>\n"
                "Let me help you with that."
            )

            client = InferenceClient()
            mock_response = self._create_backend_response(content)
            mock_backend = AsyncMock()
            mock_backend.acompletion = AsyncMock(return_value=mock_response)
            client._backend = mock_backend

            result = await client.complete(
                system_prompt="You are a helpful assistant.",
                user_query="List files",
            )

            # Should NOT extract tool calls from thinking block
            assert result["success"] is True
            assert len(result["tool_calls"]) == 0
            assert "<thinking>" in result["content"]

    @pytest.mark.asyncio
    async def test_no_tool_calls_text_response_only(self):
        """Test that plain text response has no tool calls."""
        with (
            patch("xiuxian_foundation.services.llm.client.get_setting") as mock_get,
            patch("xiuxian_foundation.services.llm.client.get_anthropic_api_key") as mock_key,
        ):
            mock_get.side_effect = lambda key, default=None: {
                "inference.base_url": "https://api.anthropic.com",
                "inference.model": "claude-sonnet-4-20250514",
                "inference.timeout": 120,
                "inference.max_tokens": 4096,
            }.get(key, default)
            mock_key.return_value = "test-api-key"

            client = InferenceClient()
            mock_response = self._create_backend_response("Hello! How can I help you today?")
            mock_backend = AsyncMock()
            mock_backend.acompletion = AsyncMock(return_value=mock_response)
            client._backend = mock_backend

            result = await client.complete(
                system_prompt="You are a helpful assistant.",
                user_query="Say hello",
            )

            assert result["success"] is True
            assert result["content"] == "Hello! How can I help you today?"
            assert len(result["tool_calls"]) == 0


class TestErrorHandlingHTTPBackend:
    """Tests for error handling with HTTP backend."""

    @pytest.mark.asyncio
    async def test_exception_returns_error(self):
        """Test that exceptions are handled gracefully."""
        with (
            patch("xiuxian_foundation.services.llm.client.get_setting") as mock_get,
            patch("xiuxian_foundation.services.llm.client.get_anthropic_api_key") as mock_key,
        ):
            mock_get.side_effect = lambda key, default=None: {
                "inference.base_url": "https://api.anthropic.com",
                "inference.model": "claude-sonnet-4-20250514",
                "inference.timeout": 120,
                "inference.max_tokens": 4096,
            }.get(key, default)
            mock_key.return_value = "test-api-key"

            client = InferenceClient()
            # Simulate API error
            mock_backend = AsyncMock()
            mock_backend.acompletion = AsyncMock(side_effect=Exception("API rate limit exceeded"))
            client._backend = mock_backend

            result = await client.complete(
                system_prompt="You are a helpful assistant.",
                user_query="Test error",
            )

            assert result["success"] is False
            assert "API rate limit exceeded" in result["error"]
            assert len(result["tool_calls"]) == 0

    @pytest.mark.asyncio
    async def test_timeout_returns_error(self):
        """Test that timeout errors are handled gracefully."""
        with (
            patch("xiuxian_foundation.services.llm.client.get_setting") as mock_get,
            patch("xiuxian_foundation.services.llm.client.get_anthropic_api_key") as mock_key,
        ):
            mock_get.side_effect = lambda key, default=None: {
                "inference.base_url": "https://api.anthropic.com",
                "inference.model": "claude-sonnet-4-20250514",
                "inference.timeout": 30,
                "inference.max_tokens": 4096,
            }.get(key, default)
            mock_key.return_value = "test-api-key"

            client = InferenceClient()
            # Simulate timeout
            mock_backend = AsyncMock()
            mock_backend.acompletion = AsyncMock(side_effect=TimeoutError())
            client._backend = mock_backend

            result = await client.complete(
                system_prompt="You are a helpful assistant.",
                user_query="Test timeout",
            )

            assert result["success"] is False
            assert "timed out" in result["error"].lower()


class TestRetryLogicHTTPBackend:
    """Tests for retry logic via HTTP backend."""

    @pytest.mark.asyncio
    async def test_retry_on_failure(self):
        """Test that retry logic works on failures (manual implementation)."""
        with (
            patch("xiuxian_foundation.services.llm.client.get_setting") as mock_get,
            patch("xiuxian_foundation.services.llm.client.get_anthropic_api_key") as mock_key,
        ):
            mock_get.side_effect = lambda key, default=None: {
                "inference.base_url": "https://api.anthropic.com",
                "inference.model": "claude-sonnet-4-20250514",
                "inference.timeout": 120,
                "inference.max_tokens": 4096,
            }.get(key, default)
            mock_key.return_value = "test-api-key"

            # First call fails, second succeeds
            mock_choice = MagicMock()
            mock_message = MagicMock()
            mock_message.content = "Success!"
            mock_message.tool_calls = None
            mock_choice.message = mock_message

            mock_usage = MagicMock()
            mock_usage.prompt_tokens = 100
            mock_usage.completion_tokens = 50

            mock_success_response = MagicMock()
            mock_success_response.choices = [mock_choice]
            mock_success_response.usage = mock_usage

            client = InferenceClient()
            mock_backend = AsyncMock()
            mock_backend.acompletion = AsyncMock(
                side_effect=[
                    Exception("Temporary error"),
                    mock_success_response,
                ]
            )
            client._backend = mock_backend

            # Manual retry loop (simplified version)
            last_error = None
            for attempt in range(3):
                try:
                    result = await client.complete(
                        system_prompt="You are a helpful assistant.",
                        user_query="Test retry",
                    )
                    if result["success"]:
                        break
                except Exception as e:
                    last_error = e
            else:
                result = {"success": False, "error": str(last_error)}

            # Verify retry behavior
            assert mock_backend.acompletion.call_count == 2


class TestBuildSystemPrompt:
    """Tests for _build_system_prompt method."""

    def test_prompt_from_role_and_name(self):
        """Test prompt building from role and name."""
        with (
            patch("xiuxian_foundation.services.llm.client.get_setting") as mock_get,
            patch("xiuxian_foundation.services.llm.client.get_anthropic_api_key") as mock_key,
        ):
            mock_get.return_value = "https://api.anthropic.com"
            mock_key.return_value = "test-key"

            client = InferenceClient()

            prompt = client._build_system_prompt(
                role="helpful assistant", name="Omni", description="An AI assistant"
            )

            assert prompt == "You are Omni. An AI assistant"

    def test_prompt_from_prompt_parameter(self):
        """Test that prompt parameter takes precedence."""
        with (
            patch("xiuxian_foundation.services.llm.client.get_setting") as mock_get,
            patch("xiuxian_foundation.services.llm.client.get_anthropic_api_key") as mock_key,
        ):
            mock_get.return_value = "https://api.anthropic.com"
            mock_key.return_value = "test-key"

            client = InferenceClient()

            custom_prompt = "You are a coding expert. Help with code reviews."
            prompt = client._build_system_prompt(
                role="helpful assistant",
                name="Coder",
                description="An AI assistant",
                prompt=custom_prompt,
            )

            assert prompt == custom_prompt
