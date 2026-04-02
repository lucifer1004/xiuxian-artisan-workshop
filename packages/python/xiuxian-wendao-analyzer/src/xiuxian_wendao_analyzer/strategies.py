"""Baseline analyzer strategies built on the xiuxian-wendao-py substrate."""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Protocol

from .config import AnalyzerConfig


class AnalyzerStrategyProtocol(Protocol):
    """Protocol for one analyzer strategy implementation."""

    def analyze_rows(self, rows: list[dict[str, object]]) -> list[dict[str, object]]: ...


def _coerce_float(value: object, field_name: str) -> float:
    if isinstance(value, bool) or not isinstance(value, int | float):
        raise TypeError(f"{field_name} must be numeric")
    number = float(value)
    if not math.isfinite(number):
        raise ValueError(f"{field_name} must be finite")
    return number


def _coerce_vector(value: object, field_name: str) -> tuple[float, ...]:
    if not isinstance(value, tuple | list):
        raise TypeError(f"{field_name} must be a sequence of numbers")
    vector = tuple(_coerce_float(component, field_name) for component in value)
    if not vector:
        raise ValueError(f"{field_name} must not be empty")
    return vector


def cosine_similarity(left: tuple[float, ...], right: tuple[float, ...]) -> float:
    """Compute cosine similarity for two equal-length vectors."""

    if len(left) != len(right):
        raise ValueError("embedding and query_embedding must share one dimension")

    dot = sum(a * b for a, b in zip(left, right, strict=True))
    left_norm = math.sqrt(sum(component * component for component in left))
    right_norm = math.sqrt(sum(component * component for component in right))
    if left_norm == 0.0 or right_norm == 0.0:
        raise ValueError("embedding vectors must have non-zero norm")
    similarity = dot / (left_norm * right_norm)
    return max(-1.0, min(1.0, similarity))


@dataclass(frozen=True, slots=True)
class LinearBlendAnalyzer:
    """Deterministic baseline analyzer using vector and cosine similarity."""

    config: AnalyzerConfig

    def analyze_rows(self, rows: list[dict[str, object]]) -> list[dict[str, object]]:
        scored_rows: list[dict[str, object]] = []
        for row in rows:
            doc_id = row.get("doc_id")
            if not isinstance(doc_id, str) or not doc_id.strip():
                raise ValueError("doc_id must be a non-empty string")
            vector_score = _coerce_float(row.get("vector_score"), "vector_score")
            embedding = _coerce_vector(row.get("embedding"), "embedding")
            query_embedding = _coerce_vector(row.get("query_embedding"), "query_embedding")
            semantic_score = cosine_similarity(embedding, query_embedding)
            final_score = (
                self.config.vector_weight * vector_score
                + self.config.similarity_weight * semantic_score
            )
            scored_rows.append(
                {
                    "doc_id": doc_id,
                    "vector_score": vector_score,
                    "semantic_score": semantic_score,
                    "final_score": final_score,
                }
            )

        ranked_rows = sorted(
            scored_rows,
            key=lambda row: (-float(row["final_score"]), str(row["doc_id"])),
        )
        for index, row in enumerate(ranked_rows, start=1):
            row["rank"] = index
        return ranked_rows


@dataclass(frozen=True, slots=True)
class ScoreRankAnalyzer:
    """Deterministic analyzer that re-ranks rows by an existing score field."""

    config: AnalyzerConfig

    def analyze_rows(self, rows: list[dict[str, object]]) -> list[dict[str, object]]:
        ranked_rows: list[dict[str, object]] = []
        for row in rows:
            score = _coerce_float(row.get("score"), "score")
            ranked_rows.append({**row, "score": score})

        ranked_rows.sort(
            key=lambda row: (-float(row["score"]), str(row.get("doc_id") or row.get("path") or "")),
        )
        for index, row in enumerate(ranked_rows, start=1):
            row["rank"] = index
        return ranked_rows


def build_analyzer(config: AnalyzerConfig) -> AnalyzerStrategyProtocol:
    """Build the configured analyzer strategy."""

    if config.strategy == "linear_blend":
        return LinearBlendAnalyzer(config=config)
    return ScoreRankAnalyzer(config=config)


__all__ = [
    "AnalyzerStrategyProtocol",
    "LinearBlendAnalyzer",
    "ScoreRankAnalyzer",
    "build_analyzer",
    "cosine_similarity",
]
