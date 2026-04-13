from __future__ import annotations

import polars as pl
import pyarrow as pa

from wendao_arrow_interface import (
    SEARCH_ATTACHMENTS_ROUTE,
    REPO_SEARCH_ROUTE,
    RERANK_EXCHANGE_ROUTE,
    WendaoArrowCall,
    WendaoArrowResult,
    WendaoArrowScriptedClient,
    WendaoArrowSession,
)
from wendao_core_lib import (
    attachment_search_metadata,
    attachment_search_request,
    WendaoAttachmentSearchRequest,
    WendaoRerankRequestRow,
    build_rerank_request_table,
    repo_search_metadata,
    repo_search_request,
    rerank_request_metadata,
)


def _repo_search_table() -> pa.Table:
    return pa.Table.from_pylist(
        [
            {
                "doc_id": "doc-1",
                "path": "packages/rust/crates/xiuxian-wendao/src/lib.rs",
                "title": "xiuxian-wendao lib",
                "best_section": "score blend",
                "match_reason": "title",
                "navigation_path": "packages/rust/crates/xiuxian-wendao/src/lib.rs",
                "navigation_category": "code",
                "navigation_line": 12,
                "navigation_line_end": 20,
                "hierarchy": ["packages", "rust", "crates"],
                "tags": ["wendao", "search"],
                "score": 0.9,
                "language": "rust",
            }
        ]
    )


def _rerank_response_table() -> pa.Table:
    return pa.Table.from_pylist(
        [
            {
                "doc_id": "doc-1",
                "vector_score": 0.7,
                "semantic_score": 0.8,
                "final_score": 0.76,
                "rank": 1,
            }
        ]
    )


def _attachment_search_table() -> pa.Table:
    return pa.Table.from_pylist(
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
            }
        ]
    )


def test_session_from_endpoint_builds_transport_client() -> None:
    session = WendaoArrowSession.from_endpoint(
        host="127.0.0.1",
        port=50051,
        metadata={"authorization": "Bearer test"},
    )

    assert session.client.config.schema_version == "v2"
    assert session.client.config.endpoint.metadata["authorization"] == "Bearer test"
    assert session.client.config.endpoint.path == "/"


def test_query_wraps_transport_table_fetch() -> None:
    session = WendaoArrowSession.for_query_testing("/search/repos/main", _repo_search_table())

    result = session.query(
        "/search/repos/main",
        extra_metadata={"x-test": "1"},
        tls_root_certs=b"roots",
    )

    assert result.route == "/search/repos/main"
    assert result.query is not None
    assert result.query.normalized_route() == "/search/repos/main"
    assert result.table.column_names[0] == "doc_id"
    assert isinstance(session.client, WendaoArrowScriptedClient)
    assert session.client.calls == [
        WendaoArrowCall(
            operation="query",
            route="/search/repos/main",
            query=result.query,
            extra_metadata={"x-test": "1"},
            connect_kwargs={"tls_root_certs": b"roots"},
        )
    ]


def test_repo_search_accepts_typed_queue_and_records_effective_metadata() -> None:
    first_request = repo_search_request("graph search", limit=5, language_filters=("rust",))
    second_request = repo_search_request("graph search", limit=2, language_filters=("python",))
    second_rows = [
        {
            **_repo_search_table().to_pylist()[0],
            "doc_id": "doc-2",
            "path": "packages/python/wendao-arrow-interface/src/wendao_arrow_interface/result.py",
            "score": 0.77,
            "language": "python",
        }
    ]
    scripted = WendaoArrowScriptedClient()
    scripted.add_repo_search_response(_repo_search_table(), request=first_request)
    scripted.add_repo_search_response(second_rows, request=second_request)
    session = WendaoArrowSession.from_client(scripted)

    first_result = session.repo_search(first_request)
    second_result = session.repo_search(second_request)

    first_rows = first_result.parse_repo_search_rows()
    second_rows = second_result.parse_repo_search_rows()

    assert first_result.request == first_request
    assert second_result.request == second_request
    assert first_rows[0].doc_id == "doc-1"
    assert second_rows[0].doc_id == "doc-2"
    assert second_rows[0].language == "python"
    assert scripted.calls == [
        WendaoArrowCall(
            operation="repo_search",
            route="/search/repos/main",
            request=first_request,
            extra_metadata=repo_search_metadata(first_request),
            connect_kwargs={},
        ),
        WendaoArrowCall(
            operation="repo_search",
            route="/search/repos/main",
            request=second_request,
            extra_metadata=repo_search_metadata(second_request),
            connect_kwargs={},
        ),
    ]


