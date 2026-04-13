from __future__ import annotations

import argparse

import polars as pl

from wendao_arrow_interface import WendaoArrowScriptedClient, WendaoArrowSession
from wendao_core_lib import attachment_search_request


def summarize_pdf_frame(frame: pl.DataFrame) -> dict[str, object]:
    pdf_frame = frame.filter(pl.col("attachmentExt").str.to_lowercase() == "pdf").sort(
        "score", descending=True
    )
    top_row = pdf_frame.row(0, named=True) if pdf_frame.height else None
    return {
        "row_count": pdf_frame.height,
        "top_attachment_name": str(top_row["attachmentName"]) if top_row is not None else None,
        "top_source_title": str(top_row["sourceTitle"]) if top_row is not None else None,
        "top_score": float(top_row["score"]) if top_row is not None else None,
    }


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Run an attachment-search workflow that keeps Arrow as the transport "
            "surface and then analyzes PDF rows through Polars."
        ),
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
                "name": "Retro PDF",
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
    frame = result.to_polars()
    analysis = summarize_pdf_frame(frame)

    print("mode=", args.mode)
    print("query_text=", request.query_text)
    print("arrow_rows=", result.table.num_rows)
    print("polars_rows=", frame.height)
    print("top_attachment_name=", analysis["top_attachment_name"])
    print("top_source_title=", analysis["top_source_title"])
    print("top_score=", analysis["top_score"])
    if isinstance(session.client, WendaoArrowScriptedClient):
        print("recorded_calls=", len(session.client.calls))
        print("recorded_route=", session.client.calls[0].route)


if __name__ == "__main__":
    main()
