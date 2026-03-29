"""Transport-first Flight client API for xiuxian-wendao Python consumers."""

from .client import WendaoTransportClient
from .config import WendaoTransportConfig, WendaoTransportEndpoint
from .mode import WendaoTransportMode
from .query import WendaoFlightRouteQuery

__all__ = [
    "WendaoFlightRouteQuery",
    "WendaoTransportClient",
    "WendaoTransportConfig",
    "WendaoTransportEndpoint",
    "WendaoTransportMode",
]
