"""Tests for retained thin LinkGraph proximity boost helpers."""

from __future__ import annotations

import asyncio
import pytest

from xiuxian_rag.link_graph import LinkGraphDirection, LinkGraphMetadata, LinkGraphNeighbor
from xiuxian_rag.link_graph import apply_link_graph_proximity_boost


class _FakeGraphBackend:
    backend_name = "wendao"

    async def neighbors(
        self,
        stem: str,
        *,
        direction: LinkGraphDirection = LinkGraphDirection.BOTH,
        hops: int = 1,
        limit: int = 50,
    ) -> list[LinkGraphNeighbor]:
        del direction, hops, limit
        if stem == "a":
            return [
                LinkGraphNeighbor(
                    stem="b",
                    direction=LinkGraphDirection.BOTH,
                    distance=1,
                    title="B",
                    path="docs/b.md",
                )
            ]
        if stem == "b":
            return [
                LinkGraphNeighbor(
                    stem="a",
                    direction=LinkGraphDirection.BOTH,
                    distance=1,
                    title="A",
                    path="docs/a.md",
                )
            ]
        return []

    async def metadata(self, stem: str) -> LinkGraphMetadata | None:
        if stem in {"a", "b"}:
            return LinkGraphMetadata(
                stem=stem,
                tags=["tag-x"],
                title=stem.upper(),
                path=f"docs/{stem}.md",
            )
        return None


class _SlowLinkGraphBackend:
    backend_name = "wendao"

    def __init__(self) -> None:
        self.neighbor_calls = 0
        self.metadata_calls = 0

    async def neighbors(
        self,
        stem: str,
        *,
        direction: LinkGraphDirection = LinkGraphDirection.BOTH,
        hops: int = 1,
        limit: int = 50,
    ) -> list[LinkGraphNeighbor]:
        del stem, direction, hops, limit
        self.neighbor_calls += 1
        await asyncio.sleep(0.05)
        return []

    async def metadata(self, stem: str) -> LinkGraphMetadata | None:
        del stem
        self.metadata_calls += 1
        await asyncio.sleep(0.05)
        return None


@pytest.mark.asyncio
async def test_apply_link_graph_proximity_boost_boosts_linked_and_tagged(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    from xiuxian_rag.link_graph import proximity as proximity_module

    backend = _FakeGraphBackend()
    proximity_module._stem_cache.clear()

    rows = [
        {"source": "docs/a.md", "score": 0.8, "content": "A"},
        {"source": "docs/b.md", "score": 0.6, "content": "B"},
        {"source": "docs/c.md", "score": 0.5, "content": "C"},
    ]

    out = await apply_link_graph_proximity_boost(
        rows,
        "query",
        backend=backend,
        notebook_dir="test_link_graph_boost",
    )
    by_source = {row["source"]: row["score"] for row in out}
    expected_boost = (
        proximity_module.DEFAULT_LINK_PROXIMITY_BOOST + proximity_module.DEFAULT_TAG_PROXIMITY_BOOST
    )
    assert by_source["docs/a.md"] == pytest.approx(0.8 + expected_boost, abs=0.001)
    assert by_source["docs/b.md"] == pytest.approx(0.6 + expected_boost, abs=0.001)
    assert by_source["docs/c.md"] == pytest.approx(0.5, abs=0.001)


@pytest.mark.asyncio
async def test_apply_link_graph_proximity_boost_passthrough_for_single_result() -> None:
    backend = _FakeGraphBackend()
    rows = [{"source": "docs/a.md", "score": 0.8, "content": "A"}]
    out = await apply_link_graph_proximity_boost(rows, "query", backend=backend)
    assert out == rows


@pytest.mark.asyncio
async def test_apply_link_graph_proximity_boost_respects_timeout_budget(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    from xiuxian_rag.link_graph import proximity as proximity_module

    rows = [
        {"source": "docs/a.md", "score": 0.8, "content": "A"},
        {"source": "docs/b.md", "score": 0.6, "content": "B"},
    ]

    def _fake_get_setting(key: str, default=None):
        values = {
            "link_graph.proximity.timeout_seconds": 0.01,
            "link_graph.proximity.max_parallel_stems": 1,
            "link_graph.proximity.stem_cache_ttl_seconds": 0,
        }
        return values.get(key, default)

    backend = _SlowLinkGraphBackend()
    monkeypatch.setattr(proximity_module, "get_setting", _fake_get_setting)
    out = await apply_link_graph_proximity_boost(rows, "query", backend=backend)
    assert [row["source"] for row in out] == ["docs/a.md", "docs/b.md"]
    assert [row["score"] for row in out] == [0.8, 0.6]
    assert backend.neighbor_calls >= 1
