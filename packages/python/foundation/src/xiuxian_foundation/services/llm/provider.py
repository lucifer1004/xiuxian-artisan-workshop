# provider.py
"""
LLM Provider API - Unified LLM Access via OpenAI-compatible HTTP backend.

Usage:
    from xiuxian_foundation.services.llm import get_llm_provider

    provider = get_llm_provider()
    result = await provider.complete("You are an expert.", "Extract entities from this text.")
    embeddings = provider.embed(["text1", "text2"])
"""

from __future__ import annotations

import asyncio
import json
import time
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any

import structlog

logger = structlog.get_logger("llm.provider")


def _minimax_model_casing(model: str) -> str:
    """Normalise MiniMax model name to dashboard-style casing before request dispatch.

    MiniMax platform docs use "MiniMax-M2.1-highspeed"; the v1 API supports
    the fast variant as "MiniMax-M2.1-lightning". We map highspeed -> lightning so
    config can use either name and the API accepts it (avoids 2013 unknown model).
    """
    if not model or not model.lower().startswith("minimax-"):
        return model
    suffix = model[len("minimax-") :]
    if suffix.lower().startswith("m2"):
        suffix = "M2" + suffix[2:]
    # v1 API uses "lightning" for the fast variant; platform docs say "highspeed"
    if "-highspeed" in suffix.lower():
        suffix = suffix.replace("-highspeed", "-lightning").replace("-Highspeed", "-lightning")
    return "MiniMax-" + suffix


@dataclass
class LLMConfig:
    """LLM Configuration."""

    provider: str = "anthropic"  # openai, anthropic, azure, google, etc.
    model: str = "sonnet"
    base_url: str | None = None
    api_key_env: str = "ANTHROPIC_API_KEY"
    timeout: int = 60
    max_tokens: int = 4096
    embedding_model: str = "text-embedding-3-small"
    embedding_dim: int = 1024


@dataclass
class LLMResponse:
    """LLM Response wrapper."""

    content: str
    success: bool
    error: str = ""
    usage: dict[str, int] = field(default_factory=dict)
    model: str = ""
    tool_calls: list[dict[str, Any]] = field(default_factory=list)


class LLMProvider(ABC):
    """Abstract base class for LLM providers."""

    @abstractmethod
    async def complete(
        self,
        system_prompt: str,
        user_query: str,
        model: str | None = None,
        max_tokens: int | None = None,
        **kwargs,
    ) -> LLMResponse:
        """Make a non-streaming LLM call."""
        pass

    @abstractmethod
    async def complete_async(
        self,
        system_prompt: str,
        user_query: str = "",
        model: str | None = None,
        max_tokens: int | None = None,
        **kwargs,
    ) -> str:
        """Make a non-streaming LLM call, returning just the content string."""
        pass

    @abstractmethod
    async def embed(self, texts: list[str]) -> list[list[float]]:
        """Generate embeddings for texts."""
        pass

    @abstractmethod
    def is_available(self) -> bool:
        """Check if LLM is properly configured."""
        pass

    @abstractmethod
    def get_config(self) -> LLMConfig:
        """Get current configuration."""
        pass