def test_attachment_search_accepts_typed_queue_and_records_effective_metadata() -> None:
    first_request = attachment_search_request(
        "architecture",
        limit=5,
        ext_filters=("pdf",),
        kind_filters=("pdf",),
    )
    second_request = attachment_search_request(
        "roadmap",
        limit=2,
        ext_filters=("png",),
        kind_filters=("image",),
    )
    second_rows = [
        {
            **_attachment_search_table().to_pylist()[0],
            "attachmentId": "attachment-2",
            "attachmentPath": "assets/roadmap.png",
            "attachmentName": "roadmap.png",
            "attachmentExt": "png",
            "kind": "image",
            "score": 0.71,
        }
    ]
    scripted = WendaoArrowScriptedClient()
    scripted.add_attachment_search_response(_attachment_search_table(), request=first_request)
    scripted.add_attachment_search_response(second_rows, request=second_request)
    session = WendaoArrowSession.from_client(scripted)

    first_result = session.attachment_search(first_request)
    second_result = session.attachment_search(second_request)

    first_rows = first_result.parse_attachment_search_rows()
    second_rows = second_result.parse_attachment_search_rows()

    assert isinstance(first_result.request, WendaoAttachmentSearchRequest)
    assert first_result.request == first_request
    assert second_result.request == second_request
    assert first_rows[0].attachment_ext == "pdf"
    assert first_rows[0].source_title == "Architecture Notes"
    assert second_rows[0].attachment_ext == "png"
    assert second_rows[0].kind == "image"
    assert scripted.calls == [
        WendaoArrowCall(
            operation="attachment_search",
            route="/search/attachments",
            request=first_request,
            extra_metadata=attachment_search_metadata(first_request),
            connect_kwargs={},
        ),
        WendaoArrowCall(
            operation="attachment_search",
            route="/search/attachments",
            request=second_request,
            extra_metadata=attachment_search_metadata(second_request),
            connect_kwargs={},
        ),
    ]


def test_rerank_uses_typed_request_builder_and_records_effective_metadata() -> None:
    request_rows = [
        WendaoRerankRequestRow(
            doc_id="doc-1",
            vector_score=0.7,
            embedding=(1.0, 0.0),
            query_embedding=(1.0, 0.0),
        )
    ]
    scripted = WendaoArrowScriptedClient()
    scripted.add_rerank_response(
        _rerank_response_table(),
        request_rows=request_rows,
        top_k=3,
        min_final_score=0.5,
    )
    session = WendaoArrowSession.from_client(scripted)

    result = session.rerank(request_rows, top_k=3, min_final_score=0.5)

    rows = result.parse_rerank_rows()

    assert result.route == "/rerank/flight"
    assert rows[0].final_score == 0.76
    assert rows[0].rank == 1
    assert scripted.calls
    call = scripted.calls[0]
    assert call.operation == "rerank"
    assert call.route == "/rerank/flight"
    assert call.table is not None
    assert call.table.column_names == [
        "doc_id",
        "vector_score",
        "embedding",
        "query_embedding",
    ]
    assert call.extra_metadata == rerank_request_metadata(
        request_rows,
        top_k=3,
        min_final_score=0.5,
    )
    assert call.embedding_dimension == 2
    assert call.top_k == 3
    assert call.min_final_score == 0.5
    assert call.connect_kwargs == {}


def test_result_supports_rows_analyzers_and_optional_polars_adapter() -> None:
    result = WendaoArrowResult(table=_repo_search_table(), route="/search/repos/main")

    class RowsProbe:
        def analyze_rows(self, rows: list[dict[str, object]]) -> dict[str, object]:
            return {"row_count": len(rows), "top_doc_id": rows[0]["doc_id"]}

    analyzed = result.analyze_rows(RowsProbe())
    frame = result.to_polars()

    assert analyzed == {"row_count": 1, "top_doc_id": "doc-1"}
    assert isinstance(frame, pl.DataFrame)
    assert frame.shape == (1, 13)


def test_result_supports_callable_arrow_hooks() -> None:
    result = WendaoArrowResult(table=_repo_search_table(), route="/search/repos/main")

    parsed = result.parse_table(lambda table: {"rows": table.num_rows})

    assert parsed == {"rows": 1}


def test_result_from_rows_builds_one_lightweight_fixture() -> None:
    result = WendaoArrowResult.from_rows(
        [
            {
                "doc_id": "doc-2",
                "path": "docs/graph.md",
                "score": 0.8,
            }
        ],
        route="search/repos/main",
    )

    assert result.route == "/search/repos/main"
    assert result.to_rows() == [
        {
            "doc_id": "doc-2",
            "path": "docs/graph.md",
            "score": 0.8,
        }
    ]


def test_generic_route_scoped_helpers_cover_query_and_exchange_paths() -> None:
    query_session = WendaoArrowSession.for_query_testing(
        "/search/custom/demo", _repo_search_table()
    )
    exchange_session = WendaoArrowSession.for_exchange_testing(
        "/exchange/custom/demo",
        [{"doc_id": "doc-9", "status": "ok"}],
    )

    query_result = query_session.query("/search/custom/demo")
    exchange_result = exchange_session.exchange(
        "/exchange/custom/demo",
        pa.Table.from_pylist([{"seed": "value"}]),
    )

    assert query_result.query is not None
    assert query_result.query.normalized_route() == "/search/custom/demo"
    assert query_result.to_rows()[0]["doc_id"] == "doc-1"
    assert exchange_result.query is not None
    assert exchange_result.query.normalized_route() == "/exchange/custom/demo"
    assert exchange_result.to_rows()[0]["status"] == "ok"


