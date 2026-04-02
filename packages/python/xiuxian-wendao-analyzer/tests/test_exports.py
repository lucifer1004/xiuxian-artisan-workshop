from __future__ import annotations

from inspect import isclass
from pathlib import Path
import tomllib

import xiuxian_wendao_analyzer as analyzer


def _pyproject_version() -> str:
    pyproject = Path(__file__).resolve().parents[1] / "pyproject.toml"
    return tomllib.loads(pyproject.read_text(encoding="utf-8"))["project"]["version"]


def test_public_exports_resolve_from_package_root() -> None:
    exported_names = analyzer.__all__

    assert exported_names
    assert len(exported_names) == len(set(exported_names))

    for name in exported_names:
        assert hasattr(analyzer, name), name


def test_public_exports_include_core_analyzer_surface() -> None:
    assert "__version__" in analyzer.__all__
    assert "AnalyzerConfig" in analyzer.__all__
    assert "AnalyzerResultRow" in analyzer.__all__
    assert "AnalysisSummary" in analyzer.__all__
    assert "RowsAnalysisRun" in analyzer.__all__
    assert "TableAnalysisRun" in analyzer.__all__
    assert "QueryAnalysisRun" in analyzer.__all__
    assert "RepoAnalysisRun" in analyzer.__all__
    assert "RerankAnalysisRun" in analyzer.__all__
    assert "analyze_query" in analyzer.__all__
    assert "analyze_repo_search" in analyzer.__all__
    assert "analyze_rerank_rows" in analyzer.__all__
    assert "run_query_analysis" in analyzer.__all__
    assert "run_repo_search_analysis" in analyzer.__all__
    assert "run_rerank_exchange_analysis" in analyzer.__all__
    assert "run_rerank_table_analysis" in analyzer.__all__
    assert "summarize_query_route" in analyzer.__all__
    assert "summarize_repo_query_text_results" in analyzer.__all__
    assert "summarize_rerank_exchange" in analyzer.__all__
    assert "summarize_rerank_table_results" in analyzer.__all__


def test_public_exports_preserve_expected_symbol_kinds() -> None:
    assert isclass(analyzer.AnalyzerConfig)
    assert isclass(analyzer.AnalyzerResultRow)
    assert isclass(analyzer.AnalysisSummary)
    assert isclass(analyzer.QueryAnalysisRun)
    assert isclass(analyzer.RepoAnalysisRun)
    assert isclass(analyzer.RerankAnalysisRun)
    assert isclass(analyzer.RowsAnalysisRun)
    assert isclass(analyzer.TableAnalysisRun)
    assert isclass(analyzer.LinearBlendAnalyzer)
    assert isclass(analyzer.ScoreRankAnalyzer)

    assert callable(analyzer.build_analyzer)
    assert callable(analyzer.parse_analyzer_result_rows)
    assert callable(analyzer.analyze_query)
    assert callable(analyzer.analyze_repo_search)
    assert callable(analyzer.analyze_rerank_rows)
    assert callable(analyzer.analyze_rerank_table)
    assert callable(analyzer.run_query_analysis)
    assert callable(analyzer.run_repo_search_analysis)
    assert callable(analyzer.run_rerank_exchange_analysis)
    assert callable(analyzer.run_rerank_table_analysis)
    assert callable(analyzer.summarize_query_route)
    assert callable(analyzer.summarize_repo_query_text_results)
    assert callable(analyzer.summarize_rerank_exchange)
    assert callable(analyzer.summarize_rerank_table_results)


def test_package_root_exports_version_matching_pyproject() -> None:
    assert analyzer.__version__ == "0.1.1"
    assert analyzer.__version__ == _pyproject_version()
