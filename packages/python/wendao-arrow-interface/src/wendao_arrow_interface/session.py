"""Session facade for Wendao Arrow Flight consumer workflows."""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import Protocol

import pyarrow as pa
from wendao_core_lib import (
    ATTACHMENT_SEARCH_DEFAULT_LIMIT,
    REPO_SEARCH_ROUTE,
    RERANK_EXCHANGE_ROUTE,
    SEARCH_ATTACHMENTS_ROUTE,
    WendaoAttachmentSearchRequest,
    WendaoFlightRouteQuery,
    WendaoRepoSearchRequest,
    WendaoRerankRequestRow,
    WendaoTransportClient,
    WendaoTransportConfig,
    WendaoTransportEndpoint,
    attachment_search_query,
    attachment_search_request,
    build_rerank_request_table,
    repo_search_query,
    repo_search_request,
    rerank_embedding_dimension,
    rerank_exchange_query,
)

from .result import WendaoArrowResult


class WendaoArrowSessionClient(Protocol):
    """Protocol for live or scripted clients consumed by the session facade."""

    def read_query_table(
        self,
        query: WendaoFlightRouteQuery,
        *,
        extra_metadata: Mapping[str, str] | None = None,
        **connect_kwargs: object,
    ) -> pa.Table: ...

    def exchange_query_table(
        self,
        query: WendaoFlightRouteQuery,
        table: pa.Table,
        *,
        extra_metadata: Mapping[str, str] | None = None,
        **connect_kwargs: object,
    ) -> pa.Table: ...

    def read_repo_search_table(
        self,
        request: WendaoRepoSearchRequest,
        **connect_kwargs: object,
    ) -> pa.Table: ...

    def read_attachment_search_table(
        self,
        request: WendaoAttachmentSearchRequest,
        **connect_kwargs: object,
    ) -> pa.Table: ...

    def exchange_rerank_table(
        self,
        table: pa.Table,
        *,
        embedding_dimension: int | None = None,
        top_k: int | None = None,
        min_final_score: float | None = None,
        **connect_kwargs: object,
    ) -> pa.Table: ...


