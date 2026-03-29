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

    client: WendaoTransportClient
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

    flight_info = (
        client.get_query_info(query, **connect_kwargs) if include_flight_info else None
    )
    table = client.read_query_table(query, **connect_kwargs)
    context = WendaoAnalyzerContext(
        client=client,
        query=query,
        flight_info=flight_info,
    )
    return analyzer(table, context)


__all__ = ["WendaoAnalyzer", "WendaoAnalyzerContext", "run_analyzer"]