class RustLLMProvider(LLMProvider):
    """Unified LLM provider backed by OpenAI-compatible HTTP."""

    def __init__(self, config: LLMConfig | None = None):
        from xiuxian_foundation.services.llm.http_backend import OpenAIHTTPBackend

        self.config = config or self._load_config()
        self._backend = OpenAIHTTPBackend()
        self._available = self._check_availability()

    def _load_config(self) -> LLMConfig:
        from xiuxian_foundation.config.settings import get_setting

        return LLMConfig(
            provider=get_setting("inference.provider"),
            model=get_setting("inference.model"),
            base_url=get_setting("inference.base_url"),
            api_key_env=get_setting("inference.api_key_env"),
            timeout=int(get_setting("inference.timeout")),
            max_tokens=int(get_setting("inference.max_tokens")),
        )

    def _check_availability(self) -> bool:
        """Check if configured LLM API key is present."""
        import os

        api_key_env = (self.config.api_key_env or "").strip()
        if not api_key_env:
            return False
        return bool(os.getenv(api_key_env))

    async def complete(
        self,
        system_prompt: str,
        user_query: str,
        model: str | None = None,
        max_tokens: int | None = None,
        **kwargs,
    ) -> LLMResponse:
        """Make a non-streaming LLM call using HTTP backend."""
        if not self._available:
            return LLMResponse(
                content="",
                success=False,
                error="No LLM API key configured. Set ANTHROPIC_API_KEY, OPENAI_API_KEY, etc.",
            )

        try:
            actual_model = model or self.config.model
            actual_max_tokens = max_tokens or self.config.max_tokens
            actual_timeout = int(kwargs.get("timeout", self.config.timeout))

            # Prepare model string.
            # For MiniMax: pass model=actual_model and custom_llm_provider="minimax" so the
            # request body keeps exact dashboard casing (e.g. MiniMax-M2.1-highspeed).
            api_key = self._get_api_key()
            if self.config.provider == "minimax":
                actual_model = _minimax_model_casing(actual_model)
                request_model = actual_model
                request_kwargs = {
                    "model": request_model,
                    "custom_llm_provider": "minimax",
                    "max_tokens": actual_max_tokens,
                    "api_key": api_key,
                    "timeout": actual_timeout,
                }
            else:
                request_model = f"{self.config.provider}/{actual_model}"
                request_kwargs = {
                    "model": request_model,
                    "max_tokens": actual_max_tokens,
                    "api_key": api_key,
                    "timeout": actual_timeout,
                }
            tools = kwargs.get("tools")
            tool_choice = kwargs.get("tool_choice")
            response_format = kwargs.get("response_format")
            messages = kwargs.get("messages")
            temperature = kwargs.get("temperature")
            top_p = kwargs.get("top_p")
            stop = kwargs.get("stop")

            # Add base_url for MiniMax.
            if self.config.provider == "minimax":
                request_kwargs["api_base"] = "https://api.minimax.io/v1"
                request_kwargs["headers"] = {"Authorization": f"Bearer {api_key}"}
                # Ensure request body uses exact model name (dashboard casing e.g.
                # MiniMax-M2.1-highspeed).
                base_extra = request_kwargs.get("extra_body") or kwargs.get("extra_body") or {}
                request_kwargs["extra_body"] = {**base_extra, "model": actual_model}
                # Optional: disable long reasoning for faster response (inference.minimax_disable_reasoning: true)
                try:
                    from xiuxian_foundation.config.settings import get_setting

                    if bool(get_setting("inference.minimax_disable_reasoning", False)):
                        request_kwargs["extra_body"]["reasoning"] = False
                except Exception:
                    pass
            elif self.config.base_url:
                request_kwargs["api_base"] = self.config.base_url

            # System prompt: only add to kwargs when using caller-provided messages
            # (when we build messages below we include system in the list)
            if tools:
                request_kwargs["tools"] = tools
            if tool_choice is not None:
                request_kwargs["tool_choice"] = tool_choice
            if response_format is not None:
                request_kwargs["response_format"] = response_format
            if temperature is not None:
                request_kwargs["temperature"] = temperature
            if top_p is not None:
                request_kwargs["top_p"] = top_p
            if stop is not None:
                request_kwargs["stop"] = stop

            # Build messages: explicit system + user for compatibility (e.g. MiniMax)
            if messages:
                # Caller-provided messages; optional system_prompt
                if system_prompt:
                    request_kwargs["system_prompt"] = system_prompt
            elif user_query:
                if system_prompt:
                    messages = [
                        {"role": "system", "content": system_prompt},
                        {"role": "user", "content": user_query},
                    ]
                else:
                    messages = [{"role": "user", "content": user_query}]
            else:
                # Single full prompt as user content; add minimal system for APIs that expect it
                messages = [
                    {
                        "role": "system",
                        "content": "You are a helpful assistant. Return only the requested format (e.g. JSON).",
                    },
                    {"role": "user", "content": system_prompt},
                ]

            if messages:
                start = time.perf_counter()
                response = await self._backend.acompletion(
                    **request_kwargs,
                    messages=messages,
                )
                duration_sec = round(time.perf_counter() - start, 2)
                logger.debug(
                    "LLM request completed",
                    duration_sec=duration_sec,
                    model=actual_model,
                )

            # Extract content - MiniMax may return content in reasoning_content
            content = ""
            tool_calls: list[dict[str, Any]] = []
            try:
                if response.choices and len(response.choices) > 0:
                    choice = response.choices[0]
                    if hasattr(choice, "message"):
                        msg = choice.message
                        # Prefer assistant content for structured outputs; fall back to reasoning text.
                        raw_content = getattr(msg, "content", None)
                        reasoning = getattr(msg, "reasoning_content", None)

                        if raw_content and isinstance(raw_content, str) and raw_content.strip():
                            content = raw_content
                        elif raw_content and isinstance(raw_content, list):
                            # Handle content array format
                            content = ""
                            for block in raw_content:
                                if hasattr(block, "text"):
                                    content += block.text
                        elif reasoning and reasoning.strip():
                            content = reasoning

                        raw_tool_calls = getattr(msg, "tool_calls", None)
                        if raw_tool_calls:
                            for tc in raw_tool_calls:
                                fn = getattr(tc, "function", None)
                                name = getattr(fn, "name", "")
                                arguments = getattr(fn, "arguments", {})
                                if isinstance(arguments, str):
                                    try:
                                        arguments = json.loads(arguments)
                                    except Exception:
                                        arguments = {"raw": arguments}
                                tool_calls.append(
                                    {
                                        "id": getattr(tc, "id", ""),
                                        "name": name,
                                        "input": arguments if isinstance(arguments, dict) else {},
                                    }
                                )
            except Exception as e:
                logger.warning("Failed to extract content", error=str(e))

            # Extract usage
            usage = {}
            if hasattr(response, "usage") and response.usage:
                usage = {
                    "input_tokens": getattr(response.usage, "prompt_tokens", 0),
                    "output_tokens": getattr(response.usage, "completion_tokens", 0),
                }

            return LLMResponse(
                content=content,
                success=True,
                usage=usage,
                model=actual_model,
                tool_calls=tool_calls,
            )

        except Exception as e:
            logger.error("LLM complete failed", error=str(e))
            return LLMResponse(content="", success=False, error=str(e))

    async def complete_async(
        self,
        system_prompt: str,
        user_query: str = "",
        model: str | None = None,
        max_tokens: int | None = None,
        **kwargs,
    ) -> str:
        """Make a non-streaming LLM call, returning just the content string."""
        response = await self.complete(system_prompt, user_query, model, max_tokens, **kwargs)
        return response.content if response.success else ""

    async def embed(self, texts: list[str]) -> list[list[float]]:
        """Generate embeddings using unified embedding service.

        Delegates to xiuxian_foundation.services.embedding for consistent behavior.
        Falls back to zero vectors if embedding service fails.
        """
        if not texts:
            return []

        # Import from unified embedding service
        try:
            from xiuxian_foundation.services.embedding import embed_batch

            # Run sync embed_batch in thread pool (it's fast for local models)
            loop = asyncio.get_running_loop()
            vectors = await loop.run_in_executor(None, lambda: embed_batch(texts))
            return vectors

        except Exception as e:
            logger.debug("Embedding service failed, using zero vectors", error=str(e))

        # Fallback: return zero vectors
        dim = self.config.embedding_dim
        return [[0.0] * dim for _ in texts]

    def is_available(self) -> bool:
        """Check if LLM is properly configured."""
        return self._available

    def get_config(self) -> LLMConfig:
        """Get current configuration."""
        return self.config

    def _get_api_key(self) -> str | None:
        """Get API key from environment."""
        import os

        api_key_env = (self.config.api_key_env or "").strip()
        if not api_key_env:
            return None
        return os.getenv(api_key_env)


