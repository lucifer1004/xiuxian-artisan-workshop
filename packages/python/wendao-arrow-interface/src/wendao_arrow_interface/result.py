"""Result facade for one Wendao Arrow query or exchange."""

from __future__ import annotations

from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, TypeVar, cast

import pyarrow as pa
from wendao_core_lib import (
    SEARCH_ATTACHMENTS_ROUTE,
    REPO_SEARCH_ROUTE,
    RERANK_EXCHANGE_ROUTE,
    WendaoAttachmentSearchRequest,
    WendaoAttachmentSearchResultRow,
    WendaoFlightRouteQuery,
    WendaoRepoSearchRequest,
    WendaoRepoSearchResultRow,
    WendaoRerankResultRow,
    attachment_search_query,
    parse_attachment_search_rows,
    parse_repo_search_rows,
    parse_rerank_response_rows,
    repo_search_query,
    rerank_exchange_query,
)

from .protocols import (
    ArrowTableAnalyzer,
    ArrowTableParser,
    RowsAnalyzer,
)

if TYPE_CHECKING:
    import polars as pl

ParsedT = TypeVar("ParsedT")
AnalyzedT = TypeVar("AnalyzedT")


def _require_polars():
    try:
        import polars as pl
    except ModuleNotFoundError as error:
        raise ModuleNotFoundError(
            "polars is optional for wendao-arrow-interface; install the 'polars' extra "
            "or add polars to the environment before calling to_polars()"
        ) from error
    return pl


@dataclass(frozen=True, slots=True)
class WendaoArrowResult:
    """One downstream-facing result wrapper around a raw Arrow table."""

    table: pa.Table
    route: str
    query: WendaoFlightRouteQuery | None = None
    request: WendaoRepoSearchRequest | WendaoAttachmentSearchRequest | None = None

    @classmethod
    def from_rows(
        cls,
        rows: Sequence[Mapping[str, object]],
        *,
        route: str,
        query: WendaoFlightRouteQuery | None = None,
        request: WendaoRepoSearchRequest | WendaoAttachmentSearchRequest | None = None,
        schema: pa.Schema | None = None,
    ) -> "WendaoArrowResult":
        """Build one result fixture from Python row dictionaries."""

        table = pa.Table.from_pylist([dict(row) for row in rows], schema=schema)
        normalized_route = WendaoFlightRouteQuery(route=route).normalized_route()
        return cls(table=table, route=normalized_route, query=query, request=request)

    @classmethod
    def from_query_rows(
        cls,
        rows: Sequence[Mapping[str, object]],
        *,
        route: str,
        ticket: str | bytes | None = None,
        schema: pa.Schema | None = None,
    ) -> "WendaoArrowResult":
        """Build one generic query result fixture with a normalized route query."""

        query = WendaoFlightRouteQuery(route=route, ticket=ticket)
        return cls.from_rows(
            rows,
            route=route,
            query=query,
            schema=schema,
        )

    @classmethod
    def from_exchange_rows(
        cls,
        rows: Sequence[Mapping[str, object]],
        *,
        route: str,
        ticket: str | bytes | None = None,
        schema: pa.Schema | None = None,
    ) -> "WendaoArrowResult":
        """Build one generic exchange result fixture with a normalized route query."""

        query = WendaoFlightRouteQuery(route=route, ticket=ticket)
        return cls.from_rows(
            rows,
            route=route,
            query=query,
            schema=schema,
        )

    @classmethod
    def from_repo_search_result_rows(
        cls,
        rows: Sequence[Mapping[str, object]],
        *,
        request: WendaoRepoSearchRequest | None = None,
        schema: pa.Schema | None = None,
    ) -> "WendaoArrowResult":
        """Build one repo-search result fixture without repeating route wiring."""

        return cls.from_rows(
            rows,
            route=REPO_SEARCH_ROUTE,
            query=repo_search_query(),
            request=request,
            schema=schema,
        )

    @classmethod
    def from_attachment_search_result_rows(
        cls,
        rows: Sequence[Mapping[str, object]],
        *,
        request: WendaoAttachmentSearchRequest | None = None,
        schema: pa.Schema | None = None,
    ) -> "WendaoArrowResult":
        """Build one attachment-search result fixture without repeating route wiring."""

        return cls.from_rows(
            rows,
            route=SEARCH_ATTACHMENTS_ROUTE,
            query=attachment_search_query(),
            request=request,
            schema=schema,
        )

    @classmethod
    def from_rerank_response_rows(
        cls,
        rows: Sequence[Mapping[str, object]],
        *,
        schema: pa.Schema | None = None,
    ) -> "WendaoArrowResult":
        """Build one rerank response fixture without repeating route wiring."""

        return cls.from_rows(
            rows,
            route=RERANK_EXCHANGE_ROUTE,
            query=rerank_exchange_query(),
            schema=schema,
        )

    def to_rows(self) -> list[dict[str, object]]:
        """Return the result table as Python row dictionaries."""

        return cast(list[dict[str, object]], self.table.to_pylist())

    def to_polars(self) -> "pl.DataFrame":
        """Materialize the result table as one optional Polars dataframe adapter."""

        pl = _require_polars()
        return pl.from_arrow(self.table)

    def parse_table(
        self,
        parser: ArrowTableParser[ParsedT] | Callable[[pa.Table], ParsedT],
    ) -> ParsedT:
        """Run one Arrow-table parser against the result."""

        parse_table = getattr(parser, "parse_table", None)
        if callable(parse_table):
            return parse_table(self.table)
        if callable(parser):
            return parser(self.table)
        raise TypeError("parser must be callable or define parse_table(table)")

    def analyze_rows(
        self,
        analyzer: RowsAnalyzer[AnalyzedT] | Callable[[list[dict[str, object]]], AnalyzedT],
    ) -> AnalyzedT:
        """Run one rows analyzer against the result."""

        rows = self.to_rows()
        analyze_rows = getattr(analyzer, "analyze_rows", None)
        if callable(analyze_rows):
            return analyze_rows(rows)
        if callable(analyzer):
            return analyzer(rows)
        raise TypeError("analyzer must be callable or define analyze_rows(rows)")

    def analyze_table(
        self,
        analyzer: ArrowTableAnalyzer[AnalyzedT] | Callable[[pa.Table], AnalyzedT],
    ) -> AnalyzedT:
        """Run one Arrow-table analyzer against the result."""

        analyze_table = getattr(analyzer, "analyze_table", None)
        if callable(analyze_table):
            return analyze_table(self.table)
        if callable(analyzer):
            return analyzer(self.table)
        raise TypeError("analyzer must be callable or define analyze_table(table)")

    def parse_repo_search_rows(self) -> list[WendaoRepoSearchResultRow]:
        """Parse the stable repo-search contract from the wrapped table."""

        return parse_repo_search_rows(self.table)

    def parse_attachment_search_rows(self) -> list[WendaoAttachmentSearchResultRow]:
        """Parse the stable attachment-search contract from the wrapped table."""

        return parse_attachment_search_rows(self.table)

    def parse_rerank_rows(self) -> list[WendaoRerankResultRow]:
        """Parse the stable rerank response contract from the wrapped table."""

        return parse_rerank_response_rows(self.table)


__all__ = ["WendaoArrowResult"]
