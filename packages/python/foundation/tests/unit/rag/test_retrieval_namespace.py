"""Unit tests for the remaining retrieval namespace surface."""

from __future__ import annotations

from xiuxian_rag import HybridRetrievalBackend, HybridRetrievalUnavailableError, RetrievalConfig
from xiuxian_rag.retrieval.interface import RetrievalResult


class _StaticBackend:
    def __init__(self, results: list[RetrievalResult], name: str):
        self._results = results
        self._name = name

    async def search(self, query: str, config: RetrievalConfig):
        del query, config
        return self._results

    async def index(self, documents, collection: str):
        del collection
        return len(documents)

    async def get_stats(self, collection: str):
        del collection
        return {"backend": self._name, "count": len(self._results)}


async def test_hybrid_backend_requires_native_hybrid_method() -> None:
    vector = _StaticBackend(
        [RetrievalResult(id="a", content="alpha", score=0.9, source="vector")],
        "vector",
    )
    backend = HybridRetrievalBackend(vector_backend=vector)

    try:
        await backend.search("typed", RetrievalConfig(top_k=5))
    except HybridRetrievalUnavailableError:
        pass
    else:
        raise AssertionError("Hybrid backend should reject Python backends without search_hybrid")


async def test_hybrid_backend_stats_include_children() -> None:
    class _NativeHybridBackend(_StaticBackend):
        async def search_hybrid(self, query: str, config: RetrievalConfig):
            del query, config
            return [RetrievalResult(id="a", content="alpha", score=0.9, source="hybrid")]

    vector = _NativeHybridBackend([RetrievalResult(id="a", content="alpha", score=0.9)], "vector")
    backend = HybridRetrievalBackend(vector)
    stats = await backend.get_stats("knowledge")
    assert stats["backend"] == "hybrid"
    assert stats["engine_owner"] == "rust"
    assert stats["vector"]["backend"] == "vector"


async def test_hybrid_backend_applies_threshold_filter() -> None:
    class _NativeHybridBackend(_StaticBackend):
        async def search_hybrid(self, query: str, config: RetrievalConfig):
            del query, config
            return [
                RetrievalResult(id="a", content="top", score=0.9, source="hybrid"),
                RetrievalResult(id="b", content="low", score=0.2, source="hybrid"),
            ]

    vector = _NativeHybridBackend([], "vector")
    backend = HybridRetrievalBackend(vector)
    results = await backend.search("python type", RetrievalConfig(top_k=2, score_threshold=0.5))
    assert [r.id for r in results] == ["a"]


async def test_hybrid_backend_prefers_native_hybrid_when_available() -> None:
    class _NativeHybridBackend(_StaticBackend):
        def __init__(self):
            super().__init__([RetrievalResult(id="ignored", content="ignored", score=0.1)], "vector")
            self.hybrid_calls = 0

        async def search_hybrid(self, query: str, config: RetrievalConfig):
            del query, config
            self.hybrid_calls += 1
            return [RetrievalResult(id="native", content="native", score=0.8, source="hybrid")]

    vector = _NativeHybridBackend()
    backend = HybridRetrievalBackend(vector)
    results = await backend.search("hybrid", RetrievalConfig(top_k=3))
    assert [r.id for r in results] == ["native"]
    assert vector.hybrid_calls == 1
