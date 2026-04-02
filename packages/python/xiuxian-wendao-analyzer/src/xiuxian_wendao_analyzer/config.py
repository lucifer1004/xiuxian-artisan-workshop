"""Analyzer-local configuration for xiuxian-wendao-analyzer."""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import Literal


AnalyzerStrategy = Literal["linear_blend", "score_rank"]


@dataclass(frozen=True, slots=True)
class AnalyzerConfig:
    """Configuration for one analyzer strategy instance."""

    strategy: AnalyzerStrategy = "linear_blend"
    vector_weight: float = 0.35
    similarity_weight: float = 0.65

    def __post_init__(self) -> None:
        if self.strategy not in {"linear_blend", "score_rank"}:
            raise ValueError(f"unsupported analyzer strategy: {self.strategy}")
        if not math.isfinite(self.vector_weight):
            raise ValueError("vector_weight must be finite")
        if not math.isfinite(self.similarity_weight):
            raise ValueError("similarity_weight must be finite")
        if self.vector_weight < 0.0:
            raise ValueError("vector_weight must be non-negative")
        if self.similarity_weight < 0.0:
            raise ValueError("similarity_weight must be non-negative")
        if self.vector_weight == 0.0 and self.similarity_weight == 0.0:
            raise ValueError("at least one analyzer weight must be positive")


__all__ = ["AnalyzerConfig", "AnalyzerStrategy"]
