from __future__ import annotations

import pyarrow as pa
from xiuxian_wendao_py import WendaoRerankRequestRow, build_rerank_request_table

from xiuxian_wendao_analyzer import (
    AnalyzerConfig,
    analyze_result_rows,
    analyze_rerank_result_rows,
    analyze_rerank_rows,
    analyze_rerank_table,
    analyze_rerank_table_results,
    analyze_rows,
    analyze_table,
    analyze_table_results,
    build_analyzer,
    run_rows_analysis,
    run_rerank_analysis,
    run_rerank_table_analysis,
    run_table_analysis,
    summarize_repo_query_text,
    summarize_repo_query_text_results,
    summarize_result_rows,
    summarize_rerank_analysis,
    summarize_rerank_result_rows,
    summarize_rows,
    summarize_rows_analysis,
    summarize_table,
    summarize_table_analysis,
    summarize_rerank_table,
    summarize_rerank_table_results,
    summarize_rerank_rows,
)


def test_analyze_rows_returns_ranked_linear_blend_results() -> None:
    rows = [
        {
            "doc_id": "doc-a",
            "vector_score": 0.2,
            "embedding": (1.0, 0.0),
            "query_embedding": (1.0, 0.0),
        },
        {
            "doc_id": "doc-b",
            "vector_score": 0.9,
            "embedding": (0.0, 1.0),
            "query_embedding": (1.0, 0.0),
        },
    ]

    ranked = analyze_rows(rows)

    assert [row["doc_id"] for row in ranked] == ["doc-a", "doc-b"]
    assert ranked[0]["rank"] == 1
    assert ranked[1]["rank"] == 2
    assert float(ranked[0]["semantic_score"]) > float(ranked[1]["semantic_score"])
    assert float(ranked[0]["final_score"]) > float(ranked[1]["final_score"])


def test_analyze_table_uses_provided_analyzer() -> None:
    table = pa.Table.from_pylist(
        [
            {
                "doc_id": "doc-a",
                "vector_score": 0.1,
                "embedding": (1.0, 0.0),
                "query_embedding": (1.0, 0.0),
            }
        ]
    )

    analyzer = build_analyzer(AnalyzerConfig(vector_weight=1.0, similarity_weight=0.0))
    ranked = analyze_table(table, analyzer=analyzer)

    assert ranked == [
        {
            "doc_id": "doc-a",
            "vector_score": 0.1,
            "semantic_score": 1.0,
            "final_score": 0.1,
            "rank": 1,
        }
    ]


def test_analyze_result_rows_returns_typed_results() -> None:
    rows = [
        {
            "doc_id": "doc-a",
            "vector_score": 0.2,
            "embedding": (1.0, 0.0),
            "query_embedding": (1.0, 0.0),
        }
    ]

    ranked = analyze_result_rows(rows)

    assert len(ranked) == 1
    assert ranked[0].doc_id == "doc-a"
    assert ranked[0].rank == 1
    assert ranked[0].final_score is not None


def test_analyze_table_results_returns_typed_results() -> None:
    table = pa.Table.from_pylist(
        [
            {
                "doc_id": "doc-a",
                "vector_score": 0.1,
                "embedding": (1.0, 0.0),
                "query_embedding": (1.0, 0.0),
            }
        ]
    )

    ranked = analyze_table_results(table)

    assert len(ranked) == 1
    assert ranked[0].doc_id == "doc-a"
    assert ranked[0].rank == 1


def test_analyze_rerank_rows_accepts_typed_request_rows() -> None:
    rows = [
        WendaoRerankRequestRow(
            doc_id="doc-a",
            vector_score=0.2,
            embedding=(1.0, 0.0),
            query_embedding=(1.0, 0.0),
        ),
        WendaoRerankRequestRow(
            doc_id="doc-b",
            vector_score=0.9,
            embedding=(0.0, 1.0),
            query_embedding=(1.0, 0.0),
        ),
    ]

    ranked = analyze_rerank_rows(rows)

    assert [row["doc_id"] for row in ranked] == ["doc-a", "doc-b"]
    assert [row["rank"] for row in ranked] == [1, 2]
    assert float(ranked[0]["final_score"]) > float(ranked[1]["final_score"])


