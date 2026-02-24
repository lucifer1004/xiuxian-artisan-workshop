from __future__ import annotations

import importlib
import json
from pathlib import Path

import pytest


def _skill_root() -> Path:
    return Path(__file__).resolve().parents[1]


def _unwrap_skill_output(payload: object) -> dict[str, object]:
    if isinstance(payload, dict):
        content = payload.get("content")
        if isinstance(content, list) and content:
            first = content[0]
            if isinstance(first, dict):
                text = first.get("text")
                if isinstance(text, str):
                    return json.loads(text)
        return payload
    if isinstance(payload, str):
        return json.loads(payload)
    raise TypeError(f"Unexpected payload type: {type(payload)!r}")


@pytest.mark.asyncio
async def test_polish_text_accepts_wrapped_internal_results(monkeypatch) -> None:
    monkeypatch.syspath_prepend(str(_skill_root()))
    writer_text = importlib.import_module("scripts.text")

    out = await writer_text.polish_text("# Title\n\nThis is very basic.")
    payload = _unwrap_skill_output(out)

    assert payload.get("status") in {"clean", "needs_polish"}
    assert isinstance(payload.get("violations"), list)
