"""Transport configuration records for xiuxian-wendao clients."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Mapping

from .mode import WendaoTransportMode

WENDAO_SCHEMA_VERSION_HEADER = "x-wendao-schema-version"
WENDAO_RERANK_DIMENSION_HEADER = "x-wendao-rerank-embedding-dimension"
WENDAO_RERANK_TOP_K_HEADER = "x-wendao-rerank-top-k"
WENDAO_RERANK_MIN_FINAL_SCORE_HEADER = "x-wendao-rerank-min-final-score"
WENDAO_REPO_SEARCH_QUERY_HEADER = "x-wendao-repo-search-query"
WENDAO_REPO_SEARCH_LIMIT_HEADER = "x-wendao-repo-search-limit"
WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER = "x-wendao-repo-search-language-filters"
WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER = "x-wendao-repo-search-path-prefixes"
WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER = "x-wendao-repo-search-filename-filters"
WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER = "x-wendao-repo-search-tag-filters"
WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER = "x-wendao-repo-search-title-filters"
REPO_SEARCH_DEFAULT_LIMIT = 10


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
    "REPO_SEARCH_DEFAULT_LIMIT",
    "WENDAO_RERANK_DIMENSION_HEADER",
    "WENDAO_RERANK_MIN_FINAL_SCORE_HEADER",
    "WENDAO_RERANK_TOP_K_HEADER",
    "WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER",
    "WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER",
    "WENDAO_REPO_SEARCH_LIMIT_HEADER",
    "WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER",
    "WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER",
    "WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER",
    "WENDAO_REPO_SEARCH_QUERY_HEADER",
    "WENDAO_SCHEMA_VERSION_HEADER",
    "WendaoTransportConfig",
    "WendaoTransportEndpoint",
]