def test_analyze_rerank_result_rows_returns_typed_results() -> None:
    rows = [
        WendaoRerankRequestRow(
            doc_id="doc-a",
            vector_score=0.2,
            embedding=(1.0, 0.0),
            query_embedding=(1.0, 0.0),
        )
    ]

    ranked = analyze_rerank_result_rows(rows)

    assert len(ranked) == 1
    assert ranked[0].doc_id == "doc-a"
    assert ranked[0].rank == 1
    assert ranked[0].final_score is not None


def test_analyze_rerank_table_accepts_typed_request_table() -> None:
    table = build_rerank_request_table(
        [
            WendaoRerankRequestRow(
                doc_id="doc-a",
                vector_score=0.2,
                embedding=(1.0, 0.0),
                query_embedding=(1.0, 0.0),
            ),
            WendaoRerankRequestRow(
                doc_id="doc-b",
                vector_score=0.9,
                embedding=(0.0, 1.0),
                query_embedding=(1.0, 0.0),
            ),
        ]
    )

    ranked = analyze_rerank_table(table)

    assert [row["doc_id"] for row in ranked] == ["doc-a", "doc-b"]
    assert [row["rank"] for row in ranked] == [1, 2]
    assert float(ranked[0]["final_score"]) > float(ranked[1]["final_score"])


def test_analyze_rerank_table_results_returns_typed_results() -> None:
    table = build_rerank_request_table(
        [
            WendaoRerankRequestRow(
                doc_id="doc-a",
                vector_score=0.2,
                embedding=(1.0, 0.0),
                query_embedding=(1.0, 0.0),
            )
        ]
    )

    ranked = analyze_rerank_table_results(table)

    assert len(ranked) == 1
    assert ranked[0].doc_id == "doc-a"
    assert ranked[0].rank == 1
    assert ranked[0].final_score is not None


def test_analyze_rows_supports_score_rank_strategy() -> None:
    rows = [
        {"path": "src/main.rs", "score": 0.3},
        {"path": "src/lib.rs", "score": 0.9},
    ]

    ranked = analyze_rows(rows, config=AnalyzerConfig(strategy="score_rank"))

    assert [row["path"] for row in ranked] == ["src/lib.rs", "src/main.rs"]
    assert [row["rank"] for row in ranked] == [1, 2]


def test_run_rows_analysis_preserves_input_and_typed_output() -> None:
    rows = [
        {"path": "src/main.rs", "score": 0.3},
        {"path": "src/lib.rs", "score": 0.9},
    ]

    run = run_rows_analysis(rows, config=AnalyzerConfig(strategy="score_rank"))

    assert [row["path"] for row in run.rows_in] == ["src/main.rs", "src/lib.rs"]
    assert [row.path for row in run.rows_out] == ["src/lib.rs", "src/main.rs"]
    assert [row.rank for row in run.rows_out] == [1, 2]


def test_run_table_analysis_preserves_input_and_typed_output() -> None:
    table = pa.Table.from_pylist(
        [
            {"path": "src/main.rs", "score": 0.3},
            {"path": "src/lib.rs", "score": 0.9},
        ]
    )

    run = run_table_analysis(table, config=AnalyzerConfig(strategy="score_rank"))

    assert run.table_in == table
    assert [row.path for row in run.rows_out] == ["src/lib.rs", "src/main.rs"]
    assert [row.rank for row in run.rows_out] == [1, 2]


def test_analyze_result_rows_supports_score_rank_strategy() -> None:
    rows = [
        {"path": "src/main.rs", "score": 0.3},
        {"path": "src/lib.rs", "score": 0.9},
    ]

    ranked = analyze_result_rows(rows, config=AnalyzerConfig(strategy="score_rank"))

    assert [row.path for row in ranked] == ["src/lib.rs", "src/main.rs"]
    assert [row.rank for row in ranked] == [1, 2]


