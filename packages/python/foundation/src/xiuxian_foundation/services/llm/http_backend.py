"""OpenAI-compatible HTTP backend for Python-side LLM calls."""

from __future__ import annotations

import json
from types import SimpleNamespace
from typing import Any

import aiohttp

from xiuxian_foundation.config.settings import get_setting


def _to_namespace(value: Any) -> Any:
    if isinstance(value, dict):
        return SimpleNamespace(**{k: _to_namespace(v) for k, v in value.items()})
    if isinstance(value, list):
        return [_to_namespace(v) for v in value]
    return value


def _resolve_chat_completions_url(api_base: str | None) -> str:
    base = str(api_base or get_setting("inference.base_url") or "").strip().rstrip("/")
    if not base:
        raise RuntimeError("inference.base_url is required for Python HTTP LLM backend")
    if base.endswith("/chat/completions"):
        return base
    if base.endswith("/v1"):
        return f"{base}/chat/completions"
    return f"{base}/v1/chat/completions"


def _build_payload(kwargs: dict[str, Any]) -> dict[str, Any]:
    payload: dict[str, Any] = {
        "model": kwargs.get("model"),
        "messages": kwargs.get("messages") or [],
        "max_tokens": kwargs.get("max_tokens"),
        "stream": bool(kwargs.get("stream", False)),
    }
    for key in ("tools", "tool_choice", "response_format", "temperature", "top_p", "stop"):
        if key in kwargs and kwargs[key] is not None:
            payload[key] = kwargs[key]
    # Preserve separate `system` field handling used by existing callers.
    system = kwargs.get("system")
    if system and not any(m.get("role") == "system" for m in payload["messages"]):
        payload["messages"] = [{"role": "system", "content": system}, *payload["messages"]]
    extra_body = kwargs.get("extra_body")
    if isinstance(extra_body, dict):
        payload.update(extra_body)
    return payload


def _build_headers(kwargs: dict[str, Any]) -> dict[str, str]:
    headers: dict[str, str] = {"Content-Type": "application/json"}
    given = kwargs.get("headers")
    if isinstance(given, dict):
        headers.update({str(k): str(v) for k, v in given.items()})
    api_key = kwargs.get("api_key")
    if api_key and "authorization" not in {k.lower() for k in headers}:
        headers["Authorization"] = f"Bearer {api_key}"
    return headers


class OpenAIHTTPBackend:
    """Small async adapter used by provider/client."""

    __name__ = "openai_http_backend"

    async def acompletion(self, **kwargs: Any) -> Any:
        timeout = float(kwargs.get("timeout") or get_setting("inference.timeout") or 120)
        payload = _build_payload(kwargs)
        headers = _build_headers(kwargs)
        url = _resolve_chat_completions_url(kwargs.get("api_base"))
        async with aiohttp.ClientSession(timeout=aiohttp.ClientTimeout(total=timeout)) as session:
            async with session.post(url, json=payload, headers=headers) as resp:
                text = await resp.text()
                if resp.status >= 400:
                    raise RuntimeError(
                        f"LLM HTTP request failed (status={resp.status}, url={url}, body={text[:300]})"
                    )
                data = json.loads(text or "{}")
                return _to_namespace(data)

    async def acompletion_stream(self, **kwargs: Any):
        timeout = float(kwargs.get("timeout") or get_setting("inference.timeout") or 120)
        payload = _build_payload({**kwargs, "stream": True})
        headers = _build_headers(kwargs)
        url = _resolve_chat_completions_url(kwargs.get("api_base"))
        async with aiohttp.ClientSession(timeout=aiohttp.ClientTimeout(total=timeout)) as session:
            async with session.post(url, json=payload, headers=headers) as resp:
                if resp.status >= 400:
                    text = await resp.text()
                    raise RuntimeError(
                        f"LLM stream request failed (status={resp.status}, url={url}, body={text[:300]})"
                    )
                async for chunk in resp.content:
                    line = chunk.decode("utf-8", errors="ignore").strip()
                    if not line or not line.startswith("data:"):
                        continue
                    raw = line[len("data:") :].strip()
                    if raw == "[DONE]":
                        break
                    try:
                        data = json.loads(raw)
                    except Exception:
                        continue
                    yield _to_namespace(data)