class NoOpProvider(LLMProvider):
    """No-op provider when LLM is not configured."""

    def __init__(self):
        self.config = LLMConfig()

    async def complete(
        self,
        system_prompt: str,
        user_query: str,
        model: str | None = None,
        max_tokens: int | None = None,
        **kwargs,
    ) -> LLMResponse:
        return LLMResponse(
            content="",
            success=False,
            error="LLM not configured - set ANTHROPIC_API_KEY, OPENAI_API_KEY, etc.",
        )

    async def complete_async(
        self,
        system_prompt: str,
        user_query: str = "",
        model: str | None = None,
        max_tokens: int | None = None,
        **kwargs,
    ) -> str:
        """Return empty string when LLM is not configured."""
        return ""

    async def embed(self, texts: list[str]) -> list[list[float]]:
        """Return zero vectors via unified embedding interface."""
        from xiuxian_foundation.services.embedding import embed_batch

        try:
            loop = asyncio.get_running_loop()
            return await loop.run_in_executor(None, lambda: embed_batch(texts))
        except Exception:
            dim = 2560  # Use unified embedding dimension
            return [[0.0] * dim for _ in texts]

    def is_available(self) -> bool:
        return False

    def get_config(self) -> LLMConfig:
        return LLMConfig()


# Provider registry
_PROVIDER_CACHE: LLMProvider | None = None


