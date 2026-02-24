from __future__ import annotations

import importlib
import json
from pathlib import Path
from unittest.mock import AsyncMock

import pytest

from omni.core.router.router import RouteResult


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
async def test_discover_uses_router_result_metadata_without_kernel(monkeypatch) -> None:
    monkeypatch.syspath_prepend(str(_skill_root() / "scripts"))
    discovery_module = importlib.import_module("discovery")

    routed = RouteResult(
        skill_name="demo",
        command_name="echo",
        score=0.81,
        confidence="high",
        final_score=0.84,
        ranking_reason="vector=0.81 | confidence=high | raw=0.810 | final=0.840",
        input_schema_digest="sha256:test-digest",
        description="Echo back the input message.",
        file_path="assets/skills/demo/scripts/echo.py",
        input_schema={
            "type": "object",
            "properties": {"message": {"type": "string"}},
            "required": ["message"],
        },
    )

    fake_router = type(
        "FakeRouter",
        (),
        {"route_hybrid": AsyncMock(return_value=[routed])},
    )()

    monkeypatch.setattr(
        "omni.core.router.main.RouterRegistry.get", lambda *_args, **_kwargs: fake_router
    )
    monkeypatch.setattr(
        "omni.core.kernel.get_kernel",
        lambda *_args, **_kwargs: (_ for _ in ()).throw(
            AssertionError("discover should not load kernel")
        ),
    )

    payload = _unwrap_skill_output(await discovery_module.discover("echo", limit=1))
    assert payload["status"] == "success"
    caps = payload["discovered_capabilities"]
    assert len(caps) == 1
    first = caps[0]
    assert first["tool"] == "demo.echo"
    assert first["description"] == "Echo back the input message."
    assert first["source_code_path"] == "assets/skills/demo/scripts/echo.py"
    assert first["input_schema_digest"] == "sha256:test-digest"
    assert '"message": "<message: string>"' in first["usage"]
    fake_router.route_hybrid.assert_awaited_once_with(
        query="echo",
        limit=1,
        threshold=0.1,
        keyword_only=True,
    )


@pytest.mark.asyncio
async def test_discover_infers_schema_from_description_when_missing(monkeypatch) -> None:
    monkeypatch.syspath_prepend(str(_skill_root() / "scripts"))
    discovery_module = importlib.import_module("discovery")

    routed = RouteResult(
        skill_name="demo",
        command_name="echo",
        score=0.7,
        confidence="medium",
        final_score=0.75,
        ranking_reason="fallback",
        input_schema_digest="sha256:empty",
        description=(
            "COMMAND: demo.echo\n"
            "DESCRIPTION: Echo text.\n\n"
            "Args:\n"
            "    - message: str - Message to echo (required)\n"
            "    - repeat: int = 1 - Repeat count\n\n"
            "Returns:\n"
            "    Echoed text.\n"
        ),
        file_path="",
        input_schema={},
    )

    fake_router = type(
        "FakeRouter",
        (),
        {"route_hybrid": AsyncMock(return_value=[routed])},
    )()

    monkeypatch.setattr(
        "omni.core.router.main.RouterRegistry.get", lambda *_args, **_kwargs: fake_router
    )

    payload = _unwrap_skill_output(await discovery_module.discover("echo", limit=1))
    first = payload["discovered_capabilities"][0]
    assert '"message": "<message: string>"' in first["usage"]
    assert '"repeat": "<repeat?>"' in first["usage"]
