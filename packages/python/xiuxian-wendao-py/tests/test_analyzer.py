from __future__ import annotations

import pyarrow as pa

from xiuxian_wendao_py.analyzer import run_analyzer
from xiuxian_wendao_py.transport import (
    WendaoFlightRouteQuery,
    WendaoTransportClient,
    WendaoTransportConfig,
    WendaoTransportEndpoint,
)


def test_run_analyzer_fetches_flight_info_and_table(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
        )
    )
    query = WendaoFlightRouteQuery(route="/search/repos/main")
    expected = pa.table({"id": ["doc-1"], "score": [0.9]})
    captured: dict[str, object] = {}

    def fake_get_query_info(self, resolved_query, **kwargs):  # type: ignore[no-untyped-def]
        captured["info_query"] = resolved_query
        captured["info_kwargs"] = kwargs
        return {"route": resolved_query.normalized_route(), "kind": "flight-info"}

    def fake_read_query_table(self, resolved_query, **kwargs):  # type: ignore[no-untyped-def]
        captured["table_query"] = resolved_query
        captured["table_kwargs"] = kwargs
        return expected

    monkeypatch.setattr(WendaoTransportClient, "get_query_info", fake_get_query_info)
    monkeypatch.setattr(WendaoTransportClient, "read_query_table", fake_read_query_table)

    def analyzer(table: pa.Table, context) -> dict[str, object]:
        captured["analyzer_table"] = table
        captured["analyzer_context"] = context
        return {
            "rows": table.num_rows,
            "route": context.query.normalized_route(),
            "flight_info": context.flight_info,
        }

    result = run_analyzer(client, analyzer, query, tls_root_certs=b"roots")

    assert result == {
        "rows": 1,
        "route": "/search/repos/main",
        "flight_info": {"route": "/search/repos/main", "kind": "flight-info"},
    }
    assert captured["info_query"] == query
    assert captured["table_query"] == query
    assert captured["info_kwargs"] == {"tls_root_certs": b"roots"}
    assert captured["table_kwargs"] == {"tls_root_certs": b"roots"}
    assert captured["analyzer_table"] == expected
    assert captured["analyzer_context"].client == client


def test_run_analyzer_can_skip_flight_info(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
        )
    )
    query = WendaoFlightRouteQuery(route="/search/repos/main")
    expected = pa.table({"id": ["doc-2"], "score": [0.8]})

    def fail_get_query_info(self, resolved_query, **kwargs):  # type: ignore[no-untyped-def]
        raise AssertionError("flight info should be skipped")

    def fake_read_query_table(self, resolved_query, **kwargs):  # type: ignore[no-untyped-def]
        return expected

    monkeypatch.setattr(WendaoTransportClient, "get_query_info", fail_get_query_info)
    monkeypatch.setattr(WendaoTransportClient, "read_query_table", fake_read_query_table)

    result = run_analyzer(
        client,
        lambda table, context: (table.num_rows, context.flight_info),
        query,
        include_flight_info=False,
    )

    assert result == (1, None)
