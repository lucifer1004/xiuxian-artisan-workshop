"""Thin transport client descriptors for xiuxian-wendao consumers."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Mapping

from .config import WendaoTransportConfig
from .mode import WendaoTransportMode


@dataclass(frozen=True, slots=True)
class WendaoTransportClient:
    """Client-side transport descriptor.

    This class does not own query execution semantics. It only describes how a
    Python consumer reaches the Rust-owned Wendao runtime.
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


__all__ = ["WendaoTransportClient"]
