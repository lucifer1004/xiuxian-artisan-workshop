"""Tests for router translation sanitization and fallback behavior."""

from __future__ import annotations

import pytest

from omni.core.router.translate import translate_query_to_english


class _AvailableProvider:
    def __init__(self, response: str) -> None:
        self._response = response

    def is_available(self) -> bool:
        return True

    async def complete_async(self, *args, **kwargs) -> str:
        return self._response


@pytest.mark.asyncio
async def test_translate_skips_for_likely_english():
    query = "research https://example.com/repo"
    translated = await translate_query_to_english(query, enabled=True)
    assert translated == query


@pytest.mark.asyncio
async def test_translate_rejects_think_only_output_and_uses_fallback(
    monkeypatch: pytest.MonkeyPatch,
):
    query = "帮我研究一下 https://github.com/acme/repo"

    monkeypatch.setattr(
        "omni.foundation.services.llm.provider.get_llm_provider",
        lambda: _AvailableProvider("<think>"),
    )

    translated = await translate_query_to_english(query, enabled=True)
    assert translated.startswith("research ")
    assert "https://github.com/acme/repo" in translated


@pytest.mark.asyncio
async def test_translate_extracts_real_line_after_reasoning(monkeypatch: pytest.MonkeyPatch):
    query = "帮我研究一下 https://github.com/acme/repo"
    response = """<think>
Need to translate briefly.
</think>
Help me research https://github.com/acme/repo"""

    monkeypatch.setattr(
        "omni.foundation.services.llm.provider.get_llm_provider",
        lambda: _AvailableProvider(response),
    )

    translated = await translate_query_to_english(query, enabled=True)
    assert translated == "Help me research https://github.com/acme/repo"


@pytest.mark.asyncio
async def test_translate_non_english_output_uses_fallback(monkeypatch: pytest.MonkeyPatch):
    query = "帮我研究这个仓库"

    monkeypatch.setattr(
        "omni.foundation.services.llm.provider.get_llm_provider",
        lambda: _AvailableProvider("研究这个仓库"),
    )

    translated = await translate_query_to_english(query, enabled=True)
    assert "research" in translated
