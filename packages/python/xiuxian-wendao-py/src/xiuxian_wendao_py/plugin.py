"""Plugin authoring scaffold for Arrow-backed Wendao analyzers."""

from __future__ import annotations

from dataclasses import asdict, dataclass
from enum import StrEnum
from typing import Mapping

from .analyzer import ResultT, WendaoAnalyzer, run_analyzer
from .transport import WendaoFlightRouteQuery, WendaoTransportClient


class PluginTransportKind(StrEnum):
    """Transport kinds aligned to the Rust plugin-runtime contract."""

    ARROW_FLIGHT = "arrow_flight"
    ARROW_IPC_HTTP = "arrow_ipc_http"
    LOCAL_PROCESS_ARROW_IPC = "local_process_arrow_ipc"


@dataclass(frozen=True, slots=True)
class PluginProviderSelector:
    """Python mirror of the Rust capability-to-provider selector."""

    capability_id: str
    provider: str


@dataclass(frozen=True, slots=True)
class PluginTransportEndpoint:
    """Python mirror of the Rust plugin transport endpoint record."""

    base_url: str | None = None
    route: str | None = None
    health_route: str | None = None
    timeout_secs: int | None = None


@dataclass(frozen=True, slots=True)
class PluginCapabilityBinding:
    """Python mirror of the Rust runtime binding contract."""

    selector: PluginProviderSelector
    endpoint: PluginTransportEndpoint
    transport: PluginTransportKind
    contract_version: str
    launch: Mapping[str, object] | None = None

    def to_dict(self) -> dict[str, object]:
        """Render a JSON-serializable contract payload."""
        payload = asdict(self)
        payload["transport"] = self.transport.value
        return payload


def build_arrow_flight_binding(
    client: WendaoTransportClient,
    *,
    capability_id: str,
    provider: str,
    query: WendaoFlightRouteQuery,
    health_route: str | None = None,
    launch: Mapping[str, object] | None = None,
) -> PluginCapabilityBinding:
    """Build one Arrow Flight binding from the Python transport client."""

    return PluginCapabilityBinding(
        selector=PluginProviderSelector(
            capability_id=capability_id,
            provider=provider,
        ),
        endpoint=PluginTransportEndpoint(
            base_url=client.endpoint_url(),
            route=query.normalized_route(),
            health_route=health_route,
            timeout_secs=int(client.config.request_timeout_seconds),
        ),
        transport=PluginTransportKind.ARROW_FLIGHT,
        contract_version=client.schema_version(),
        launch=launch,
    )


@dataclass(frozen=True, slots=True)
class WendaoAnalyzerPlugin:
    """One minimal analyzer-plugin scaffold for downstream Python authors."""

    capability_id: str
    provider: str
    analyzer: WendaoAnalyzer[ResultT]
    health_route: str | None = None
    launch: Mapping[str, object] | None = None

    def binding_for_client(
        self,
        client: WendaoTransportClient,
        query: WendaoFlightRouteQuery,
    ) -> PluginCapabilityBinding:
        """Build the runtime binding advertised by this plugin."""

        return build_arrow_flight_binding(
            client,
            capability_id=self.capability_id,
            provider=self.provider,
            query=query,
            health_route=self.health_route,
            launch=self.launch,
        )

    def run(
        self,
        client: WendaoTransportClient,
        query: WendaoFlightRouteQuery,
        *,
        include_flight_info: bool = True,
        **connect_kwargs: object,
    ) -> ResultT:
        """Run the plugin analyzer against one routed Arrow query."""

        return run_analyzer(
            client,
            self.analyzer,
            query,
            include_flight_info=include_flight_info,
            **connect_kwargs,
        )


__all__ = [
    "PluginCapabilityBinding",
    "PluginProviderSelector",
    "PluginTransportEndpoint",
    "PluginTransportKind",
    "WendaoAnalyzerPlugin",
    "build_arrow_flight_binding",
]
