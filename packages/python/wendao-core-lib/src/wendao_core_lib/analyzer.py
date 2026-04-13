"""Analyzer authoring helpers for Arrow-backed Wendao Python plugins."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Protocol, TypeVar

from .transport import WendaoFlightRouteQuery, WendaoTransportClient

if TYPE_CHECKING:
    import pyarrow as pa


@dataclass(frozen=True, slots=True)
class WendaoAnalyzerContext:
    """Runtime context passed to one analyzer invocation."""

    client: WendaoTransportClient | None
    query: WendaoFlightRouteQuery
    flight_info: object | None = None


ResultT = TypeVar("ResultT")


class WendaoAnalyzer(Protocol[ResultT]):
    """Callable protocol for one Arrow-backed analyzer."""

    def __call__(
        self,
        table: "pa.Table",
        context: WendaoAnalyzerContext,
    ) -> ResultT: ...


def build_mock_flight_info(
    query: WendaoFlightRouteQuery,
    rows: list[dict[str, object]] | None = None,
) -> dict[str, object]:
    """Build one stable mock Flight-info payload for local analyzer replay."""

    return {
        "route": query.normalized_route(),
        "descriptor_path": query.descriptor_segments(),
        "ticket": query.effective_ticket(),
        "row_count": 0 if rows is None else len(rows),
        "mode": "mock_flight",
    }


def run_analyzer(
    client: WendaoTransportClient,
    analyzer: WendaoAnalyzer[ResultT],
    query: WendaoFlightRouteQuery,
    *,
    include_flight_info: bool = True,
    **connect_kwargs: object,
) -> ResultT:
    """Fetch one Arrow table and invoke one analyzer callable.

    This keeps the Python plugin surface narrow: downstream analyzers only need
    to implement Arrow-table logic while the package owns Flight descriptor,
    metadata, and table-fetch plumbing.
    """

    flight_info = client.get_query_info(query, **connect_kwargs) if include_flight_info else None
    table = client.read_query_table(query, **connect_kwargs)
    context = WendaoAnalyzerContext(
        client=client,
        query=query,
        flight_info=flight_info,
    )
    return analyzer(table, context)


def run_analyzer_with_table(
    analyzer: WendaoAnalyzer[ResultT],
    table: "pa.Table",
    query: WendaoFlightRouteQuery,
    *,
    client: WendaoTransportClient | None = None,
    flight_info: object | None = None,
) -> ResultT:
    """Invoke one analyzer against a prebuilt Arrow table."""

    context = WendaoAnalyzerContext(
        client=client,
        query=query,
        flight_info=flight_info,
    )
    return analyzer(table, context)


def run_analyzer_with_rows(
    analyzer: WendaoAnalyzer[ResultT],
    rows: list[dict[str, object]],
    query: WendaoFlightRouteQuery,
    *,
    client: WendaoTransportClient | None = None,
    flight_info: object | None = None,
) -> ResultT:
    """Invoke one analyzer against a local row replay payload."""

    import pyarrow as pa

    return run_analyzer_with_table(
        analyzer,
        pa.Table.from_pylist(rows),
        query,
        client=client,
        flight_info=flight_info,
    )


def run_analyzer_with_mock_rows(
    analyzer: WendaoAnalyzer[ResultT],
    rows: list[dict[str, object]],
    query: WendaoFlightRouteQuery,
) -> ResultT:
    """Invoke one analyzer against local rows with mock Flight metadata."""

    return run_analyzer_with_rows(
        analyzer,
        rows,
        query,
        client=None,
        flight_info=build_mock_flight_info(query, rows),
    )


__all__ = [
    "build_mock_flight_info",
    "WendaoAnalyzer",
    "WendaoAnalyzerContext",
    "run_analyzer",
    "run_analyzer_with_mock_rows",
    "run_analyzer_with_rows",
    "run_analyzer_with_table",
]
