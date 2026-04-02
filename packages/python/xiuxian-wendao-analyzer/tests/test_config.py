from __future__ import annotations

import pytest

from xiuxian_wendao_analyzer import AnalyzerConfig


def test_analyzer_config_defaults_to_linear_blend() -> None:
    config = AnalyzerConfig()

    assert config.strategy == "linear_blend"
    assert config.vector_weight == 0.35
    assert config.similarity_weight == 0.65


def test_analyzer_config_rejects_invalid_weights() -> None:
    with pytest.raises(ValueError, match="non-negative"):
        AnalyzerConfig(vector_weight=-0.1)
    with pytest.raises(ValueError, match="at least one analyzer weight must be positive"):
        AnalyzerConfig(vector_weight=0.0, similarity_weight=0.0)


def test_analyzer_config_accepts_score_rank_strategy() -> None:
    config = AnalyzerConfig(strategy="score_rank")

    assert config.strategy == "score_rank"
