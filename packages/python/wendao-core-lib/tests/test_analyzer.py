from __future__ import annotations

import pyarrow as pa

from wendao_core_lib.analyzer import (
    build_mock_flight_info,
    run_analyzer,
    run_analyzer_with_mock_rows,
    run_analyzer_with_rows,
    run_analyzer_with_table,
)
from wendao_core_lib.transport import (
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


def test_run_analyzer_with_table_supports_offline_replay() -> None:
    query = WendaoFlightRouteQuery(route="/search/repos/main")
    table = pa.table({"id": ["doc-3"], "score": [0.7]})

    result = run_analyzer_with_table(
        lambda value, context: {
            "rows": value.num_rows,
            "route": context.query.normalized_route(),
            "client": context.client,
            "flight_info": context.flight_info,
        },
        table,
        query,
        flight_info={"mode": "offline"},
    )

    assert result == {
        "rows": 1,
        "route": "/search/repos/main",
        "client": None,
        "flight_info": {"mode": "offline"},
    }


def test_run_analyzer_with_rows_builds_arrow_table_for_offline_replay() -> None:
    query = WendaoFlightRouteQuery(route="/search/repos/main")

    result = run_analyzer_with_rows(
        lambda table, context: {
            "rows": table.num_rows,
            "columns": table.column_names,
            "route": context.query.normalized_route(),
        },
        [{"id": "doc-4", "score": 0.6}],
        query,
    )

    assert result == {
        "rows": 1,
        "columns": ["id", "score"],
        "route": "/search/repos/main",
    }


def test_build_mock_flight_info_creates_stable_replay_metadata() -> None:
    query = WendaoFlightRouteQuery(route="/search/repos/main")

    flight_info = build_mock_flight_info(
        query,
        [{"id": "doc-4", "score": 0.6}],
    )

    assert flight_info == {
        "route": "/search/repos/main",
        "descriptor_path": ("search", "repos", "main"),
        "ticket": "/search/repos/main",
        "row_count": 1,
        "mode": "mock_flight",
    }


def test_run_analyzer_with_mock_rows_adds_mock_flight_context() -> None:
    query = WendaoFlightRouteQuery(route="/search/repos/main")

    result = run_analyzer_with_mock_rows(
        lambda table, context: {
            "rows": table.num_rows,
            "route": context.query.normalized_route(),
            "flight_info": context.flight_info,
            "client": context.client,
        },
        [{"id": "doc-5", "score": 0.5}],
        query,
    )

    assert result == {
        "rows": 1,
        "route": "/search/repos/main",
        "flight_info": {
            "route": "/search/repos/main",
            "descriptor_path": ("search", "repos", "main"),
            "ticket": "/search/repos/main",
            "row_count": 1,
            "mode": "mock_flight",
        },
        "client": None,
    }