@dataclass(frozen=True, slots=True)
class WendaoArrowSession:
    """Downstream-facing session wrapper over ``WendaoTransportClient``."""

    client: WendaoArrowSessionClient

    @staticmethod
    def _query_from_route_input(
        route_or_query: str | WendaoFlightRouteQuery,
        *,
        ticket: str | bytes | None = None,
    ) -> WendaoFlightRouteQuery:
        if isinstance(route_or_query, WendaoFlightRouteQuery):
            if ticket is not None:
                raise ValueError(
                    "ticket must not be passed separately when route_or_query is a "
                    "WendaoFlightRouteQuery"
                )
            return route_or_query
        return WendaoFlightRouteQuery(route=route_or_query, ticket=ticket)

    @classmethod
    def from_endpoint(
        cls,
        *,
        host: str,
        port: int,
        path: str = "/",
        tls: bool = False,
        metadata: Mapping[str, str] | None = None,
        schema_version: str = "v2",
        request_timeout_seconds: float = 30.0,
        prefer_arrow_ipc_fallback: bool = True,
        allow_embedded: bool = False,
    ) -> "WendaoArrowSession":
        """Build one session from Wendao transport endpoint settings."""

        endpoint = WendaoTransportEndpoint(
            host=host,
            port=port,
            path=path,
            tls=tls,
            metadata=dict(metadata or {}),
        )
        config = WendaoTransportConfig(
            endpoint=endpoint,
            schema_version=schema_version,
            request_timeout_seconds=request_timeout_seconds,
            allow_embedded=allow_embedded,
            prefer_arrow_ipc_fallback=prefer_arrow_ipc_fallback,
        )
        return cls(client=WendaoTransportClient(config))

    @classmethod
    def from_client(cls, client: WendaoArrowSessionClient) -> "WendaoArrowSession":
        """Build one session from an injected live or scripted client."""

        return cls(client=client)

    @classmethod
    def for_testing(
        cls,
        *,
        query_tables: Mapping[str, pa.Table | Sequence[Mapping[str, object]]] | None = None,
        exchange_tables: Mapping[str, pa.Table | Sequence[Mapping[str, object]]] | None = None,
        repo_search_table: pa.Table | Sequence[Mapping[str, object]] | None = None,
        attachment_search_table: pa.Table | Sequence[Mapping[str, object]] | None = None,
        rerank_table: pa.Table | Sequence[Mapping[str, object]] | None = None,
    ) -> "WendaoArrowSession":
        """Build one in-memory session for downstream tests."""

        from .testing import WendaoArrowScriptedClient

        return cls.from_client(
            WendaoArrowScriptedClient(
                query_tables=query_tables,
                exchange_tables=exchange_tables,
                repo_search_table=repo_search_table,
                attachment_search_table=attachment_search_table,
                rerank_table=rerank_table,
            )
        )

    @classmethod
    def for_repo_search_testing(
        cls,
        rows: pa.Table | Sequence[Mapping[str, object]],
        *,
        request: WendaoRepoSearchRequest | None = None,
        extra_metadata: Mapping[str, str] | None = None,
    ) -> "WendaoArrowSession":
        """Build one scripted session for the stable repo-search workflow."""

        from .testing import WendaoArrowScriptedClient

        return cls.from_client(
            WendaoArrowScriptedClient.for_repo_search_rows(
                rows,
                request=request,
                extra_metadata=extra_metadata,
            )
        )

    @classmethod
    def for_attachment_search_testing(
        cls,
        rows: pa.Table | Sequence[Mapping[str, object]],
        *,
        request: WendaoAttachmentSearchRequest | None = None,
        extra_metadata: Mapping[str, str] | None = None,
    ) -> "WendaoArrowSession":
        """Build one scripted session for the stable attachment-search workflow."""

        from .testing import WendaoArrowScriptedClient

        return cls.from_client(
            WendaoArrowScriptedClient.for_attachment_search_rows(
                rows,
                request=request,
                extra_metadata=extra_metadata,
            )
        )

    @classmethod
    def for_query_testing(
        cls,
        route: str,
        rows: pa.Table | Sequence[Mapping[str, object]],
        *,
        ticket: str | bytes | None = None,
        extra_metadata: Mapping[str, str] | None = None,
    ) -> "WendaoArrowSession":
        """Build one scripted session for a single generic query route."""

        from .testing import WendaoArrowScriptedClient

        return cls.from_client(
            WendaoArrowScriptedClient().add_query_response(
                route,
                rows,
                ticket=ticket,
                extra_metadata=extra_metadata,
            )
        )

    @classmethod
    def for_rerank_response_testing(
        cls,
        rows: pa.Table | Sequence[Mapping[str, object]],
        *,
        request_rows: Sequence[WendaoRerankRequestRow] | None = None,
        top_k: int | None = None,
        min_final_score: float | None = None,
        extra_metadata: Mapping[str, str] | None = None,
    ) -> "WendaoArrowSession":
        """Build one scripted session for the stable rerank workflow."""

        from .testing import WendaoArrowScriptedClient

        return cls.from_client(
            WendaoArrowScriptedClient.for_rerank_response_rows(
                rows,
                request_rows=request_rows,
                top_k=top_k,
                min_final_score=min_final_score,
                extra_metadata=extra_metadata,
            )
        )

    @classmethod
    def for_exchange_testing(
        cls,
        route: str,
        rows: pa.Table | Sequence[Mapping[str, object]],
        *,
        ticket: str | bytes | None = None,
        extra_metadata: Mapping[str, str] | None = None,
        request_table: pa.Table | Sequence[Mapping[str, object]] | None = None,
    ) -> "WendaoArrowSession":
        """Build one scripted session for a single generic exchange route."""

        from .testing import WendaoArrowScriptedClient

        return cls.from_client(
            WendaoArrowScriptedClient().add_exchange_response(
                route,
                rows,
                ticket=ticket,
                extra_metadata=extra_metadata,
                request_table=request_table,
            )
        )

    def query(
        self,
        route_or_query: str | WendaoFlightRouteQuery,
        *,
        ticket: str | bytes | None = None,
        extra_metadata: Mapping[str, str] | None = None,
        **connect_kwargs: object,
    ) -> WendaoArrowResult:
        """Read one query result table from a typed Wendao Flight route."""

        query = self._query_from_route_input(route_or_query, ticket=ticket)
        table = self.client.read_query_table(
            query,
            extra_metadata=extra_metadata,
            **connect_kwargs,
        )
        return WendaoArrowResult(
            table=table,
            route=query.normalized_route(),
            query=query,
        )

    def exchange(
        self,
        route_or_query: str | WendaoFlightRouteQuery,
        table: pa.Table,
        *,
        ticket: str | bytes | None = None,
        extra_metadata: Mapping[str, str] | None = None,
        **connect_kwargs: object,
    ) -> WendaoArrowResult:
        """Exchange one Arrow table through a typed Wendao Flight route."""

        query = self._query_from_route_input(route_or_query, ticket=ticket)
        response = self.client.exchange_query_table(
            query,
            table,
            extra_metadata=extra_metadata,
            **connect_kwargs,
        )
        return WendaoArrowResult(
            table=response,
            route=query.normalized_route(),
            query=query,
        )

    def repo_search(
        self,
        request_or_query: WendaoRepoSearchRequest | str,
        *,
        limit: int = 10,
        language_filters: tuple[str, ...] | list[str] = (),
        path_prefixes: tuple[str, ...] | list[str] = (),
        title_filters: tuple[str, ...] | list[str] = (),
        tag_filters: tuple[str, ...] | list[str] = (),
        filename_filters: tuple[str, ...] | list[str] = (),
        **connect_kwargs: object,
    ) -> WendaoArrowResult:
        """Read the stable repo-search result table."""

        request = (
            request_or_query
            if isinstance(request_or_query, WendaoRepoSearchRequest)
            else repo_search_request(
                request_or_query,
                limit=limit,
                language_filters=language_filters,
                path_prefixes=path_prefixes,
                title_filters=title_filters,
                tag_filters=tag_filters,
                filename_filters=filename_filters,
            )
        )
        table = self.client.read_repo_search_table(request, **connect_kwargs)
        return WendaoArrowResult(
            table=table,
            route=REPO_SEARCH_ROUTE,
            query=repo_search_query(),
            request=request,
        )

    def attachment_search(
        self,
        request_or_query: WendaoAttachmentSearchRequest | str,
        *,
        limit: int = ATTACHMENT_SEARCH_DEFAULT_LIMIT,
        ext_filters: tuple[str, ...] | list[str] = (),
        kind_filters: tuple[str, ...] | list[str] = (),
        case_sensitive: bool = False,
        **connect_kwargs: object,
    ) -> WendaoArrowResult:
        """Read the stable attachment-search result table."""

        request = (
            request_or_query
            if isinstance(request_or_query, WendaoAttachmentSearchRequest)
            else attachment_search_request(
                request_or_query,
                limit=limit,
                ext_filters=ext_filters,
                kind_filters=kind_filters,
                case_sensitive=case_sensitive,
            )
        )
        table = self.client.read_attachment_search_table(request, **connect_kwargs)
        return WendaoArrowResult(
            table=table,
            route=SEARCH_ATTACHMENTS_ROUTE,
            query=attachment_search_query(),
            request=request,
        )

    def rerank(
        self,
        rows: Sequence[WendaoRerankRequestRow],
        *,
        top_k: int | None = None,
        min_final_score: float | None = None,
        **connect_kwargs: object,
    ) -> WendaoArrowResult:
        """Exchange one typed rerank request batch through the stable route."""

        request_rows = list(rows)
        response = self.client.exchange_rerank_table(
            build_rerank_request_table(request_rows),
            embedding_dimension=rerank_embedding_dimension(request_rows),
            top_k=top_k,
            min_final_score=min_final_score,
            **connect_kwargs,
        )
        return WendaoArrowResult(
            table=response,
            route=RERANK_EXCHANGE_ROUTE,
            query=rerank_exchange_query(),
        )


def connect(**kwargs: object) -> WendaoArrowSession:
    """Build one session from endpoint settings."""

    return WendaoArrowSession.from_endpoint(**kwargs)


__all__ = ["WendaoArrowSession", "connect"]
