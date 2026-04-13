"""Thin Flight-first transport client for xiuxian-wendao consumers."""

from __future__ import annotations

from dataclasses import dataclass
import math
from typing import TYPE_CHECKING, Mapping

from .config import (
    WENDAO_RERANK_DIMENSION_HEADER,
    WENDAO_RERANK_MIN_FINAL_SCORE_HEADER,
    WENDAO_RERANK_TOP_K_HEADER,
    WENDAO_SCHEMA_VERSION_HEADER,
    WendaoTransportConfig,
)
from .mode import WendaoTransportMode
from .query import (
    WendaoAttachmentSearchRequest,
    WendaoFlightRouteQuery,
    WendaoRepoSearchRequest,
    WendaoRerankRequestRow,
    attachment_search_metadata,
    attachment_search_query,
    build_rerank_request_table,
    parse_attachment_search_rows,
    parse_rerank_response_rows,
    parse_repo_search_rows,
    repo_search_metadata,
    repo_search_query,
    rerank_exchange_query,
    rerank_request_metadata,
)

if TYPE_CHECKING:
    import pyarrow.flight as flight


@dataclass(frozen=True, slots=True)
class WendaoTransportClient:
    """Client-side transport descriptor and Flight connector.

    This class does not own query execution semantics. It only describes and
    initializes how a Python consumer reaches the Rust-owned Wendao runtime.
    """

    config: WendaoTransportConfig

    def preferred_modes(self) -> tuple[WendaoTransportMode, ...]:
        """Return the configured transport priority order."""
        return self.config.preferred_modes()

    def flight_authority(self) -> str:
        """Return the Arrow Flight authority for gRPC transport."""
        return self.config.endpoint.flight_authority()

    def endpoint_url(self) -> str:
        """Return the normalized companion endpoint URL."""
        return self.config.endpoint.endpoint_url()

    def request_metadata(self) -> Mapping[str, str]:
        """Return immutable request metadata for transport setup."""
        return self.config.endpoint.metadata

    def schema_version(self) -> str:
        """Return the declared Wendao Arrow contract version."""
        return self.config.schema_version

    def flight_route(self) -> str:
        """Return the normalized route used for Flight descriptors."""
        return self.config.endpoint.normalized_path()

    def flight_location(self) -> "flight.Location":
        """Return the Arrow Flight location for the configured endpoint."""
        flight = _flight_module()
        if self.config.endpoint.tls:
            return flight.Location.for_grpc_tls(
                self.config.endpoint.host,
                self.config.endpoint.port,
            )
        return flight.Location.for_grpc_tcp(
            self.config.endpoint.host,
            self.config.endpoint.port,
        )

    def flight_call_options(
        self,
        *,
        extra_metadata: Mapping[str, str] | None = None,
    ) -> "flight.FlightCallOptions":
        """Build call options for Arrow Flight RPCs."""
        flight = _flight_module()
        metadata = {
            WENDAO_SCHEMA_VERSION_HEADER: self.schema_version(),
            **self.request_metadata(),
            **(extra_metadata or {}),
        }
        headers = [(key.encode("utf-8"), value.encode("utf-8")) for key, value in metadata.items()]
        return flight.FlightCallOptions(
            timeout=self.config.request_timeout_seconds,
            headers=headers or None,
        )

    def connect_flight(self, **kwargs: object) -> "flight.FlightClient":
        """Construct a ``pyarrow.flight.FlightClient`` for the configured endpoint."""
        flight = _flight_module()
        return flight.connect(self.flight_location(), **kwargs)

    def flight_descriptor(self) -> "flight.FlightDescriptor":
        """Build a route-backed Flight descriptor aligned to the Rust runtime."""
        return self.flight_descriptor_for_query(WendaoFlightRouteQuery(route=self.flight_route()))

    def flight_descriptor_for_query(
        self,
        query: WendaoFlightRouteQuery,
    ) -> "flight.FlightDescriptor":
        """Build a route-backed Flight descriptor for one typed query."""
        flight = _flight_module()
        segments = query.descriptor_segments()
        return flight.FlightDescriptor.for_path(*segments)

    def make_ticket(self, ticket: str | bytes) -> "flight.Ticket":
        """Build a Flight ticket from UTF-8 text or raw bytes."""
        flight = _flight_module()
        ticket_bytes = ticket.encode("utf-8") if isinstance(ticket, str) else ticket
        return flight.Ticket(ticket_bytes)

    def get_flight_info(self, **connect_kwargs: object):
        """Fetch Flight metadata for the configured route descriptor."""
        return self.get_query_info(
            WendaoFlightRouteQuery(route=self.flight_route()),
            **connect_kwargs,
        )

    def get_query_info(
        self,
        query: WendaoFlightRouteQuery,
        *,
        extra_metadata: Mapping[str, str] | None = None,
        **connect_kwargs: object,
    ):
        """Fetch Flight metadata for one typed route query."""
        client = self.connect_flight(**connect_kwargs)
        return client.get_flight_info(
            self.flight_descriptor_for_query(query),
            self.flight_call_options(extra_metadata=extra_metadata),
        )

    def read_table(
        self,
        ticket: str | bytes,
        **connect_kwargs: object,
    ):
        """Read one Arrow table through ``do_get``."""
        return self.read_query_table(
            WendaoFlightRouteQuery(route=self.flight_route(), ticket=ticket),
            **connect_kwargs,
        )

    def read_query_table(
        self,
        query: WendaoFlightRouteQuery,
        *,
        extra_metadata: Mapping[str, str] | None = None,
        **connect_kwargs: object,
    ):
        """Read one Arrow table through ``do_get`` for one typed route query."""
        client = self.connect_flight(**connect_kwargs)
        reader = client.do_get(
            self.make_ticket(query.effective_ticket()),
            self.flight_call_options(extra_metadata=extra_metadata),
        )
        return reader.read_all()

    def get_repo_search_info(
        self,
        request: WendaoRepoSearchRequest,
        **connect_kwargs: object,
    ):
        """Fetch Flight metadata for the stable repo-search query."""

        return self.get_query_info(
            repo_search_query(),
            extra_metadata=repo_search_metadata(request),
            **connect_kwargs,
        )

    def get_attachment_search_info(
        self,
        request: WendaoAttachmentSearchRequest,
        **connect_kwargs: object,
    ):
        """Fetch Flight metadata for the stable attachment-search query."""

        return self.get_query_info(
            attachment_search_query(),
            extra_metadata=attachment_search_metadata(request),
            **connect_kwargs,
        )

    def read_repo_search_table(
        self,
        request: WendaoRepoSearchRequest,
        **connect_kwargs: object,
    ):
        """Read the stable repo-search Arrow table."""

        return self.read_query_table(
            repo_search_query(),
            extra_metadata=repo_search_metadata(request),
            **connect_kwargs,
        )

    def read_attachment_search_table(
        self,
        request: WendaoAttachmentSearchRequest,
        **connect_kwargs: object,
    ):
        """Read the stable attachment-search Arrow table."""

        return self.read_query_table(
            attachment_search_query(),
            extra_metadata=attachment_search_metadata(request),
            **connect_kwargs,
        )

    def read_repo_search_rows(
        self,
        request: WendaoRepoSearchRequest,
        **connect_kwargs: object,
    ):
        """Read and parse stable repo-search rows."""

        return parse_repo_search_rows(self.read_repo_search_table(request, **connect_kwargs))

    def read_attachment_search_rows(
        self,
        request: WendaoAttachmentSearchRequest,
        **connect_kwargs: object,
    ):
        """Read and parse stable attachment-search rows."""

        return parse_attachment_search_rows(
            self.read_attachment_search_table(request, **connect_kwargs)
        )

    def exchange_table(
        self,
        table,
        **connect_kwargs: object,
    ):
        """Round-trip one Arrow table through ``do_exchange``."""
        return self.exchange_query_table(
            WendaoFlightRouteQuery(route=self.flight_route()),
            table,
            **connect_kwargs,
        )

    def exchange_query_table(
        self,
        query: WendaoFlightRouteQuery,
        table,
        *,
        extra_metadata: Mapping[str, str] | None = None,
        **connect_kwargs: object,
    ):
        """Round-trip one Arrow table through ``do_exchange`` for one typed query."""
        client = self.connect_flight(**connect_kwargs)
        writer, reader = client.do_exchange(
            self.flight_descriptor_for_query(query),
            self.flight_call_options(extra_metadata=extra_metadata),
        )
        begin = getattr(writer, "begin", None)
        if callable(begin):
            begin(table.schema)
        writer.write_table(table)
        done_writing = getattr(writer, "done_writing", None)
        if callable(done_writing):
            done_writing()
        return reader.read_all()

    def exchange_rerank_table(
        self,
        table,
        *,
        embedding_dimension: int | None = None,
        top_k: int | None = None,
        min_final_score: float | None = None,
        **connect_kwargs: object,
    ):
        """Round-trip one typed rerank request table through the stable route."""
        extra_metadata: dict[str, str] | None = None
        if embedding_dimension is not None:
            extra_metadata = {WENDAO_RERANK_DIMENSION_HEADER: str(embedding_dimension)}
        if top_k is not None:
            if top_k <= 0:
                raise ValueError("rerank top_k must be greater than zero")
            extra_metadata = dict(extra_metadata or {})
            extra_metadata[WENDAO_RERANK_TOP_K_HEADER] = str(top_k)
        if min_final_score is not None:
            if not math.isfinite(min_final_score):
                raise ValueError("rerank min_final_score must be finite")
            if not 0.0 <= min_final_score <= 1.0:
                raise ValueError(
                    "rerank min_final_score must stay within inclusive range [0.0, 1.0]"
                )
            extra_metadata = dict(extra_metadata or {})
            extra_metadata[WENDAO_RERANK_MIN_FINAL_SCORE_HEADER] = str(min_final_score)
        return self.exchange_query_table(
            rerank_exchange_query(),
            table,
            extra_metadata=extra_metadata,
            **connect_kwargs,
        )

    def exchange_rerank_rows(
        self,
        rows: list[WendaoRerankRequestRow],
        *,
        top_k: int | None = None,
        min_final_score: float | None = None,
        **connect_kwargs: object,
    ):
        """Build and send one typed rerank request through the stable route."""
        metadata = rerank_request_metadata(
            rows,
            top_k=top_k,
            min_final_score=min_final_score,
        )
        kwargs: dict[str, object] = {
            "embedding_dimension": int(metadata[WENDAO_RERANK_DIMENSION_HEADER]),
            **connect_kwargs,
        }
        if top_k is not None:
            kwargs["top_k"] = top_k
        if min_final_score is not None:
            kwargs["min_final_score"] = min_final_score
        return self.exchange_rerank_table(
            build_rerank_request_table(rows),
            **kwargs,
        )

    def exchange_rerank_result_rows(
        self,
        rows: list[WendaoRerankRequestRow],
        *,
        top_k: int | None = None,
        min_final_score: float | None = None,
        **connect_kwargs: object,
    ):
        """Build, send, and parse one typed rerank request through the stable route."""
        return parse_rerank_response_rows(
            self.exchange_rerank_rows(
                rows,
                top_k=top_k,
                min_final_score=min_final_score,
                **connect_kwargs,
            )
        )


def _flight_module() -> "flight":
    import pyarrow.flight as flight

    return flight


__all__ = ["WendaoTransportClient"]
