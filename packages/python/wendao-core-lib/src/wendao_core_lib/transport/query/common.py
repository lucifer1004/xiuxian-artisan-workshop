"""Shared route-query helpers for Wendao Flight transport contracts."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class WendaoFlightRouteQuery:
    """One route-backed Flight query description.

    The Rust runtime owns the actual query semantics. Python only keeps the
    route plus the effective ticket bytes needed for `get_flight_info(...)`
    and `do_get(...)`.
    """

    route: str
    ticket: str | bytes | None = None

    def normalized_route(self) -> str:
        """Return the route with a single leading slash."""

        stripped = self.route.strip()
        if not stripped or stripped == "/":
            raise ValueError(
                "Arrow Flight route query must resolve to at least one descriptor segment"
            )
        return f"/{stripped.lstrip('/')}"

    def descriptor_segments(self) -> tuple[str, ...]:
        """Return the normalized route split into descriptor segments."""

        return tuple(
            segment for segment in self.normalized_route().strip("/").split("/") if segment
        )

    def effective_ticket(self) -> str | bytes:
        """Return the explicit ticket or fall back to the normalized route."""

        return self.ticket if self.ticket is not None else self.normalized_route()


__all__ = ["WendaoFlightRouteQuery"]
