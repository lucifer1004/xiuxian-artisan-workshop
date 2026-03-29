"""Transport modes for xiuxian-wendao Python consumers."""

from __future__ import annotations

from enum import StrEnum


class WendaoTransportMode(StrEnum):
    """Supported client-side transport modes.

    `EMBEDDED` exists only as a compatibility seam while direct bindings are
    retired from the default architecture.
    """

    FLIGHT = "flight"
    ARROW_IPC = "arrow_ipc"
    EMBEDDED = "embedded"


__all__ = ["WendaoTransportMode"]
