"""Thin Flight-first transport client for xiuxian-wendao consumers."""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING, Mapping

from .config import WENDAO_SCHEMA_VERSION_HEADER, WendaoTransportConfig
from .mode import WendaoTransportMode
from .query import WendaoFlightRouteQuery

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

    def flight_call_options(self) -> "flight.FlightCallOptions":
        """Build call options for Arrow Flight RPCs."""
        flight = _flight_module()
        metadata = {
            WENDAO_SCHEMA_VERSION_HEADER: self.schema_version(),
            **self.request_metadata(),
        }
        headers = [
            (key.encode("utf-8"), value.encode("utf-8"))
            for key, value in metadata.items()
        ]
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
        return self.flight_descriptor_for_query(
            WendaoFlightRouteQuery(route=self.flight_route())
        )

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
        **connect_kwargs: object,
    ):
        """Fetch Flight metadata for one typed route query."""
        client = self.connect_flight(**connect_kwargs)
        return client.get_flight_info(
            self.flight_descriptor_for_query(query),
            self.flight_call_options(),
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
        **connect_kwargs: object,
    ):
        """Read one Arrow table through ``do_get`` for one typed route query."""
        client = self.connect_flight(**connect_kwargs)
        reader = client.do_get(
            self.make_ticket(query.effective_ticket()),
            self.flight_call_options(),
        )
        return reader.read_all()


def _flight_module() -> "flight":
    import pyarrow.flight as flight

    return flight


__all__ = ["WendaoTransportClient"]