def test_run_rerank_analysis_preserves_input_and_typed_output() -> None:
    rows = [
        WendaoRerankRequestRow(
            doc_id="doc-a",
            vector_score=0.2,
            embedding=(1.0, 0.0),
            query_embedding=(1.0, 0.0),
        ),
        WendaoRerankRequestRow(
            doc_id="doc-b",
            vector_score=0.9,
            embedding=(0.0, 1.0),
            query_embedding=(1.0, 0.0),
        ),
    ]

    run = run_rerank_analysis(rows)

    assert [row.doc_id for row in run.rows_in] == ["doc-a", "doc-b"]
    assert [row.doc_id for row in run.rows_out] == ["doc-a", "doc-b"]
    assert [row.rank for row in run.rows_out] == [1, 2]


def test_run_rerank_table_analysis_preserves_input_and_typed_output() -> None:
    table = build_rerank_request_table(
        [
            WendaoRerankRequestRow(
                doc_id="doc-a",
                vector_score=0.2,
                embedding=(1.0, 0.0),
                query_embedding=(1.0, 0.0),
            ),
            WendaoRerankRequestRow(
                doc_id="doc-b",
                vector_score=0.9,
                embedding=(0.0, 1.0),
                query_embedding=(1.0, 0.0),
            ),
        ]
    )

    run = run_rerank_table_analysis(table)

    assert [row.doc_id for row in run.rows_in] == ["doc-a", "doc-b"]
    assert [row.doc_id for row in run.rows_out] == ["doc-a", "doc-b"]
    assert [row.rank for row in run.rows_out] == [1, 2]


def test_summarize_result_rows_uses_first_ranked_row() -> None:
    rows = analyze_result_rows(
        [
            {
                "doc_id": "doc-a",
                "vector_score": 0.2,
                "embedding": (1.0, 0.0),
                "query_embedding": (1.0, 0.0),
            },
            {
                "doc_id": "doc-b",
                "vector_score": 0.9,
                "embedding": (0.0, 1.0),
                "query_embedding": (1.0, 0.0),
            },
        ]
    )

    summary = summarize_result_rows(rows)

    assert summary.row_count == 2
    assert summary.top_doc_id == "doc-a"
    assert summary.top_rank == 1
    assert summary.top_final_score is not None


def test_summarize_rows_returns_top_row_snapshot() -> None:
    summary = summarize_rows(
        [
            {
                "doc_id": "doc-a",
                "vector_score": 0.2,
                "embedding": (1.0, 0.0),
                "query_embedding": (1.0, 0.0),
            },
            {
                "doc_id": "doc-b",
                "vector_score": 0.9,
                "embedding": (0.0, 1.0),
                "query_embedding": (1.0, 0.0),
            },
        ]
    )

    assert summary.row_count == 2
    assert summary.top_doc_id == "doc-a"
    assert summary.top_rank == 1


def test_summarize_table_returns_top_row_snapshot() -> None:
    table = pa.Table.from_pylist(
        [
            {
                "doc_id": "doc-a",
                "vector_score": 0.2,
                "embedding": (1.0, 0.0),
                "query_embedding": (1.0, 0.0),
            },
            {
                "doc_id": "doc-b",
                "vector_score": 0.9,
                "embedding": (0.0, 1.0),
                "query_embedding": (1.0, 0.0),
            },
        ]
    )

    summary = summarize_table(table)

    assert summary.row_count == 2
    assert summary.top_doc_id == "doc-a"
    assert summary.top_rank == 1


def test_summarize_rerank_analysis_returns_top_row_snapshot() -> None:
    run = run_rerank_analysis(
        [
            WendaoRerankRequestRow(
                doc_id="doc-a",
                vector_score=0.2,
                embedding=(1.0, 0.0),
                query_embedding=(1.0, 0.0),
            ),
            WendaoRerankRequestRow(
                doc_id="doc-b",
                vector_score=0.9,
                embedding=(0.0, 1.0),
                query_embedding=(1.0, 0.0),
            ),
        ]
    )

    summary = summarize_rerank_analysis(run)

    assert summary.row_count == 2
    assert summary.top_doc_id == "doc-a"
    assert summary.top_rank == 1


