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

    @property
    def effective_metadata(self) -> dict[str, str]:
        """Return the recorded effective Flight metadata for this call."""

        return dict(self.extra_metadata or {})

    def derived_metadata(self) -> dict[str, str]:
        """Derive the expected effective metadata from the call contract."""

        if self.operation == "repo_search":
            if self.request is None:
                raise ValueError("repo_search calls require one request to derive metadata")
            return repo_search_metadata(self.request)
        if self.operation == "attachment_search":
            if self.request is None:
                raise ValueError("attachment_search calls require one request to derive metadata")
            return attachment_search_metadata(self.request)
        if self.operation == "rerank":
            return _rerank_call_metadata(
                embedding_dimension=self.embedding_dimension,
                top_k=self.top_k,
                min_final_score=self.min_final_score,
            )
        return self.effective_metadata

    def metadata_matches_contract(self) -> bool:
        """Return whether the recorded metadata matches the call contract."""

        return self.effective_metadata == self.derived_metadata()

    def assert_metadata_matches_contract(self) -> None:
        """Raise when the recorded metadata diverges from the call contract."""

        expected = self.derived_metadata()
        actual = self.effective_metadata
        if actual != expected:
            raise AssertionError(
                f"recorded metadata mismatch for {self.operation!r} on route {self.route!r}: "
                f"expected {expected!r}, got {actual!r}"
            )


@dataclass(slots=True)
class _QueuedTypedResponse:
    table: pa.Table
    expected_request: WendaoRepoSearchRequest | WendaoAttachmentSearchRequest | None = None
    expected_extra_metadata: Mapping[str, str] | None = None


@dataclass(slots=True)
class _QueuedGenericResponse:
    route: str
    table: pa.Table
    expected_ticket: str | bytes | None = None
    expected_extra_metadata: Mapping[str, str] | None = None
    expected_request_table: pa.Table | None = None


