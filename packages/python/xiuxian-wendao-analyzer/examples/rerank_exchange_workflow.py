from __future__ import annotations

import argparse

from xiuxian_wendao_analyzer import (
    run_rerank_exchange_analysis,
    summarize_rerank_exchange,
)
from xiuxian_wendao_py import (
    WendaoRerankRequestRow,
    WendaoTransportClient,
    WendaoTransportConfig,
    WendaoTransportEndpoint,
)


def build_sample_rows() -> list[WendaoRerankRequestRow]:
    return [
        WendaoRerankRequestRow(
            doc_id="doc-0",
            vector_score=0.5,
            embedding=(1.0, 0.0, 0.0),
            query_embedding=(1.0, 0.0, 0.0),
        ),
        WendaoRerankRequestRow(
            doc_id="doc-1",
            vector_score=0.8,
            embedding=(0.0, 1.0, 0.0),
            query_embedding=(1.0, 0.0, 0.0),
        ),
    ]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run a host-backed rerank exchange workflow.",
    )
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--schema-version", default="v2")
    parser.add_argument("--top-k", type=int, default=2)
    parser.add_argument("--min-final-score", type=float, default=None)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host=args.host, port=args.port),
            schema_version=args.schema_version,
            request_timeout_seconds=10.0,
        )
    )
    rows = build_sample_rows()
    run = run_rerank_exchange_analysis(
        client,
        rows,
        top_k=args.top_k,
        min_final_score=args.min_final_score,
    )
    summary = summarize_rerank_exchange(
        client,
        rows,
        top_k=args.top_k,
        min_final_score=args.min_final_score,
    )

    print("rows_in=", len(run.rows_in))
    print("rows_out=", len(run.rows_out))
    print("top_doc_id=", summary.top_doc_id)
    print("top_rank=", summary.top_rank)
    print("top_final_score=", summary.top_final_score)


if __name__ == "__main__":
    main()
