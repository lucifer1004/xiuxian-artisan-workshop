from __future__ import annotations

import argparse

from wendao_arrow_interface import WendaoArrowScriptedClient, WendaoArrowSession
from wendao_core_lib import attachment_search_request
from xiuxian_wendao_analyzer import run_table_analysis, summarize_table_analysis


class PdfAttachmentAnalyzer:
    def analyze_rows(self, rows: list[dict[str, object]]) -> list[dict[str, object]]:
        ranked = sorted(
            (
                {
                    "doc_id": str(row["attachmentId"]),
                    "path": str(row["attachmentPath"]),
                    "score": float(row["score"]),
                    "attachment_name": str(row["attachmentName"]),
                    "source_title": str(row["sourceTitle"]),
                    "vision_snippet": (
                        str(row["visionSnippet"]) if row.get("visionSnippet") is not None else None
                    ),
                }
                for row in rows
                if str(row.get("attachmentExt", "")).lower() == "pdf"
            ),
            key=lambda row: (-float(row["score"]), str(row["path"])),
        )
        return [{**row, "rank": index} for index, row in enumerate(ranked, start=1)]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run a PDF attachment analyzer workflow over Rust-returned attachment-search rows.",
    )
    parser.add_argument("--mode", choices=("scripted", "endpoint"), default="scripted")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int)
    parser.add_argument("--query-text", default="architecture")
    parser.add_argument("--limit", type=int, default=5)
    parser.add_argument("--ext-filter", action="append", default=["pdf"])
    parser.add_argument("--kind-filter", action="append", default=["pdf"])
    parser.add_argument("--case-sensitive", action="store_true")
    parser.add_argument("--schema-version", default="v2")
    return parser.parse_args()


def build_scripted_session() -> WendaoArrowSession:
    return WendaoArrowSession.for_attachment_search_testing(
        [
            {
                "name": "Architecture PDF",
                "path": "notes/architecture.md#attachments/design-review.pdf",
                "sourceId": "doc-attachment-1",
                "sourceStem": "architecture",
                "sourceTitle": "Architecture Notes",
                "navigationTargetJson": '{"kind":"note","path":"notes/architecture.md"}',
                "sourcePath": "notes/architecture.md",
                "attachmentId": "attachment-1",
                "attachmentPath": "assets/design-review.pdf",
                "attachmentName": "design-review.pdf",
                "attachmentExt": "pdf",
                "kind": "pdf",
                "score": 0.82,
                "visionSnippet": "System design overview",
            },
            {
                "name": "Review PDF",
                "path": "notes/review.md#attachments/retro.pdf",
                "sourceId": "doc-attachment-2",
                "sourceStem": "review",
                "sourceTitle": "Review Notes",
                "navigationTargetJson": '{"kind":"note","path":"notes/review.md"}',
                "sourcePath": "notes/review.md",
                "attachmentId": "attachment-2",
                "attachmentPath": "assets/retro.pdf",
                "attachmentName": "retro.pdf",
                "attachmentExt": "pdf",
                "kind": "pdf",
                "score": 0.63,
                "visionSnippet": None,
            },
        ]
    )


def build_endpoint_session(args: argparse.Namespace) -> WendaoArrowSession:
    if args.port is None:
        raise ValueError("--port is required when --mode endpoint")
    return WendaoArrowSession.from_endpoint(
        host=args.host,
        port=args.port,
        schema_version=args.schema_version,
        request_timeout_seconds=10.0,
    )


def main() -> None:
    args = parse_args()
    session = build_scripted_session() if args.mode == "scripted" else build_endpoint_session(args)
    request = attachment_search_request(
        args.query_text,
        limit=args.limit,
        ext_filters=tuple(args.ext_filter),
        kind_filters=tuple(args.kind_filter),
        case_sensitive=args.case_sensitive,
    )

    result = session.attachment_search(request)
    typed_rows = result.parse_attachment_search_rows()
    run = run_table_analysis(result.table, analyzer=PdfAttachmentAnalyzer())
    summary = summarize_table_analysis(run)

    print("mode=", args.mode)
    print("query_text=", request.query_text)
    print("rows=", len(typed_rows))
    print("top_path=", summary.top_path)
    print("top_rank=", summary.top_rank)
    if run.rows_out:
        print("top_attachment_name=", run.rows_out[0].payload["attachment_name"])
        print("top_source_title=", run.rows_out[0].payload["source_title"])
    if isinstance(session.client, WendaoArrowScriptedClient):
        print("recorded_calls=", len(session.client.calls))
        print("recorded_route=", session.client.calls[0].route)


if __name__ == "__main__":
    main()
