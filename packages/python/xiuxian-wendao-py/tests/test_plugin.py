from __future__ import annotations

import pyarrow as pa

from xiuxian_wendao_py import WendaoAnalyzerPlugin
from xiuxian_wendao_py.plugin import (
    PluginTransportKind,
    build_arrow_flight_binding,
)
from xiuxian_wendao_py.transport import (
    WendaoFlightRouteQuery,
    WendaoTransportClient,
    WendaoTransportConfig,
    WendaoTransportEndpoint,
)


def test_build_arrow_flight_binding_matches_transport_contract() -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(
                host="127.0.0.1",
                port=50051,
                path="/ignored-by-query",
            ),
            schema_version="v3",
            request_timeout_seconds=12.0,
        )
    )
    query = WendaoFlightRouteQuery(route="/search/repos/main")

    binding = build_arrow_flight_binding(
        client,
        capability_id="repo_search",
        provider="acme.python.analyzer",
        query=query,
        health_route="/healthz",
        launch={"kind": "python_module", "module": "acme_plugin"},
    )

    assert binding.selector.capability_id == "repo_search"
    assert binding.selector.provider == "acme.python.analyzer"
    assert binding.endpoint.base_url == "http://127.0.0.1:50051/ignored-by-query"
    assert binding.endpoint.route == "/search/repos/main"
    assert binding.endpoint.health_route == "/healthz"
    assert binding.endpoint.timeout_secs == 12
    assert binding.transport is PluginTransportKind.ARROW_FLIGHT
    assert binding.contract_version == "v3"
    assert binding.to_dict()["transport"] == "arrow_flight"


def test_analyzer_plugin_builds_binding_and_runs(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
        )
    )
    query = WendaoFlightRouteQuery(route="/search/repos/main")
    expected = pa.table({"id": ["doc-1"], "score": [0.9]})
    captured: dict[str, object] = {}

    def fake_get_query_info(self, resolved_query, **kwargs):  # type: ignore[no-untyped-def]
        return {"route": resolved_query.normalized_route()}

    def fake_read_query_table(self, resolved_query, **kwargs):  # type: ignore[no-untyped-def]
        captured["query"] = resolved_query
        return expected

    monkeypatch.setattr(WendaoTransportClient, "get_query_info", fake_get_query_info)
    monkeypatch.setattr(WendaoTransportClient, "read_query_table", fake_read_query_table)

    plugin = WendaoAnalyzerPlugin(
        capability_id="repo_search",
        provider="acme.python.analyzer",
        health_route="/healthz",
        analyzer=lambda table, context: {
            "rows": table.num_rows,
            "route": context.query.normalized_route(),
        },
    )

    binding = plugin.binding_for_client(client, query)
    result = plugin.run(client, query)

    assert binding.endpoint.route == "/search/repos/main"
    assert binding.transport is PluginTransportKind.ARROW_FLIGHT
    assert result == {"rows": 1, "route": "/search/repos/main"}
    assert captured["query"] == query
