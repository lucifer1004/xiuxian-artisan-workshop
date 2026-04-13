"""In-memory testing helpers for Wendao Arrow consumer workflows."""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

import pyarrow as pa
from wendao_core_lib import (
    SEARCH_ATTACHMENTS_ROUTE,
    REPO_SEARCH_ROUTE,
    RERANK_EXCHANGE_ROUTE,
    WENDAO_RERANK_DIMENSION_HEADER,
    WENDAO_RERANK_MIN_FINAL_SCORE_HEADER,
    WENDAO_RERANK_TOP_K_HEADER,
    WendaoAttachmentSearchRequest,
    WendaoFlightRouteQuery,
    WendaoRepoSearchRequest,
    WendaoRerankRequestRow,
    WendaoRerankResultRow,
    attachment_search_metadata,
    build_rerank_request_table,
    parse_rerank_response_rows,
    repo_search_metadata,
    rerank_embedding_dimension,
)

TableLike = pa.Table | Sequence[Mapping[str, object]]


def _coerce_table(table_or_rows: TableLike) -> pa.Table:
    if isinstance(table_or_rows, pa.Table):
        return table_or_rows
    return pa.Table.from_pylist([dict(row) for row in table_or_rows])


def _normalize_route(route: str) -> str:
    return WendaoFlightRouteQuery(route=route).normalized_route()


@dataclass(frozen=True, slots=True)
class WendaoArrowCall:
    """One recorded in-memory Wendao Arrow session call."""

    operation: str
    route: str
    query: WendaoFlightRouteQuery | None = None
    request: WendaoRepoSearchRequest | WendaoAttachmentSearchRequest | None = None
    table: pa.Table | None = None
    extra_metadata: Mapping[str, str] | None = None
    connect_kwargs: Mapping[str, object] | None = None
    embedding_dimension: int | None = None
    top_k: int | None = None
    min_final_score: float | None = None


@dataclass(slots=True)
class _QueuedTypedResponse:
    table: pa.Table
    expected_request: WendaoRepoSearchRequest | WendaoAttachmentSearchRequest | None = None


@dataclass(slots=True)
class _QueuedRerankResponse:
    table: pa.Table
    expected_request_table: pa.Table | None = None
    expected_embedding_dimension: int | None = None
    expected_top_k: int | None = None
    expected_min_final_score: float | None = None


