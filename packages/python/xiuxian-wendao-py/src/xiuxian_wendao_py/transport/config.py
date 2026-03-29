"""Transport configuration records for xiuxian-wendao clients."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Mapping

from .mode import WendaoTransportMode

WENDAO_SCHEMA_VERSION_HEADER = "x-wendao-schema-version"


@dataclass(frozen=True, slots=True)
class WendaoTransportEndpoint:
    """Network coordinates for the Rust-owned Wendao runtime."""

    host: str
    port: int
    tls: bool = False
    path: str = "/"
    metadata: Mapping[str, str] = field(default_factory=dict)

    def normalized_path(self) -> str:
        """Return the HTTP-style path with a single leading slash."""
        stripped = self.path.strip()
        if not stripped or stripped == "/":
            return "/"
        return f"/{stripped.lstrip('/')}"

    def scheme(self) -> str:
        """Return the transport scheme for HTTP-style companion endpoints."""
        return "https" if self.tls else "http"

    def flight_authority(self) -> str:
        """Return the gRPC authority used by Arrow Flight clients."""
        return f"{self.host}:{self.port}"

    def endpoint_url(self) -> str:
        """Return the normalized endpoint URL for auxiliary HTTP surfaces."""
        return f"{self.scheme()}://{self.flight_authority()}{self.normalized_path()}"


@dataclass(frozen=True, slots=True)
class WendaoTransportConfig:
    """Client-side transport policy.

    Flight is the required first choice. Arrow IPC is the sanctioned fallback.
    Embedded mode is opt-in and compatibility-only.
    """

    endpoint: WendaoTransportEndpoint
    schema_version: str = "v1"
    request_timeout_seconds: float = 30.0
    allow_embedded: bool = False
    prefer_arrow_ipc_fallback: bool = True

    def preferred_modes(self) -> tuple[WendaoTransportMode, ...]:
        """Return transport modes in execution order."""
        modes: list[WendaoTransportMode] = [WendaoTransportMode.FLIGHT]
        if self.prefer_arrow_ipc_fallback:
            modes.append(WendaoTransportMode.ARROW_IPC)
        if self.allow_embedded:
            modes.append(WendaoTransportMode.EMBEDDED)
        return tuple(modes)


__all__ = [
    "WENDAO_SCHEMA_VERSION_HEADER",
    "WendaoTransportConfig",
    "WendaoTransportEndpoint",
]