def test_local_rerank_v1_workflow_keeps_run_and_summary_in_sync() -> None:
    rows = [
        WendaoRerankRequestRow(
            doc_id="doc-a",
            vector_score=0.2,
            embedding=(1.0, 0.0),
            query_embedding=(1.0, 0.0),
        ),
        WendaoRerankRequestRow(
            doc_id="doc-b",
            vector_score=0.9,
            embedding=(0.0, 1.0),
            query_embedding=(1.0, 0.0),
        ),
    ]

    run = run_rerank_analysis(rows)
    summary = summarize_rerank_analysis(run)

    assert list(run.rows_in) == rows
    assert len(run.rows_out) == 2
    assert run.rows_out[0].doc_id == summary.top_doc_id == "doc-a"
    assert run.rows_out[0].rank == summary.top_rank == 1
    assert summary.row_count == len(run.rows_out)


def test_summarize_rows_analysis_returns_top_row_snapshot() -> None:
    run = run_rows_analysis(
        [
            {"path": "src/main.rs", "score": 0.3},
            {"path": "src/lib.rs", "score": 0.9},
        ],
        config=AnalyzerConfig(strategy="score_rank"),
    )

    summary = summarize_rows_analysis(run)

    assert summary.row_count == 2
    assert summary.top_path == "src/lib.rs"
    assert summary.top_rank == 1


def test_summarize_table_analysis_returns_top_row_snapshot() -> None:
    run = run_table_analysis(
        pa.Table.from_pylist(
            [
                {"path": "src/main.rs", "score": 0.3},
                {"path": "src/lib.rs", "score": 0.9},
            ]
        ),
        config=AnalyzerConfig(strategy="score_rank"),
    )

    summary = summarize_table_analysis(run)

    assert summary.row_count == 2
    assert summary.top_path == "src/lib.rs"
    assert summary.top_rank == 1


def test_summarize_rerank_rows_returns_top_row_snapshot() -> None:
    summary = summarize_rerank_rows(
        [
            WendaoRerankRequestRow(
                doc_id="doc-a",
                vector_score=0.2,
                embedding=(1.0, 0.0),
                query_embedding=(1.0, 0.0),
            ),
            WendaoRerankRequestRow(
                doc_id="doc-b",
                vector_score=0.9,
                embedding=(0.0, 1.0),
                query_embedding=(1.0, 0.0),
            ),
        ]
    )

    assert summary.row_count == 2
    assert summary.top_doc_id == "doc-a"
    assert summary.top_rank == 1


def test_summarize_rerank_table_returns_top_row_snapshot() -> None:
    table = build_rerank_request_table(
        [
            WendaoRerankRequestRow(
                doc_id="doc-a",
                vector_score=0.2,
                embedding=(1.0, 0.0),
                query_embedding=(1.0, 0.0),
            ),
            WendaoRerankRequestRow(
                doc_id="doc-b",
                vector_score=0.9,
                embedding=(0.0, 1.0),
                query_embedding=(1.0, 0.0),
            ),
        ]
    )

    summary = summarize_rerank_table(table)

    assert summary.row_count == 2
    assert summary.top_doc_id == "doc-a"
    assert summary.top_rank == 1


def test_summarize_rerank_result_rows_returns_top_row_snapshot() -> None:
    summary = summarize_rerank_result_rows(
        [
            WendaoRerankRequestRow(
                doc_id="doc-a",
                vector_score=0.2,
                embedding=(1.0, 0.0),
                query_embedding=(1.0, 0.0),
            ),
            WendaoRerankRequestRow(
                doc_id="doc-b",
                vector_score=0.9,
                embedding=(0.0, 1.0),
                query_embedding=(1.0, 0.0),
            ),
        ]
    )

    assert summary.row_count == 2
    assert summary.top_doc_id == "doc-a"
    assert summary.top_rank == 1


def test_summarize_rerank_table_results_returns_top_row_snapshot() -> None:
    table = build_rerank_request_table(
        [
            WendaoRerankRequestRow(
                doc_id="doc-a",
                vector_score=0.2,
                embedding=(1.0, 0.0),
                query_embedding=(1.0, 0.0),
            ),
            WendaoRerankRequestRow(
                doc_id="doc-b",
                vector_score=0.9,
                embedding=(0.0, 1.0),
                query_embedding=(1.0, 0.0),
            ),
        ]
    )

    summary = summarize_rerank_table_results(table)

    assert summary.row_count == 2
    assert summary.top_doc_id == "doc-a"
    assert summary.top_rank == 1