def get_llm_provider() -> LLMProvider:
    """Get the configured LLM provider (singleton).

    Returns the appropriate provider based on configuration.
    Falls back to NoOpProvider if LLM is not configured.

    Usage:
        from xiuxian_foundation.services.llm import get_llm_provider

        provider = get_llm_provider()
        result = await provider.complete("You are helpful.", "What is 2+2?")
        print(result.content)  # "4"
    """
    global _PROVIDER_CACHE

    if _PROVIDER_CACHE is not None:
        return _PROVIDER_CACHE

    # Try to create configured provider
    try:
        provider = RustLLMProvider()
        if provider.is_available():
            _PROVIDER_CACHE = provider
            logger.info("Using LLM provider", provider=provider.config.provider)
            return provider

        # Fall through to NoOpProvider
    except Exception as e:
        logger.warning("Failed to create LLM provider", error=str(e))

    # Use NoOpProvider
    _PROVIDER_CACHE = NoOpProvider()
    logger.info("Using NoOpProvider (LLM not configured)")
    return _PROVIDER_CACHE


def reset_provider() -> None:
    """Reset the provider cache (for testing)."""
    global _PROVIDER_CACHE
    _PROVIDER_CACHE = None


# Convenience function for quick access
async def complete(
    system_prompt: str,
    user_query: str = "",
    model: str | None = None,
    max_tokens: int | None = None,
) -> str:
    """Quick LLM completion using default provider.

    Args:
        system_prompt: System prompt (or full prompt if user_query is empty).
        user_query: Optional user query.
        model: Optional model override.
        max_tokens: Optional max tokens.

    Returns:
        The LLM response content.
    """
    provider = get_llm_provider()
    return await provider.complete_async(system_prompt, user_query, model, max_tokens)


# Note: For embeddings, use the unified interface from embedding.py:
#   from xiuxian_foundation.services.embedding import embed_text, embed_batch


__all__ = [
    "LLMConfig",
    "LLMProvider",
    "LLMResponse",
    "RustLLMProvider",
    "NoOpProvider",
    "complete",
    "get_llm_provider",
    "reset_provider",
]