def test_generic_result_fixture_helpers_attach_normalized_queries() -> None:
    query_result = WendaoArrowResult.from_query_rows(
        [{"doc_id": "doc-2", "score": 0.8}],
        route="search/custom/demo",
    )
    exchange_result = WendaoArrowResult.from_exchange_rows(
        [{"doc_id": "doc-3", "status": "ok"}],
        route="exchange/custom/demo",
    )

    assert query_result.route == "/search/custom/demo"
    assert query_result.query is not None
    assert query_result.query.normalized_route() == "/search/custom/demo"
    assert exchange_result.route == "/exchange/custom/demo"
    assert exchange_result.query is not None
    assert exchange_result.query.normalized_route() == "/exchange/custom/demo"


def test_attachment_result_fixture_helper_attaches_normalized_query() -> None:
    result = WendaoArrowResult.from_attachment_search_result_rows(
        _attachment_search_table().to_pylist()
    )

    assert result.route == "/search/attachments"
    assert result.query is not None
    assert result.query.normalized_route() == "/search/attachments"
    assert result.parse_attachment_search_rows()[0].attachment_ext == "pdf"


def test_contract_aware_repo_search_helpers_cover_typed_and_query_paths() -> None:
    session = WendaoArrowSession.for_repo_search_testing(_repo_search_table())

    repo_result = session.repo_search("graph search", limit=5)
    query_result = session.query(REPO_SEARCH_ROUTE)

    assert repo_result.parse_repo_search_rows()[0].doc_id == "doc-1"
    assert query_result.parse_repo_search_rows()[0].doc_id == "doc-1"
    assert isinstance(session.client, WendaoArrowScriptedClient)
    assert [call.operation for call in session.client.calls] == ["repo_search", "query"]


def test_contract_aware_attachment_helpers_cover_typed_and_query_paths() -> None:
    session = WendaoArrowSession.for_attachment_search_testing(_attachment_search_table())

    attachment_result = session.attachment_search(
        "architecture",
        limit=5,
        ext_filters=["pdf"],
        kind_filters=["pdf"],
    )
    query_result = session.query(SEARCH_ATTACHMENTS_ROUTE)

    assert (
        attachment_result.parse_attachment_search_rows()[0].attachment_name == "design-review.pdf"
    )
    assert query_result.parse_attachment_search_rows()[0].attachment_name == "design-review.pdf"
    assert isinstance(session.client, WendaoArrowScriptedClient)
    assert [call.operation for call in session.client.calls] == ["attachment_search", "query"]


def test_contract_aware_rerank_helpers_cover_typed_and_exchange_paths() -> None:
    session = WendaoArrowSession.for_rerank_response_testing(_rerank_response_table())
    request_table = build_rerank_request_table(
        [
            WendaoRerankRequestRow(
                doc_id="doc-1",
                vector_score=0.7,
                embedding=(1.0, 0.0),
                query_embedding=(1.0, 0.0),
            )
        ]
    )

    rerank_result = session.rerank(
        [
            WendaoRerankRequestRow(
                doc_id="doc-1",
                vector_score=0.7,
                embedding=(1.0, 0.0),
                query_embedding=(1.0, 0.0),
            )
        ]
    )
    exchange_result = session.exchange(RERANK_EXCHANGE_ROUTE, request_table)

    assert rerank_result.parse_rerank_rows()[0].doc_id == "doc-1"
    assert exchange_result.parse_rerank_rows()[0].doc_id == "doc-1"
    assert isinstance(session.client, WendaoArrowScriptedClient)
    assert [call.operation for call in session.client.calls] == ["rerank", "exchange"]


def test_contract_aware_result_fixture_helpers_lock_stable_routes() -> None:
    repo_result = WendaoArrowResult.from_repo_search_result_rows(
        [{"doc_id": "doc-2", "path": "docs/cache.md", "score": 0.8}]
    )
    rerank_result = WendaoArrowResult.from_rerank_response_rows(
        [
            {
                "doc_id": "doc-2",
                "vector_score": 0.7,
                "semantic_score": 0.9,
                "final_score": 0.8,
                "rank": 1,
            }
        ]
    )

    assert repo_result.route == REPO_SEARCH_ROUTE
    assert repo_result.query is not None
    assert repo_result.query.normalized_route() == REPO_SEARCH_ROUTE
    assert repo_result.to_rows()[0]["doc_id"] == "doc-2"
    assert rerank_result.route == RERANK_EXCHANGE_ROUTE
    assert rerank_result.query is not None
    assert rerank_result.query.normalized_route() == RERANK_EXCHANGE_ROUTE
    assert rerank_result.parse_rerank_rows()[0].final_score == 0.8


def test_scripted_client_reports_missing_route_clearly() -> None:
    session = WendaoArrowSession.for_testing()

    try:
        session.query("/missing/route")
    except LookupError as error:
        assert "no scripted query response registered" in str(error)
        assert "/missing/route" in str(error)
    else:
        raise AssertionError("expected missing scripted route to raise LookupError")
