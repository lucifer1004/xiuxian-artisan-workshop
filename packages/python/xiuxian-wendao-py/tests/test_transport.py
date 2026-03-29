from __future__ import annotations

import pyarrow as pa
import pyarrow.flight as flight

from xiuxian_wendao_py.transport import (
    WendaoFlightRouteQuery,
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
    assert client.flight_location() == flight.Location.for_grpc_tcp("127.0.0.1", 50051)


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
    assert client.flight_location() == flight.Location.for_grpc_tls("wendao.internal", 8443)


def test_transport_client_builds_flight_call_options() -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(
                host="wendao.internal",
                port=8443,
                metadata={"authorization": "Bearer token", "x-trace-id": "trace-123"},
            ),
            schema_version="v7",
            request_timeout_seconds=12.5,
        )
    )

    options = client.flight_call_options()

    assert options.timeout == 12.5
    assert options.headers == [
        (b"x-wendao-schema-version", b"v7"),
        (b"authorization", b"Bearer token"),
        (b"x-trace-id", b"trace-123"),
    ]


def test_transport_client_builds_route_backed_flight_descriptor() -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(
                host="127.0.0.1",
                port=50051,
                path="search/repos/main",
            ),
        )
    )

    descriptor = client.flight_descriptor()

    assert descriptor.descriptor_type == flight.DescriptorType.PATH
    assert descriptor.path == [b"search", b"repos", b"main"]


def test_route_query_normalizes_route_and_defaults_ticket() -> None:
    query = WendaoFlightRouteQuery(route="search/repos/main")

    assert query.normalized_route() == "/search/repos/main"
    assert query.descriptor_segments() == ("search", "repos", "main")
    assert query.effective_ticket() == "/search/repos/main"


def test_transport_client_builds_descriptor_for_typed_query() -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
        )
    )

    descriptor = client.flight_descriptor_for_query(
        WendaoFlightRouteQuery(route="/search/repos/main")
    )

    assert descriptor.descriptor_type == flight.DescriptorType.PATH
    assert descriptor.path == [b"search", b"repos", b"main"]


def test_transport_client_connects_via_pyarrow_flight(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
        )
    )
    captured: dict[str, object] = {}

    def fake_connect(location: object, **kwargs: object) -> object:
        captured["location"] = location
        captured["kwargs"] = kwargs
        return "flight-client"

    monkeypatch.setattr(flight, "connect", fake_connect)

    connected = client.connect_flight(tls_root_certs=b"roots")

    assert connected == "flight-client"
    assert captured["location"] == flight.Location.for_grpc_tcp("127.0.0.1", 50051)
    assert captured["kwargs"] == {"tls_root_certs": b"roots"}


def test_transport_client_builds_ticket_from_utf8_text() -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
        )
    )

    ticket = client.make_ticket("search/repos/main")

    assert isinstance(ticket, flight.Ticket)
    assert ticket.ticket == b"search/repos/main"


def test_transport_client_fetches_flight_info_for_route_descriptor(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(
                host="127.0.0.1",
                port=50051,
                path="/search/repos/main",
            ),
            schema_version="v3",
            request_timeout_seconds=9.0,
        )
    )
    captured: dict[str, object] = {}

    class FakeClient:
        def get_flight_info(self, descriptor: object, options: object) -> str:
            captured["descriptor"] = descriptor
            captured["options"] = options
            return "flight-info"

    monkeypatch.setattr(
        WendaoTransportClient,
        "connect_flight",
        lambda self, **kwargs: FakeClient(),
    )

    info = client.get_flight_info()

    assert info == "flight-info"
    assert captured["descriptor"].descriptor_type == flight.DescriptorType.PATH
    assert captured["descriptor"].path == [b"search", b"repos", b"main"]
    assert captured["options"].timeout == 9.0
    assert captured["options"].headers == [(b"x-wendao-schema-version", b"v3")]


def test_transport_client_fetches_flight_info_for_typed_query(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
        )
    )
    captured: dict[str, object] = {}

    class FakeClient:
        def get_flight_info(self, descriptor: object, options: object) -> str:
            captured["descriptor"] = descriptor
            captured["options"] = options
            return "query-info"

    monkeypatch.setattr(
        WendaoTransportClient,
        "connect_flight",
        lambda self, **kwargs: FakeClient(),
    )

    info = client.get_query_info(
        WendaoFlightRouteQuery(route="/search/repos/main", ticket="ticket/repos/main")
    )

    assert info == "query-info"
    assert captured["descriptor"].path == [b"search", b"repos", b"main"]
    assert captured["options"].headers == [(b"x-wendao-schema-version", b"v1")]


def test_transport_client_reads_table_via_do_get(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(
                host="127.0.0.1",
                port=50051,
                metadata={"authorization": "Bearer token"},
            ),
            request_timeout_seconds=9.0,
        )
    )
    expected = pa.table({"id": ["doc-1"], "score": [0.9]})
    captured: dict[str, object] = {}

    class FakeReader:
        def read_all(self) -> pa.Table:
            return expected

    class FakeClient:
        def do_get(self, ticket: object, options: object) -> FakeReader:
            captured["ticket"] = ticket
            captured["options"] = options
            return FakeReader()

    monkeypatch.setattr(
        WendaoTransportClient,
        "connect_flight",
        lambda self, **kwargs: FakeClient(),
    )

    table = client.read_table("search/repos/main")

    assert table == expected
    assert isinstance(captured["ticket"], flight.Ticket)
    assert captured["ticket"].ticket == b"search/repos/main"
    assert captured["options"].timeout == 9.0
    assert captured["options"].headers == [
        (b"x-wendao-schema-version", b"v1"),
        (b"authorization", b"Bearer token"),
    ]


def test_transport_client_reads_table_for_typed_query(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
        )
    )
    expected = pa.table({"id": ["doc-2"], "score": [0.8]})
    captured: dict[str, object] = {}

    class FakeReader:
        def read_all(self) -> pa.Table:
            return expected

    class FakeClient:
        def do_get(self, ticket: object, options: object) -> FakeReader:
            captured["ticket"] = ticket
            captured["options"] = options
            return FakeReader()

    monkeypatch.setattr(
        WendaoTransportClient,
        "connect_flight",
        lambda self, **kwargs: FakeClient(),
    )

    table = client.read_query_table(
        WendaoFlightRouteQuery(
            route="/search/repos/main",
            ticket="ticket/repos/main",
        )
    )

    assert table == expected
    assert isinstance(captured["ticket"], flight.Ticket)
    assert captured["ticket"].ticket == b"ticket/repos/main"
    assert captured["options"].headers == [(b"x-wendao-schema-version", b"v1")]