class WendaoArrowScriptedClient:
    """Thin scripted client for downstream tests.

    This helper is intentionally in-memory only. It does not emulate Flight
    transport behavior beyond returning pre-registered Arrow tables and
    recording how the session facade was invoked across generic, repo-search,
    attachment-search, and rerank workflows.
    """

    def __init__(
        self,
        *,
        query_tables: Mapping[str, TableLike] | None = None,
        exchange_tables: Mapping[str, TableLike] | None = None,
        repo_search_table: TableLike | None = None,
        attachment_search_table: TableLike | None = None,
        rerank_table: TableLike | None = None,
    ) -> None:
        self.query_tables = {
            _normalize_route(route): _coerce_table(table)
            for route, table in (query_tables or {}).items()
        }
        self.exchange_tables = {
            _normalize_route(route): _coerce_table(table)
            for route, table in (exchange_tables or {}).items()
        }
        self._repo_search_responses: list[_QueuedTypedResponse] = []
        self._attachment_search_responses: list[_QueuedTypedResponse] = []
        self._rerank_responses: list[_QueuedRerankResponse] = []
        if repo_search_table is not None:
            self.add_repo_search_response(repo_search_table)
        if attachment_search_table is not None:
            self.add_attachment_search_response(attachment_search_table)
        if rerank_table is not None:
            self.add_rerank_response(rerank_table)
        self.calls: list[WendaoArrowCall] = []

    def add_repo_search_response(
        self,
        table_or_rows: TableLike,
        *,
        request: WendaoRepoSearchRequest | None = None,
    ) -> "WendaoArrowScriptedClient":
        """Queue one typed repo-search response, optionally tied to one request."""

        self._repo_search_responses.append(
            _QueuedTypedResponse(
                table=_coerce_table(table_or_rows),
                expected_request=request,
            )
        )
        return self

    def add_attachment_search_response(
        self,
        table_or_rows: TableLike,
        *,
        request: WendaoAttachmentSearchRequest | None = None,
    ) -> "WendaoArrowScriptedClient":
        """Queue one typed attachment-search response, optionally tied to one request."""

        self._attachment_search_responses.append(
            _QueuedTypedResponse(
                table=_coerce_table(table_or_rows),
                expected_request=request,
            )
        )
        return self

    def add_rerank_response(
        self,
        table_or_rows: TableLike,
        *,
        request_rows: Sequence[WendaoRerankRequestRow] | None = None,
        top_k: int | None = None,
        min_final_score: float | None = None,
    ) -> "WendaoArrowScriptedClient":
        """Queue one typed rerank response, optionally tied to one request batch."""

        expected_request_table = None
        expected_embedding_dimension = None
        if request_rows is not None:
            rows = list(request_rows)
            expected_request_table = build_rerank_request_table(rows)
            expected_embedding_dimension = rerank_embedding_dimension(rows)
        self._rerank_responses.append(
            _QueuedRerankResponse(
                table=_coerce_table(table_or_rows),
                expected_request_table=expected_request_table,
                expected_embedding_dimension=expected_embedding_dimension,
                expected_top_k=top_k,
                expected_min_final_score=min_final_score,
            )
        )
        return self

    @classmethod
    def for_repo_search_rows(cls, rows: TableLike) -> "WendaoArrowScriptedClient":
        """Build one scripted client for the stable repo-search response shape.

        The same rows are registered on both the typed repo-search helper path
        and the generic route-backed query path so downstream tests can reuse
        one fixture across the session facade and analyzer runtime.
        """

        return cls(
            query_tables={REPO_SEARCH_ROUTE: rows},
            repo_search_table=rows,
        )

    @classmethod
    def for_attachment_search_rows(cls, rows: TableLike) -> "WendaoArrowScriptedClient":
        """Build one scripted client for the stable attachment-search response shape.

        The same rows are registered on both the typed attachment-search helper
        path and the generic route-backed query path.
        """

        return cls(
            query_tables={SEARCH_ATTACHMENTS_ROUTE: rows},
            attachment_search_table=rows,
        )

    @classmethod
    def for_query_route(cls, route: str, rows: TableLike) -> "WendaoArrowScriptedClient":
        """Build one scripted client for a single generic query route."""

        return cls(query_tables={route: rows})

    @classmethod
    def for_rerank_response_rows(cls, rows: TableLike) -> "WendaoArrowScriptedClient":
        """Build one scripted client for the stable rerank response shape.

        The same rows are registered on both the typed rerank helper path and
        the generic route-backed exchange path.
        """

        return cls(
            exchange_tables={RERANK_EXCHANGE_ROUTE: rows},
            rerank_table=rows,
        )

    @classmethod
    def for_exchange_route(cls, route: str, rows: TableLike) -> "WendaoArrowScriptedClient":
        """Build one scripted client for a single generic exchange route."""

        return cls(exchange_tables={route: rows})

    def read_query_table(
        self,
        query: WendaoFlightRouteQuery,
        *,
        extra_metadata: Mapping[str, str] | None = None,
        **connect_kwargs: object,
    ) -> pa.Table:
        route = query.normalized_route()
        self.calls.append(
            WendaoArrowCall(
                operation="query",
                route=route,
                query=query,
                extra_metadata=dict(extra_metadata or {}),
                connect_kwargs=dict(connect_kwargs),
            )
        )
        return self._require_registered_table(self.query_tables, route, "query")

    def exchange_query_table(
        self,
        query: WendaoFlightRouteQuery,
        table: pa.Table,
        *,
        extra_metadata: Mapping[str, str] | None = None,
        **connect_kwargs: object,
    ) -> pa.Table:
        route = query.normalized_route()
        self.calls.append(
            WendaoArrowCall(
                operation="exchange",
                route=route,
                query=query,
                table=table,
                extra_metadata=dict(extra_metadata or {}),
                connect_kwargs=dict(connect_kwargs),
            )
        )
        return self._require_registered_table(self.exchange_tables, route, "exchange")

    def read_repo_search_table(
        self,
        request: WendaoRepoSearchRequest,
        **connect_kwargs: object,
    ) -> pa.Table:
        extra_metadata = repo_search_metadata(request)
        self.calls.append(
            WendaoArrowCall(
                operation="repo_search",
                route=REPO_SEARCH_ROUTE,
                request=request,
                extra_metadata=extra_metadata,
                connect_kwargs=dict(connect_kwargs),
            )
        )
        return self._dequeue_typed_response(
            self._repo_search_responses,
            operation="repo_search",
            route=REPO_SEARCH_ROUTE,
            request=request,
        )

    def read_attachment_search_table(
        self,
        request: WendaoAttachmentSearchRequest,
        **connect_kwargs: object,
    ) -> pa.Table:
        extra_metadata = attachment_search_metadata(request)
        self.calls.append(
            WendaoArrowCall(
                operation="attachment_search",
                route=SEARCH_ATTACHMENTS_ROUTE,
                request=request,
                extra_metadata=extra_metadata,
                connect_kwargs=dict(connect_kwargs),
            )
        )
        return self._dequeue_typed_response(
            self._attachment_search_responses,
            operation="attachment_search",
            route=SEARCH_ATTACHMENTS_ROUTE,
            request=request,
        )

    def exchange_rerank_table(
        self,
        table: pa.Table,
        *,
        embedding_dimension: int | None = None,
        top_k: int | None = None,
        min_final_score: float | None = None,
        **connect_kwargs: object,
    ) -> pa.Table:
        extra_metadata = _rerank_call_metadata(
            embedding_dimension=embedding_dimension,
            top_k=top_k,
            min_final_score=min_final_score,
        )
        self.calls.append(
            WendaoArrowCall(
                operation="rerank",
                route=RERANK_EXCHANGE_ROUTE,
                table=table,
                extra_metadata=extra_metadata,
                connect_kwargs=dict(connect_kwargs),
                embedding_dimension=embedding_dimension,
                top_k=top_k,
                min_final_score=min_final_score,
            )
        )
        return self._dequeue_rerank_response(
            table,
            embedding_dimension=embedding_dimension,
            top_k=top_k,
            min_final_score=min_final_score,
        )

    def exchange_rerank_result_rows(
        self,
        rows: Sequence[WendaoRerankRequestRow],
        *,
        top_k: int | None = None,
        min_final_score: float | None = None,
        **connect_kwargs: object,
    ) -> list[WendaoRerankResultRow]:
        request_rows = list(rows)
        response = self.exchange_rerank_table(
            build_rerank_request_table(request_rows),
            embedding_dimension=rerank_embedding_dimension(request_rows),
            top_k=top_k,
            min_final_score=min_final_score,
            **connect_kwargs,
        )
        return parse_rerank_response_rows(response)

    @staticmethod
    def _require_registered_table(
        registered_tables: Mapping[str, pa.Table],
        route: str,
        operation: str,
    ) -> pa.Table:
        try:
            return registered_tables[route]
        except KeyError as error:
            raise LookupError(
                f"no scripted {operation} response registered for route {route!r}"
            ) from error

    @staticmethod
    def _dequeue_typed_response(
        queued_responses: list[_QueuedTypedResponse],
        *,
        operation: str,
        route: str,
        request: WendaoRepoSearchRequest | WendaoAttachmentSearchRequest,
    ) -> pa.Table:
        if not queued_responses:
            raise LookupError(f"no scripted {operation} response registered for route {route!r}")
        response = queued_responses[0]
        if response.expected_request is not None and response.expected_request != request:
            raise AssertionError(
                f"scripted {operation} request mismatch for route {route!r}: "
                f"expected {response.expected_request!r}, got {request!r}"
            )
        queued_responses.pop(0)
        return response.table

    def _dequeue_rerank_response(
        self,
        request_table: pa.Table,
        *,
        embedding_dimension: int | None,
        top_k: int | None,
        min_final_score: float | None,
    ) -> pa.Table:
        if not self._rerank_responses:
            raise LookupError(
                f"no scripted rerank response registered for route {RERANK_EXCHANGE_ROUTE!r}"
            )
        response = self._rerank_responses[0]
        if response.expected_request_table is not None and not _tables_equal(
            response.expected_request_table,
            request_table,
        ):
            raise AssertionError(
                f"scripted rerank request table mismatch for route {RERANK_EXCHANGE_ROUTE!r}"
            )
        if (
            response.expected_embedding_dimension is not None
            and response.expected_embedding_dimension != embedding_dimension
        ):
            raise AssertionError(
                "scripted rerank embedding dimension mismatch for route "
                f"{RERANK_EXCHANGE_ROUTE!r}: expected {response.expected_embedding_dimension!r}, "
                f"got {embedding_dimension!r}"
            )
        if response.expected_top_k is not None and response.expected_top_k != top_k:
            raise AssertionError(
                f"scripted rerank top_k mismatch for route {RERANK_EXCHANGE_ROUTE!r}: "
                f"expected {response.expected_top_k!r}, got {top_k!r}"
            )
        if (
            response.expected_min_final_score is not None
            and response.expected_min_final_score != min_final_score
        ):
            raise AssertionError(
                "scripted rerank min_final_score mismatch for route "
                f"{RERANK_EXCHANGE_ROUTE!r}: expected "
                f"{response.expected_min_final_score!r}, got {min_final_score!r}"
            )
        self._rerank_responses.pop(0)
        return response.table


def _rerank_call_metadata(
    *,
    embedding_dimension: int | None,
    top_k: int | None,
    min_final_score: float | None,
) -> dict[str, str]:
    metadata: dict[str, str] = {}
    if embedding_dimension is not None:
        metadata[WENDAO_RERANK_DIMENSION_HEADER] = str(embedding_dimension)
    if top_k is not None:
        metadata[WENDAO_RERANK_TOP_K_HEADER] = str(top_k)
    if min_final_score is not None:
        metadata[WENDAO_RERANK_MIN_FINAL_SCORE_HEADER] = str(min_final_score)
    return metadata


def _tables_equal(left: pa.Table, right: pa.Table) -> bool:
    return (
        left.schema.equals(right.schema, check_metadata=False)
        and left.to_pylist() == right.to_pylist()
    )


__all__ = ["TableLike", "WendaoArrowCall", "WendaoArrowScriptedClient"]
