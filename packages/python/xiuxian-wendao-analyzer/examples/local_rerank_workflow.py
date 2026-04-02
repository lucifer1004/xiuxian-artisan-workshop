from __future__ import annotations

from xiuxian_wendao_analyzer import run_rerank_analysis, summarize_rerank_analysis
from xiuxian_wendao_py import WendaoRerankRequestRow


def build_sample_rows() -> list[WendaoRerankRequestRow]:
    return [
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


def main() -> None:
    run = run_rerank_analysis(build_sample_rows())
    summary = summarize_rerank_analysis(run)

    print("rows_in=", len(run.rows_in))
    print("rows_out=", len(run.rows_out))
    print("top_doc_id=", summary.top_doc_id)
    print("top_rank=", summary.top_rank)


if __name__ == "__main__":
    main()
