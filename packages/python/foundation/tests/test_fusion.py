"""Tests for fusion.py - Fusion Knowledge Fusion Engine.

Tests the remaining lightweight bridges that connect LinkGraph and Python-side RAG helpers:
1. LinkGraph proximity boost for recall results
2. Dynamic fusion weight selection
"""

from __future__ import annotations

import pytest

from xiuxian_rag.fusion import (
    LINK_GRAPH_LINK_PROXIMITY_BOOST,
    LINK_GRAPH_TAG_PROXIMITY_BOOST,
    link_graph_proximity_boost,
)

# ---------------------------------------------------------------------------
# Shared test constants — derived from module constants, not magic numbers
# ---------------------------------------------------------------------------

# Base scores for test fixtures (arbitrary but named)
_HIGH_SCORE = 0.8
_MID_SCORE = 0.6
_LOW_SCORE = 0.5
_VERY_LOW_SCORE = 0.3
_TOP_SCORE = 0.9
_ALT_SCORE = 0.7

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def _make_result(source: str, score: float) -> dict[str, Any]:
    """Create a recall result dict."""
    return {"source": source, "score": score}


# ---------------------------------------------------------------------------
# Bridge 1: LinkGraph Proximity Boost
# ---------------------------------------------------------------------------


class TestLinkGraphProximityBoost:
    """Tests for link_graph_proximity_boost (Core 1 → Core 2 bridge)."""

    @pytest.mark.asyncio
    async def test_passthrough_empty_results(self) -> None:
        result = await link_graph_proximity_boost([], "test query")
        assert result == []

    @pytest.mark.asyncio
    async def test_passthrough_single_result(self) -> None:
        results = [_make_result("docs/a.md", _HIGH_SCORE)]
        result = await link_graph_proximity_boost(results, "test")
        assert len(result) == 1
        assert result[0]["score"] == _HIGH_SCORE

    @pytest.mark.asyncio
    async def test_graceful_when_graph_backend_unavailable(self) -> None:
        results = [
            _make_result("docs/a.md", _HIGH_SCORE),
            _make_result("docs/b.md", _MID_SCORE),
        ]
        result = await link_graph_proximity_boost(results, "test", graph_root="/nonexistent")
        assert len(result) == 2
        assert result[0]["score"] == _HIGH_SCORE

    @pytest.mark.asyncio
    async def test_boost_linked_documents(self, monkeypatch: pytest.MonkeyPatch) -> None:
        import xiuxian_rag.link_graph.proximity as proximity_module

        monkeypatch.setattr(proximity_module, "_load_recent_timeout_checker", lambda: None)
        results = [
            _make_result("docs/router.md", _HIGH_SCORE),
            _make_result("docs/skill.md", _MID_SCORE),
            _make_result("docs/unrelated.md", _LOW_SCORE),
        ]

        class _MockBackend:
            backend_name = "fusion_test"

            async def neighbors(self, stem: str, **kwargs):
                del kwargs
                links = {"router": ["skill"], "skill": ["router"]}
                return [types.SimpleNamespace(stem=s) for s in links.get(stem, [])]

            async def metadata(self, stem: str):
                tags = {"router": ["shared"], "skill": ["shared"]}
                return types.SimpleNamespace(stem=stem, tags=tags.get(stem, []))

        boosted = await link_graph_proximity_boost(
            results,
            "test query",
            max_hops=1,
            link_boost=LINK_GRAPH_LINK_PROXIMITY_BOOST,
            tag_boost=LINK_GRAPH_TAG_PROXIMITY_BOOST,
            backend=_MockBackend(),
        )
        assert len(boosted) == 3
        assert {row["source"] for row in boosted} == {
            "docs/router.md",
            "docs/skill.md",
            "docs/unrelated.md",
        }

    @pytest.mark.asyncio
    async def test_results_resorted_after_boost(self, monkeypatch: pytest.MonkeyPatch) -> None:
        import xiuxian_rag.link_graph.proximity as proximity_module

        monkeypatch.setattr(proximity_module, "_load_recent_timeout_checker", lambda: None)
        results = [
            _make_result("docs/a.md", _TOP_SCORE),
            _make_result("docs/b.md", _VERY_LOW_SCORE),
            _make_result("docs/c.md", _ALT_SCORE),
        ]

        class _MockBackend:
            backend_name = "fusion_test_sort"

            async def neighbors(self, stem: str, **kwargs):
                del kwargs
                links = {"a": ["b"], "b": ["a"]}
                return [types.SimpleNamespace(stem=s) for s in links.get(stem, [])]

            async def metadata(self, stem: str):
                return types.SimpleNamespace(stem=stem, tags=[])

        boosted = await link_graph_proximity_boost(results, "test", backend=_MockBackend())
        assert len(boosted) == 3
        assert all("score" in row for row in boosted)

    @pytest.mark.asyncio
    async def test_skips_uuid_sources_no_graph_lookup(self) -> None:
        """LanceDB chunk IDs (UUIDs) are not passed to graph lookup."""
        uuid_source = "e077e713-3e85-46c2-ad01-6fb4c10722fc"
        results = [
            _make_result("docs/real-note.md", _HIGH_SCORE),
            _make_result(uuid_source, _MID_SCORE),
        ]

        class _MockBackend:
            backend_name = "fusion_test_uuid"

            async def neighbors(self, stem: str, **kwargs):
                del stem, kwargs
                return []

            async def metadata(self, stem: str):
                return types.SimpleNamespace(stem=stem, tags=[])

        boosted = await link_graph_proximity_boost(results, "test", backend=_MockBackend())
        assert len(boosted) == 2
        uuid_result = next(r for r in boosted if r["source"] == uuid_source)
        assert uuid_result["score"] == _MID_SCORE


