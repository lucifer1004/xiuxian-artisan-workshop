from __future__ import annotations

from wendao_arrow_interface import WendaoArrowScriptedClient, WendaoArrowSession
from xiuxian_wendao_analyzer import run_repo_analysis, summarize_repo_analysis


class CustomScoreAnalyzer:
    def analyze_rows(self, rows: list[dict[str, object]]) -> list[dict[str, object]]:
        ranked = sorted(rows, key=lambda row: float(row["score"]), reverse=True)
        return [
            {
                "doc_id": str(row["doc_id"]),
                "path": str(row["path"]),
                "score": float(row["score"]),
                "rank": index + 1,
            }
            for index, row in enumerate(ranked)
        ]


def build_session() -> WendaoArrowSession:
    return WendaoArrowSession.for_repo_search_testing(
        [
            {"doc_id": "doc-alpha", "path": "src/alpha.py", "score": 0.91},
            {"doc_id": "doc-beta", "path": "docs/alpha.md", "score": 0.44},
            {"doc_id": "doc-gamma", "path": "src/beta.py", "score": 0.72},
        ]
    )


def main() -> None:
    session = build_session()
    if not isinstance(session.client, WendaoArrowScriptedClient):
        raise TypeError("scripted example expects WendaoArrowSession.for_repo_search_testing()")

    run = run_repo_analysis(
        session.client,
        "alpha",
        limit=3,
        analyzer=CustomScoreAnalyzer(),
    )
    summary = summarize_repo_analysis(run)
    recorded_call = session.client.calls[0]

    print("query_text=", run.request.query_text)
    print("rows=", len(run.rows))
    print("top_path=", summary.top_path)
    print("top_rank=", summary.top_rank)
    print("recorded_calls=", len(session.client.calls))
    print("recorded_route=", recorded_call.route)


if __name__ == "__main__":
    main()
