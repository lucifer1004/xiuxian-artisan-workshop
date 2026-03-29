from __future__ import annotations

from xiuxian_wendao_py import (
    WendaoTransportClient,
    WendaoTransportConfig,
    WendaoTransportEndpoint,
    WendaoTransportMode,
)


def test_transport_client_prefers_flight_then_arrow_ipc() -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
        )
    )

    assert client.preferred_modes() == (
        WendaoTransportMode.FLIGHT,
        WendaoTransportMode.ARROW_IPC,
    )
    assert client.flight_authority() == "127.0.0.1:50051"
    assert client.endpoint_url() == "http://127.0.0.1:50051/"


def test_transport_client_embedded_mode_is_opt_in() -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(
                host="wendao.internal",
                port=8443,
                tls=True,
                path="flight",
                metadata={"authorization": "Bearer token"},
            ),
            allow_embedded=True,
        )
    )

    assert client.preferred_modes() == (
        WendaoTransportMode.FLIGHT,
        WendaoTransportMode.ARROW_IPC,
        WendaoTransportMode.EMBEDDED,
    )
    assert client.endpoint_url() == "https://wendao.internal:8443/flight"
    assert client.request_metadata() == {"authorization": "Bearer token"}