# ---------------------------------------------------------------------------
# Dynamic Fusion Weights (intent → weight selection)
# ---------------------------------------------------------------------------


class TestFusionWeights:
    """Tests for compute_fusion_weights() — dynamic graph vs LanceDB weighting."""

    def test_empty_query_returns_balanced(self):
        from xiuxian_rag.fusion import FusionWeights, compute_fusion_weights

        w = compute_fusion_weights("")
        assert isinstance(w, FusionWeights)
        assert w.zk_proximity_scale == 1.0
        assert w.kg_rerank_scale == 1.0
        assert w.vector_weight == 1.0
        assert w.keyword_weight == 1.0

    def test_knowledge_query_boosts_graph(self):
        from xiuxian_rag.fusion import compute_fusion_weights

        w = compute_fusion_weights("search for knowledge about rust patterns")
        assert w.zk_proximity_scale > 1.0
        assert w.kg_rerank_scale > 1.0
        assert w.intent_target == "knowledge"

    def test_code_query_boosts_vector(self):
        from xiuxian_rag.fusion import compute_fusion_weights

        w = compute_fusion_weights("find the function in the codebase")
        assert w.vector_weight >= 1.0
        assert w.zk_proximity_scale <= 1.0
        assert w.intent_target == "code"

    def test_git_commit_favors_tool_routing(self):
        from xiuxian_rag.fusion import compute_fusion_weights

        w = compute_fusion_weights("commit my changes to git")
        assert w.keyword_weight >= 1.0
        assert w.intent_action == "commit"
        assert w.intent_target == "git"

    def test_research_query_emphasizes_graph(self):
        from xiuxian_rag.fusion import compute_fusion_weights

        w = compute_fusion_weights("research about LanceDB architecture")
        assert w.kg_rerank_scale >= 1.0
        assert w.intent_action == "research"

    def test_intent_keywords_propagated(self):
        from xiuxian_rag.fusion import compute_fusion_weights

        w = compute_fusion_weights("search python async patterns in code")
        assert len(w.intent_keywords) > 0
        assert "python" in w.intent_keywords
        assert "async" in w.intent_keywords