@dataclass(slots=True)
class _QueuedRerankResponse:
    table: pa.Table
    expected_request_table: pa.Table | None = None
    expected_embedding_dimension: int | None = None
    expected_top_k: int | None = None
    expected_min_final_score: float | None = None
    expected_extra_metadata: Mapping[str, str] | None = None


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
        self.query_tables = {}
        self.exchange_tables = {}
        self._query_responses: list[_QueuedGenericResponse] = []
        self._exchange_responses: list[_QueuedGenericResponse] = []
        for route, table in (query_tables or {}).items():
            self.add_query_response(route, table)
        for route, table in (exchange_tables or {}).items():
            self.add_exchange_response(route, table)
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
        extra_metadata: Mapping[str, str] | None = None,
    ) -> "WendaoArrowScriptedClient":
        """Queue one typed repo-search response, optionally tied to one request."""

        expected_extra_metadata = (
            dict(extra_metadata)
            if extra_metadata is not None
            else None
            if request is None
            else repo_search_metadata(request)
        )
        self._repo_search_responses.append(
            _QueuedTypedResponse(
                table=_coerce_table(table_or_rows),
                expected_request=request,
                expected_extra_metadata=expected_extra_metadata,
            )
        )
        return self

    def add_attachment_search_response(
        self,
        table_or_rows: TableLike,
        *,
        request: WendaoAttachmentSearchRequest | None = None,
        extra_metadata: Mapping[str, str] | None = None,
    ) -> "WendaoArrowScriptedClient":
        """Queue one typed attachment-search response, optionally tied to one request."""

        expected_extra_metadata = (
            dict(extra_metadata)
            if extra_metadata is not None
            else None
            if request is None
            else attachment_search_metadata(request)
        )
        self._attachment_search_responses.append(
            _QueuedTypedResponse(
                table=_coerce_table(table_or_rows),
                expected_request=request,
                expected_extra_metadata=expected_extra_metadata,
            )
        )
        return self

    def add_query_response(
        self,
        route: str,
        table_or_rows: TableLike,
        *,
        ticket: str | bytes | None = None,
        extra_metadata: Mapping[str, str] | None = None,
    ) -> "WendaoArrowScriptedClient":
        """Queue one generic query response, optionally tied to ticket or metadata."""

        normalized_route = _normalize_route(route)
        table = _coerce_table(table_or_rows)
        self.query_tables[normalized_route] = table
        self._query_responses.append(
            _QueuedGenericResponse(
                route=normalized_route,
                table=table,
                expected_ticket=ticket,
                expected_extra_metadata=None if extra_metadata is None else dict(extra_metadata),
            )
        )
        return self

    def add_exchange_response(
        self,
        route: str,
        table_or_rows: TableLike,
        *,
        ticket: str | bytes | None = None,
        extra_metadata: Mapping[str, str] | None = None,
        request_table: pa.Table | Sequence[Mapping[str, object]] | None = None,
    ) -> "WendaoArrowScriptedClient":
        """Queue one generic exchange response, optionally tied to request details."""

        normalized_route = _normalize_route(route)
        table = _coerce_table(table_or_rows)
        self.exchange_tables[normalized_route] = table
        self._exchange_responses.append(
            _QueuedGenericResponse(
                route=normalized_route,
                table=table,
                expected_ticket=ticket,
                expected_extra_metadata=None if extra_metadata is None else dict(extra_metadata),
                expected_request_table=(
                    None if request_table is None else _coerce_table(request_table)
                ),
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
        extra_metadata: Mapping[str, str] | None = None,
    ) -> "WendaoArrowScriptedClient":
        """Queue one typed rerank response, optionally tied to one request batch."""

        expected_request_table = None
        expected_embedding_dimension = None
        if request_rows is not None:
            rows = list(request_rows)
            expected_request_table = build_rerank_request_table(rows)
            expected_embedding_dimension = rerank_embedding_dimension(rows)
        expected_extra_metadata = (
            dict(extra_metadata)
            if extra_metadata is not None
            else _rerank_call_metadata(
                embedding_dimension=expected_embedding_dimension,
                top_k=top_k,
                min_final_score=min_final_score,
            )
            if (request_rows is not None or top_k is not None or min_final_score is not None)
            else None
        )
        self._rerank_responses.append(
            _QueuedRerankResponse(
                table=_coerce_table(table_or_rows),
                expected_request_table=expected_request_table,
                expected_embedding_dimension=expected_embedding_dimension,
                expected_top_k=top_k,
                expected_min_final_score=min_final_score,
                expected_extra_metadata=expected_extra_metadata,
            )
        )
        return self

    @classmethod
    def for_repo_search_rows(
        cls,
        rows: TableLike,
        *,
        request: WendaoRepoSearchRequest | None = None,
        extra_metadata: Mapping[str, str] | None = None,
    ) -> "WendaoArrowScriptedClient":
        """Build one scripted client for the stable repo-search response shape.

        The same rows are registered on both the typed repo-search helper path
        and the generic route-backed query path so downstream tests can reuse
        one fixture across the session facade and analyzer runtime.
        """

        return cls(query_tables={REPO_SEARCH_ROUTE: rows}).add_repo_search_response(
            rows,
            request=request,
            extra_metadata=extra_metadata,
        )

    @classmethod
    def for_attachment_search_rows(
        cls,
        rows: TableLike,
        *,
        request: WendaoAttachmentSearchRequest | None = None,
        extra_metadata: Mapping[str, str] | None = None,
    ) -> "WendaoArrowScriptedClient":
        """Build one scripted client for the stable attachment-search response shape.

        The same rows are registered on both the typed attachment-search helper
        path and the generic route-backed query path.
        """

        return cls(query_tables={SEARCH_ATTACHMENTS_ROUTE: rows}).add_attachment_search_response(
            rows,
            request=request,
            extra_metadata=extra_metadata,
        )

    @classmethod
    def for_query_route(cls, route: str, rows: TableLike) -> "WendaoArrowScriptedClient":
        """Build one scripted client for a single generic query route."""

        return cls().add_query_response(route, rows)

    @classmethod
    def for_rerank_response_rows(
        cls,
        rows: TableLike,
        *,
        request_rows: Sequence[WendaoRerankRequestRow] | None = None,
        top_k: int | None = None,
        min_final_score: float | None = None,
        extra_metadata: Mapping[str, str] | None = None,
    ) -> "WendaoArrowScriptedClient":
        """Build one scripted client for the stable rerank response shape.

        The same rows are registered on both the typed rerank helper path and
        the generic route-backed exchange path.
        """

        return cls(exchange_tables={RERANK_EXCHANGE_ROUTE: rows}).add_rerank_response(
            rows,
            request_rows=request_rows,
            top_k=top_k,
            min_final_score=min_final_score,
            extra_metadata=extra_metadata,
        )

    @classmethod
    def for_exchange_route(cls, route: str, rows: TableLike) -> "WendaoArrowScriptedClient":
        """Build one scripted client for a single generic exchange route."""

        return cls().add_exchange_response(route, rows)

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
        return self._dequeue_generic_response(
            self._query_responses,
            operation="query",
            route=route,
            ticket=query.ticket,
            extra_metadata=dict(extra_metadata or {}),
        )

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
        return self._dequeue_generic_response(
            self._exchange_responses,
            operation="exchange",
            route=route,
            ticket=query.ticket,
            extra_metadata=dict(extra_metadata or {}),
            request_table=table,
        )

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
            extra_metadata=extra_metadata,
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
            extra_metadata=extra_metadata,
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
            extra_metadata=extra_metadata,
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
        extra_metadata: Mapping[str, str],
    ) -> pa.Table:
        if not queued_responses:
            raise LookupError(f"no scripted {operation} response registered for route {route!r}")
        response = queued_responses[0]
        if response.expected_request is not None and response.expected_request != request:
            raise AssertionError(
                f"scripted {operation} request mismatch for route {route!r}: "
                f"expected {response.expected_request!r}, got {request!r}"
            )
        if response.expected_extra_metadata is not None and dict(
            response.expected_extra_metadata
        ) != dict(extra_metadata):
            raise AssertionError(
                f"scripted {operation} metadata mismatch for route {route!r}: "
                f"expected {dict(response.expected_extra_metadata)!r}, got {dict(extra_metadata)!r}"
            )
        queued_responses.pop(0)
        return response.table

    @staticmethod
    def _dequeue_generic_response(
        queued_responses: list[_QueuedGenericResponse],
        *,
        operation: str,
        route: str,
        ticket: str | bytes | None,
        extra_metadata: Mapping[str, str],
        request_table: pa.Table | None = None,
    ) -> pa.Table:
        if not queued_responses:
            raise LookupError(f"no scripted {operation} response registered for route {route!r}")
        response = queued_responses[0]
        if response.route != route:
            raise AssertionError(
                f"scripted {operation} route mismatch: expected {response.route!r}, got {route!r}"
            )
        if response.expected_ticket is not None and response.expected_ticket != ticket:
            raise AssertionError(
                f"scripted {operation} ticket mismatch for route {route!r}: "
                f"expected {response.expected_ticket!r}, got {ticket!r}"
            )
        if response.expected_extra_metadata is not None and dict(
            response.expected_extra_metadata
        ) != dict(extra_metadata):
            raise AssertionError(
                f"scripted {operation} metadata mismatch for route {route!r}: "
                f"expected {dict(response.expected_extra_metadata)!r}, got {dict(extra_metadata)!r}"
            )
        if response.expected_request_table is not None:
            if request_table is None or not _tables_equal(
                response.expected_request_table, request_table
            ):
                raise AssertionError(
                    f"scripted {operation} request table mismatch for route {route!r}"
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
        extra_metadata: Mapping[str, str],
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
        if response.expected_extra_metadata is not None and dict(
            response.expected_extra_metadata
        ) != dict(extra_metadata):
            raise AssertionError(
                f"scripted rerank metadata mismatch for route {RERANK_EXCHANGE_ROUTE!r}: "
                f"expected {dict(response.expected_extra_metadata)!r}, got {dict(extra_metadata)!r}"
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
