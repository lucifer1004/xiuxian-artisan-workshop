from __future__ import annotations

import os
import socket
import subprocess
import time

import pyarrow as pa
import pytest

from xiuxian_wendao_analyzer import (
    AnalyzerConfig,
    analyze_query,
    analyze_query_results,
    analyze_repo_query_text,
    analyze_repo_query_text_results,
    analyze_repo_search,
    analyze_repo_search_results,
    build_analyzer,
    run_query_analysis,
    run_repo_analysis,
    run_repo_search_analysis,
    run_rerank_exchange_analysis,
    summarize_query,
    summarize_query_results,
    summarize_query_route,
    summarize_repo_analysis,
    summarize_repo_search,
    summarize_repo_search_results,
    summarize_repo_query_text,
    summarize_repo_query_text_results,
    summarize_rerank_exchange,
)
from xiuxian_wendao_py import (
    WendaoFlightRouteQuery,
    WendaoRerankRequestRow,
    WendaoRerankResultRow,
    WendaoTransportClient,
    WendaoTransportConfig,
    WendaoTransportEndpoint,
    repo_search_metadata,
    repo_search_query,
    repo_search_request,
)


def test_analyze_query_uses_transport_client_table_fetch(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )
    query = WendaoFlightRouteQuery(route="/rerank/flight")

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == query
        assert connect_kwargs == {"tls_root_certs": b"roots"}
        return pa.Table.from_pylist(
            [
                {
                    "doc_id": "doc-a",
                    "vector_score": 0.2,
                    "embedding": (1.0, 0.0),
                    "query_embedding": (1.0, 0.0),
                },
                {
                    "doc_id": "doc-b",
                    "vector_score": 0.9,
                    "embedding": (0.0, 1.0),
                    "query_embedding": (1.0, 0.0),
                },
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    ranked = analyze_query(client, query, tls_root_certs=b"roots")

    assert [row["doc_id"] for row in ranked] == ["doc-a", "doc-b"]
    assert ranked[0]["rank"] == 1
    assert ranked[1]["rank"] == 2


def test_analyze_query_uses_explicit_analyzer() -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )
    query = WendaoFlightRouteQuery(route="/rerank/flight")

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == query
        assert connect_kwargs == {}
        return pa.Table.from_pylist(
            [
                {
                    "doc_id": "doc-a",
                    "vector_score": 0.1,
                    "embedding": (1.0, 0.0),
                    "query_embedding": (1.0, 0.0),
                }
            ]
        )

    analyzer = build_analyzer(AnalyzerConfig(vector_weight=1.0, similarity_weight=0.0))

    from pytest import MonkeyPatch

    monkeypatch = MonkeyPatch()
    try:
        monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)
        ranked = analyze_query(client, query, analyzer=analyzer)
    finally:
        monkeypatch.undo()

    assert ranked == [
        {
            "doc_id": "doc-a",
            "vector_score": 0.1,
            "semantic_score": 1.0,
            "final_score": 0.1,
            "rank": 1,
        }
    ]


def test_run_query_analysis_preserves_query_and_typed_rows(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )
    query = WendaoFlightRouteQuery(route="/rerank/flight")

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == query
        assert connect_kwargs == {}
        return pa.Table.from_pylist(
            [
                {
                    "doc_id": "doc-a",
                    "vector_score": 0.1,
                    "embedding": (1.0, 0.0),
                    "query_embedding": (1.0, 0.0),
                }
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    run = run_query_analysis(client, query)

    assert run.query == query
    assert len(run.rows) == 1
    assert run.rows[0].doc_id == "doc-a"
    assert run.rows[0].rank == 1


def test_summarize_query_returns_top_row_snapshot(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )
    query = WendaoFlightRouteQuery(route="/rerank/flight")

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == query
        assert connect_kwargs == {}
        return pa.Table.from_pylist(
            [
                {
                    "doc_id": "doc-a",
                    "vector_score": 0.1,
                    "embedding": (1.0, 0.0),
                    "query_embedding": (1.0, 0.0),
                }
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    summary = summarize_query(run_query_analysis(client, query))

    assert summary.row_count == 1
    assert summary.top_doc_id == "doc-a"
    assert summary.top_rank == 1


def test_summarize_query_route_returns_top_row_snapshot(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )
    query = WendaoFlightRouteQuery(route="/rerank/flight")

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == query
        assert connect_kwargs == {}
        return pa.Table.from_pylist(
            [
                {
                    "doc_id": "doc-a",
                    "vector_score": 0.1,
                    "embedding": (1.0, 0.0),
                    "query_embedding": (1.0, 0.0),
                }
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    summary = summarize_query_route(client, query)

    assert summary.row_count == 1
    assert summary.top_doc_id == "doc-a"
    assert summary.top_rank == 1


def test_summarize_query_results_returns_top_row_snapshot(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )
    query = WendaoFlightRouteQuery(route="/rerank/flight")

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == query
        assert connect_kwargs == {}
        return pa.Table.from_pylist(
            [
                {
                    "doc_id": "doc-a",
                    "vector_score": 0.1,
                    "embedding": (1.0, 0.0),
                    "query_embedding": (1.0, 0.0),
                }
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    summary = summarize_query_results(client, query)

    assert summary.row_count == 1
    assert summary.top_doc_id == "doc-a"
    assert summary.top_rank == 1


def test_analyze_repo_search_uses_typed_request_metadata(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )
    request = repo_search_request("alpha", limit=2, path_prefixes=("src/",))

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == repo_search_query()
        assert connect_kwargs == {"extra_metadata": repo_search_metadata(request)}
        return pa.Table.from_pylist(
            [
                {
                    "path": "src/lib.rs",
                    "score": 0.9,
                },
                {
                    "path": "src/main.rs",
                    "score": 0.3,
                },
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    ranked = analyze_repo_search(
        client,
        request,
        analyzer=_RepoSearchScoreAnalyzer(),
    )

    assert ranked == [
        {"path": "src/lib.rs", "score": 0.9, "rank": 1},
        {"path": "src/main.rs", "score": 0.3, "rank": 2},
    ]


def test_analyze_repo_search_supports_score_rank_config(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )
    request = repo_search_request("alpha", limit=2, path_prefixes=("src/",))

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == repo_search_query()
        assert connect_kwargs == {"extra_metadata": repo_search_metadata(request)}
        return pa.Table.from_pylist(
            [
                {
                    "path": "src/main.rs",
                    "score": 0.3,
                },
                {
                    "path": "src/lib.rs",
                    "score": 0.9,
                },
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    ranked = analyze_repo_search(
        client,
        request,
        config=AnalyzerConfig(strategy="score_rank"),
    )

    assert [row["path"] for row in ranked] == ["src/lib.rs", "src/main.rs"]
    assert [row["rank"] for row in ranked] == [1, 2]


def test_analyze_query_results_returns_typed_rows(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )
    query = WendaoFlightRouteQuery(route="/rerank/flight")

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == query
        assert connect_kwargs == {}
        return pa.Table.from_pylist(
            [
                {
                    "doc_id": "doc-a",
                    "vector_score": 0.1,
                    "embedding": (1.0, 0.0),
                    "query_embedding": (1.0, 0.0),
                }
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    ranked = analyze_query_results(client, query)

    assert len(ranked) == 1
    assert ranked[0].doc_id == "doc-a"
    assert ranked[0].rank == 1
    assert ranked[0].final_score is not None


def test_analyze_repo_search_results_returns_typed_rows(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )
    request = repo_search_request("alpha", limit=2, path_prefixes=("src/",))

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == repo_search_query()
        assert connect_kwargs == {"extra_metadata": repo_search_metadata(request)}
        return pa.Table.from_pylist(
            [
                {
                    "path": "src/lib.rs",
                    "score": 0.9,
                    "rank": 1,
                }
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    ranked = analyze_repo_search_results(
        client,
        request,
        analyzer=_RepoSearchScoreAnalyzer(),
    )

    assert len(ranked) == 1
    assert ranked[0].path == "src/lib.rs"
    assert ranked[0].score == 0.9
    assert ranked[0].rank == 1


def test_analyze_repo_search_results_supports_score_rank_config(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )
    request = repo_search_request("alpha", limit=2, path_prefixes=("src/",))

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == repo_search_query()
        assert connect_kwargs == {"extra_metadata": repo_search_metadata(request)}
        return pa.Table.from_pylist(
            [
                {"path": "src/main.rs", "score": 0.3},
                {"path": "src/lib.rs", "score": 0.9},
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    ranked = analyze_repo_search_results(
        client,
        request,
        config=AnalyzerConfig(strategy="score_rank"),
    )

    assert [row.path for row in ranked] == ["src/lib.rs", "src/main.rs"]
    assert [row.rank for row in ranked] == [1, 2]


def test_analyze_repo_query_text_builds_request_and_applies_score_rank(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == repo_search_query()
        assert connect_kwargs == {
            "extra_metadata": repo_search_metadata(
                repo_search_request("alpha", limit=2, path_prefixes=("src/",))
            )
        }
        return pa.Table.from_pylist(
            [
                {"path": "src/main.rs", "score": 0.3},
                {"path": "src/lib.rs", "score": 0.9},
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    ranked = analyze_repo_query_text(
        client,
        "alpha",
        limit=2,
        path_prefixes=("src/",),
        config=AnalyzerConfig(strategy="score_rank"),
    )

    assert [row["path"] for row in ranked] == ["src/lib.rs", "src/main.rs"]
    assert [row["rank"] for row in ranked] == [1, 2]


def test_analyze_repo_query_text_results_return_typed_rows(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == repo_search_query()
        assert connect_kwargs == {
            "extra_metadata": repo_search_metadata(
                repo_search_request("alpha", limit=2, path_prefixes=("src/",))
            )
        }
        return pa.Table.from_pylist(
            [
                {"path": "src/main.rs", "score": 0.3},
                {"path": "src/lib.rs", "score": 0.9},
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    ranked = analyze_repo_query_text_results(
        client,
        "alpha",
        limit=2,
        path_prefixes=("src/",),
        config=AnalyzerConfig(strategy="score_rank"),
    )

    assert [row.path for row in ranked] == ["src/lib.rs", "src/main.rs"]
    assert [row.rank for row in ranked] == [1, 2]


def test_run_repo_analysis_returns_request_and_typed_rows(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == repo_search_query()
        assert connect_kwargs == {
            "extra_metadata": repo_search_metadata(
                repo_search_request("alpha", limit=2, path_prefixes=("src/",))
            )
        }
        return pa.Table.from_pylist(
            [
                {"path": "src/main.rs", "score": 0.3},
                {"path": "src/lib.rs", "score": 0.9},
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    run = run_repo_analysis(
        client,
        "alpha",
        limit=2,
        path_prefixes=("src/",),
        config=AnalyzerConfig(strategy="score_rank"),
    )

    assert run.request.query_text == "alpha"
    assert run.request.path_prefixes == ("src/",)
    assert [row.path for row in run.rows] == ["src/lib.rs", "src/main.rs"]
    assert [row.rank for row in run.rows] == [1, 2]


def test_run_repo_search_analysis_preserves_typed_request_and_rows(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )
    request = repo_search_request("alpha", limit=2, path_prefixes=("src/",))

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == repo_search_query()
        assert connect_kwargs == {"extra_metadata": repo_search_metadata(request)}
        return pa.Table.from_pylist(
            [
                {"path": "src/main.rs", "score": 0.3},
                {"path": "src/lib.rs", "score": 0.9},
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    run = run_repo_search_analysis(
        client,
        request,
        config=AnalyzerConfig(strategy="score_rank"),
    )

    assert run.request == request
    assert [row.path for row in run.rows] == ["src/lib.rs", "src/main.rs"]
    assert [row.rank for row in run.rows] == [1, 2]


def test_summarize_repo_analysis_returns_top_row_snapshot(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == repo_search_query()
        assert connect_kwargs == {
            "extra_metadata": repo_search_metadata(
                repo_search_request("alpha", limit=2, path_prefixes=("src/",))
            )
        }
        return pa.Table.from_pylist(
            [
                {"path": "src/main.rs", "score": 0.3},
                {"path": "src/lib.rs", "score": 0.9},
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    run = run_repo_analysis(
        client,
        "alpha",
        limit=2,
        path_prefixes=("src/",),
        config=AnalyzerConfig(strategy="score_rank"),
    )
    summary = summarize_repo_analysis(run)

    assert summary.row_count == 2
    assert summary.top_path == "src/lib.rs"
    assert summary.top_rank == 1


def test_summarize_repo_search_returns_top_row_snapshot(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )
    request = repo_search_request("alpha", limit=2, path_prefixes=("src/",))

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == repo_search_query()
        assert connect_kwargs == {"extra_metadata": repo_search_metadata(request)}
        return pa.Table.from_pylist(
            [
                {"path": "src/main.rs", "score": 0.3},
                {"path": "src/lib.rs", "score": 0.9},
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    summary = summarize_repo_search(
        client,
        request,
        config=AnalyzerConfig(strategy="score_rank"),
    )

    assert summary.row_count == 2
    assert summary.top_path == "src/lib.rs"
    assert summary.top_rank == 1


def test_summarize_repo_search_results_returns_top_row_snapshot(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )
    request = repo_search_request("alpha", limit=2, path_prefixes=("src/",))

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == repo_search_query()
        assert connect_kwargs == {"extra_metadata": repo_search_metadata(request)}
        return pa.Table.from_pylist(
            [
                {"path": "src/main.rs", "score": 0.3},
                {"path": "src/lib.rs", "score": 0.9},
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    summary = summarize_repo_search_results(
        client,
        request,
        config=AnalyzerConfig(strategy="score_rank"),
    )

    assert summary.row_count == 2
    assert summary.top_path == "src/lib.rs"
    assert summary.top_rank == 1


def test_summarize_repo_query_text_returns_top_row_snapshot(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == repo_search_query()
        assert connect_kwargs == {
            "extra_metadata": repo_search_metadata(
                repo_search_request("alpha", limit=2, path_prefixes=("src/",))
            )
        }
        return pa.Table.from_pylist(
            [
                {"path": "src/main.rs", "score": 0.3},
                {"path": "src/lib.rs", "score": 0.9},
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    summary = summarize_repo_query_text(
        client,
        "alpha",
        limit=2,
        path_prefixes=("src/",),
        config=AnalyzerConfig(strategy="score_rank"),
    )

    assert summary.row_count == 2
    assert summary.top_path == "src/lib.rs"
    assert summary.top_rank == 1


def test_summarize_repo_query_text_results_returns_top_row_snapshot(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )

    def _fake_read_query_table(self, query_arg, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert query_arg == repo_search_query()
        assert connect_kwargs == {
            "extra_metadata": repo_search_metadata(
                repo_search_request("alpha", limit=2, path_prefixes=("src/",))
            )
        }
        return pa.Table.from_pylist(
            [
                {"path": "src/main.rs", "score": 0.3},
                {"path": "src/lib.rs", "score": 0.9},
            ]
        )

    monkeypatch.setattr(WendaoTransportClient, "read_query_table", _fake_read_query_table)

    summary = summarize_repo_query_text_results(
        client,
        "alpha",
        limit=2,
        path_prefixes=("src/",),
        config=AnalyzerConfig(strategy="score_rank"),
    )

    assert summary.row_count == 2
    assert summary.top_path == "src/lib.rs"
    assert summary.top_rank == 1


def _project_root() -> str:
    project_root = os.environ.get("PRJ_ROOT")
    if not project_root:
        pytest.skip("set PRJ_ROOT before running analyzer real-host integration tests")
    return project_root


def _wendao_search_flight_server_binary() -> str:
    return os.path.join(
        _project_root(),
        ".cache",
        "pyflight-f56-target",
        "debug",
        "wendao_search_flight_server",
    )


def _wendao_search_seed_binary() -> str:
    return os.path.join(
        _project_root(),
        ".cache",
        "pyflight-f56-target",
        "debug",
        "wendao_search_seed_sample",
    )


def _wendao_runtime_flight_server_binary() -> str:
    return os.path.join(
        _project_root(),
        ".cache",
        "pyflight-rust-contract-target",
        "debug",
        "wendao_flight_server",
    )


def _run_rust_search_plane_seed_binary(project_root: str, *, repo_id: str = "alpha/repo") -> None:
    binary = os.environ.get("WENDAO_SEARCH_SEED_BINARY", _wendao_search_seed_binary())
    if not os.path.exists(binary):
        pytest.skip(f"build {binary} before running analyzer real-host integration tests")

    result = subprocess.run(
        [binary, repo_id, project_root],
        cwd=_project_root(),
        text=True,
        capture_output=True,
        check=False,
    )
    if result.returncode != 0:
        raise AssertionError(
            "Wendao search-plane seed binary failed:\n"
            f"stdout:\n{result.stdout}\n"
            f"stderr:\n{result.stderr}"
        )


def _spawn_wendao_search_flight_server(
    host: str, port: int, project_root: str
) -> subprocess.Popen[str]:
    binary = os.environ.get("WENDAO_SEARCH_SERVER_BINARY", _wendao_search_flight_server_binary())
    if not os.path.exists(binary):
        pytest.skip(f"build {binary} before running analyzer real-host integration tests")

    process = subprocess.Popen(
        [
            binary,
            f"{host}:{port}",
            "--schema-version=v2",
            "alpha/repo",
            project_root,
            "3",
        ],
        cwd=project_root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env={**os.environ, "PRJ_ROOT": project_root},
    )
    ready_line = ""
    deadline = time.time() + 120
    while time.time() < deadline:
        line = process.stdout.readline() if process.stdout is not None else ""
        if line.startswith("READY http://"):
            ready_line = line.strip()
            break
        if process.poll() is not None:
            stderr = process.stderr.read() if process.stderr is not None else ""
            raise AssertionError(f"Wendao search Flight server exited before readiness:\n{stderr}")
    if not ready_line:
        raise AssertionError("timed out waiting for Wendao search Flight server readiness")
    time.sleep(1.0)
    return process


def _spawn_wendao_runtime_flight_server(host: str, port: int) -> subprocess.Popen[str]:
    binary = os.environ.get("WENDAO_RUNTIME_SERVER_BINARY", _wendao_runtime_flight_server_binary())
    if not os.path.exists(binary):
        pytest.skip(f"build {binary} before running analyzer real-host integration tests")

    process = subprocess.Popen(
        [
            binary,
            f"{host}:{port}",
            "--schema-version=v2",
            "--rerank-dimension=3",
        ],
        cwd=_project_root(),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=os.environ.copy(),
    )
    ready_line = ""
    deadline = time.time() + 120
    while time.time() < deadline:
        line = process.stdout.readline() if process.stdout is not None else ""
        if line.startswith("READY http://"):
            ready_line = line.strip()
            break
        if process.poll() is not None:
            stderr = process.stderr.read() if process.stderr is not None else ""
            raise AssertionError(f"Wendao runtime Flight server exited before readiness:\n{stderr}")
    if not ready_line:
        raise AssertionError("timed out waiting for Wendao runtime Flight server readiness")
    time.sleep(1.0)
    return process


def _terminate_process(process: subprocess.Popen[str]) -> None:
    process.terminate()
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=10)


class _RepoSearchScoreAnalyzer:
    def analyze_rows(self, rows: list[dict[str, object]]) -> list[dict[str, object]]:
        ranked = sorted(rows, key=lambda row: float(row["score"]), reverse=True)
        return [
            {
                "path": str(row["path"]),
                "score": float(row["score"]),
                "rank": index + 1,
            }
            for index, row in enumerate(ranked)
        ]


def test_run_rerank_exchange_analysis_uses_transport_exchange(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )
    rows = [
        WendaoRerankRequestRow(
            doc_id="doc-a",
            vector_score=0.2,
            embedding=(1.0, 0.0),
            query_embedding=(1.0, 0.0),
        ),
        WendaoRerankRequestRow(
            doc_id="doc-b",
            vector_score=0.9,
            embedding=(0.0, 1.0),
            query_embedding=(1.0, 0.0),
        ),
    ]

    def _fake_exchange(self, rows_arg, *, top_k=None, min_final_score=None, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert rows_arg == rows
        assert top_k == 2
        assert min_final_score == 0.6
        assert connect_kwargs == {"tls_root_certs": b"roots"}
        return [
            WendaoRerankResultRow(
                doc_id="doc-a",
                vector_score=0.2,
                semantic_score=1.0,
                final_score=0.68,
                rank=1,
            ),
            WendaoRerankResultRow(
                doc_id="doc-b",
                vector_score=0.9,
                semantic_score=0.0,
                final_score=0.36,
                rank=2,
            ),
        ]

    monkeypatch.setattr(WendaoTransportClient, "exchange_rerank_result_rows", _fake_exchange)

    run = run_rerank_exchange_analysis(
        client,
        rows,
        top_k=2,
        min_final_score=0.6,
        tls_root_certs=b"roots",
    )

    assert [row.doc_id for row in run.rows_in] == ["doc-a", "doc-b"]
    assert [row.doc_id for row in run.rows_out] == ["doc-a", "doc-b"]
    assert [row.rank for row in run.rows_out] == [1, 2]
    assert [row.final_score for row in run.rows_out] == pytest.approx([0.68, 0.36])


def test_summarize_rerank_exchange_returns_top_runtime_row(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=8815),
        )
    )
    rows = [
        WendaoRerankRequestRow(
            doc_id="doc-a",
            vector_score=0.2,
            embedding=(1.0, 0.0),
            query_embedding=(1.0, 0.0),
        ),
        WendaoRerankRequestRow(
            doc_id="doc-b",
            vector_score=0.9,
            embedding=(0.0, 1.0),
            query_embedding=(1.0, 0.0),
        ),
    ]

    def _fake_exchange(self, rows_arg, *, top_k=None, min_final_score=None, **connect_kwargs):  # type: ignore[no-untyped-def]
        assert rows_arg == rows
        assert top_k == 2
        assert min_final_score is None
        assert connect_kwargs == {}
        return [
            WendaoRerankResultRow(
                doc_id="doc-a",
                vector_score=0.2,
                semantic_score=1.0,
                final_score=0.68,
                rank=1,
            ),
            WendaoRerankResultRow(
                doc_id="doc-b",
                vector_score=0.9,
                semantic_score=0.0,
                final_score=0.36,
                rank=2,
            ),
        ]

    monkeypatch.setattr(WendaoTransportClient, "exchange_rerank_result_rows", _fake_exchange)

    summary = summarize_rerank_exchange(client, rows, top_k=2)

    assert summary.row_count == 2
    assert summary.top_doc_id == "doc-a"
    assert summary.top_rank == 1
    assert summary.top_final_score == pytest.approx(0.68)


@pytest.mark.integration
def test_transport_substrate_exchanges_rerank_rows_via_runtime_wendao_flight_server() -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_wendao_runtime_flight_server(host, port)
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        rows = client.exchange_rerank_result_rows(
            [
                WendaoRerankRequestRow(
                    doc_id="doc-0",
                    vector_score=0.5,
                    embedding=(1.0, 0.0, 0.0),
                    query_embedding=(1.0, 0.0, 0.0),
                ),
                WendaoRerankRequestRow(
                    doc_id="doc-1",
                    vector_score=0.8,
                    embedding=(0.0, 1.0, 0.0),
                    query_embedding=(1.0, 0.0, 0.0),
                ),
            ],
            top_k=2,
        )

        assert [row.doc_id for row in rows] == ["doc-0", "doc-1"]
        assert [row.rank for row in rows] == [1, 2]
        assert [row.final_score for row in rows] == pytest.approx([0.8, 0.62])
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_run_rerank_exchange_analysis_reads_rows_via_runtime_wendao_flight_server() -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_wendao_runtime_flight_server(host, port)
    rows = [
        WendaoRerankRequestRow(
            doc_id="doc-0",
            vector_score=0.5,
            embedding=(1.0, 0.0, 0.0),
            query_embedding=(1.0, 0.0, 0.0),
        ),
        WendaoRerankRequestRow(
            doc_id="doc-1",
            vector_score=0.8,
            embedding=(0.0, 1.0, 0.0),
            query_embedding=(1.0, 0.0, 0.0),
        ),
    ]
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        run = run_rerank_exchange_analysis(client, rows, top_k=2)
        summary = summarize_rerank_exchange(client, rows, top_k=2)

        assert list(run.rows_in) == rows
        assert [row.doc_id for row in run.rows_out] == ["doc-0", "doc-1"]
        assert [row.rank for row in run.rows_out] == [1, 2]
        assert [row.final_score for row in run.rows_out] == pytest.approx([0.8, 0.62])
        assert summary.row_count == 2
        assert summary.top_doc_id == "doc-0"
        assert summary.top_rank == 1
        assert summary.top_final_score == pytest.approx(0.8)
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_run_query_analysis_reads_repo_search_rows_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_wendao_search_flight_server(host, port, str(tmp_path))
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        query = repo_search_query()
        run = run_query_analysis(
            client,
            query,
            analyzer=_RepoSearchScoreAnalyzer(),
            extra_metadata=repo_search_metadata(
                repo_search_request("alpha", limit=3, path_prefixes=("src/",))
            ),
        )

        assert run.query == query
        assert run.rows
        assert all((row.path or "").startswith("src/") for row in run.rows)
        assert [row.rank for row in run.rows] == list(range(1, len(run.rows) + 1))
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_analyze_query_reads_repo_search_rows_via_wendao_search_flight_server(tmp_path) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_wendao_search_flight_server(host, port, str(tmp_path))
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        request = repo_search_request("alpha", limit=3, path_prefixes=("src/",))

        ranked = analyze_query(
            client,
            repo_search_query(),
            analyzer=_RepoSearchScoreAnalyzer(),
            extra_metadata=repo_search_metadata(request),
        )

        assert ranked
        assert all(str(row["path"]).startswith("src/") for row in ranked)
        assert [int(row["rank"]) for row in ranked] == list(range(1, len(ranked) + 1))
        assert [float(row["score"]) for row in ranked] == sorted(
            [float(row["score"]) for row in ranked],
            reverse=True,
        )
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_summarize_query_reads_repo_search_rows_via_wendao_search_flight_server(tmp_path) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_wendao_search_flight_server(host, port, str(tmp_path))
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        summary = summarize_query(
            run_query_analysis(
                client,
                repo_search_query(),
                analyzer=_RepoSearchScoreAnalyzer(),
                extra_metadata=repo_search_metadata(
                    repo_search_request("alpha", limit=3, path_prefixes=("src/",))
                ),
            )
        )

        assert summary.row_count >= 1
        assert summary.top_path is not None
        assert summary.top_path.startswith("src/")
        assert summary.top_rank == 1
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_summarize_query_route_reads_repo_search_rows_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_wendao_search_flight_server(host, port, str(tmp_path))
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        summary = summarize_query_route(
            client,
            repo_search_query(),
            analyzer=_RepoSearchScoreAnalyzer(),
            extra_metadata=repo_search_metadata(
                repo_search_request("alpha", limit=3, path_prefixes=("src/",))
            ),
        )

        assert summary.row_count >= 1
        assert summary.top_path is not None
        assert summary.top_path.startswith("src/")
        assert summary.top_rank == 1
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_summarize_query_results_reads_repo_search_rows_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_wendao_search_flight_server(host, port, str(tmp_path))
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        summary = summarize_query_results(
            client,
            repo_search_query(),
            analyzer=_RepoSearchScoreAnalyzer(),
            extra_metadata=repo_search_metadata(
                repo_search_request("alpha", limit=3, path_prefixes=("src/",))
            ),
        )

        assert summary.row_count >= 1
        assert summary.top_path is not None
        assert summary.top_path.startswith("src/")
        assert summary.top_rank == 1
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_analyze_repo_search_reads_rows_via_wendao_search_flight_server(tmp_path) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_wendao_search_flight_server(host, port, str(tmp_path))
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        ranked = analyze_repo_search(
            client,
            repo_search_request("alpha", limit=3, path_prefixes=("src/",)),
            analyzer=_RepoSearchScoreAnalyzer(),
        )

        assert ranked
        assert all(str(row["path"]).startswith("src/") for row in ranked)
        assert [int(row["rank"]) for row in ranked] == list(range(1, len(ranked) + 1))
        assert [float(row["score"]) for row in ranked] == sorted(
            [float(row["score"]) for row in ranked],
            reverse=True,
        )
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_summarize_repo_search_results_reads_rows_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_wendao_search_flight_server(host, port, str(tmp_path))
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        summary = summarize_repo_search_results(
            client,
            repo_search_request("alpha", limit=3, path_prefixes=("src/",)),
            config=AnalyzerConfig(strategy="score_rank"),
        )

        assert summary.row_count >= 1
        assert summary.top_path is not None
        assert summary.top_path.startswith("src/")
        assert summary.top_rank == 1
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_analyze_repo_query_text_reads_rows_via_wendao_search_flight_server(tmp_path) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_wendao_search_flight_server(host, port, str(tmp_path))
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        ranked = analyze_repo_query_text(
            client,
            "alpha",
            limit=3,
            path_prefixes=("src/",),
            config=AnalyzerConfig(strategy="score_rank"),
        )

        assert ranked
        assert all(str(row["path"]).startswith("src/") for row in ranked)
        assert [int(row["rank"]) for row in ranked] == list(range(1, len(ranked) + 1))
        assert [float(row["score"]) for row in ranked] == sorted(
            [float(row["score"]) for row in ranked],
            reverse=True,
        )
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_summarize_repo_query_text_results_reads_rows_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_wendao_search_flight_server(host, port, str(tmp_path))
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        summary = summarize_repo_query_text_results(
            client,
            "alpha",
            limit=3,
            path_prefixes=("src/",),
            config=AnalyzerConfig(strategy="score_rank"),
        )

        assert summary.row_count >= 1
        assert summary.top_path is not None
        assert summary.top_path.startswith("src/")
        assert summary.top_rank == 1
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_run_repo_analysis_reads_rows_via_wendao_search_flight_server(tmp_path) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_wendao_search_flight_server(host, port, str(tmp_path))
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        run = run_repo_analysis(
            client,
            "alpha",
            limit=3,
            path_prefixes=("src/",),
            config=AnalyzerConfig(strategy="score_rank"),
        )

        assert run.request.query_text == "alpha"
        assert run.rows
        assert all((row.path or "").startswith("src/") for row in run.rows)
        assert [row.rank for row in run.rows] == list(range(1, len(run.rows) + 1))
        assert [row.score for row in run.rows] == sorted(
            [row.score for row in run.rows if row.score is not None],
            reverse=True,
        )
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_run_repo_search_analysis_reads_rows_via_wendao_search_flight_server(tmp_path) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_wendao_search_flight_server(host, port, str(tmp_path))
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        run = run_repo_search_analysis(
            client,
            repo_search_request("alpha", limit=3, path_prefixes=("src/",)),
            config=AnalyzerConfig(strategy="score_rank"),
        )

        assert run.request.query_text == "alpha"
        assert run.request.path_prefixes == ("src/",)
        assert run.rows
        assert all((row.path or "").startswith("src/") for row in run.rows)
        assert [row.rank for row in run.rows] == list(range(1, len(run.rows) + 1))
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_summarize_repo_analysis_reads_rows_via_wendao_search_flight_server(tmp_path) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_wendao_search_flight_server(host, port, str(tmp_path))
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        run = run_repo_analysis(
            client,
            "alpha",
            limit=3,
            path_prefixes=("src/",),
            config=AnalyzerConfig(strategy="score_rank"),
        )
        summary = summarize_repo_analysis(run)

        assert summary.row_count == len(run.rows)
        assert summary.top_path is not None
        assert summary.top_path.startswith("src/")
        assert summary.top_rank == 1
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_repo_search_v1_workflow_reads_run_and_summary_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_wendao_search_flight_server(host, port, str(tmp_path))
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        run = run_repo_analysis(
            client,
            "alpha",
            limit=3,
            path_prefixes=("src/",),
            config=AnalyzerConfig(strategy="score_rank"),
        )
        summary = summarize_repo_analysis(run)

        assert run.request.query_text == "alpha"
        assert run.request.path_prefixes == ("src/",)
        assert run.rows
        assert summary.row_count == len(run.rows)
        assert summary.top_path == run.rows[0].path
        assert summary.top_rank == run.rows[0].rank == 1
        assert summary.top_path is not None
        assert summary.top_path.startswith("src/")
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_custom_repo_analyzer_workflow_reads_run_and_summary_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_wendao_search_flight_server(host, port, str(tmp_path))
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        run = run_repo_analysis(
            client,
            "alpha",
            limit=3,
            path_prefixes=("src/",),
            analyzer=_RepoSearchScoreAnalyzer(),
        )
        summary = summarize_repo_analysis(run)

        assert run.request.query_text == "alpha"
        assert run.rows
        assert summary.row_count == len(run.rows)
        assert summary.top_path == run.rows[0].path
        assert summary.top_rank == run.rows[0].rank == 1
        assert summary.top_path is not None
        assert summary.top_path.startswith("src/")
        assert [row.score for row in run.rows] == sorted(
            [row.score for row in run.rows if row.score is not None],
            reverse=True,
        )
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_summarize_repo_search_reads_rows_via_wendao_search_flight_server(tmp_path) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_wendao_search_flight_server(host, port, str(tmp_path))
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        summary = summarize_repo_search(
            client,
            repo_search_request("alpha", limit=3, path_prefixes=("src/",)),
            config=AnalyzerConfig(strategy="score_rank"),
        )

        assert summary.row_count >= 1
        assert summary.top_path is not None
        assert summary.top_path.startswith("src/")
        assert summary.top_rank == 1
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_summarize_repo_query_text_reads_rows_via_wendao_search_flight_server(tmp_path) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_wendao_search_flight_server(host, port, str(tmp_path))
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        summary = summarize_repo_query_text(
            client,
            "alpha",
            limit=3,
            path_prefixes=("src/",),
            config=AnalyzerConfig(strategy="score_rank"),
        )

        assert summary.row_count >= 1
        assert summary.top_path is not None
        assert summary.top_path.startswith("src/")
        assert summary.top_rank == 1
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_analyze_repo_search_reads_rows_via_wendao_search_flight_server_with_score_rank_config(
    tmp_path,
) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_wendao_search_flight_server(host, port, str(tmp_path))
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        ranked = analyze_repo_search(
            client,
            repo_search_request("alpha", limit=3, path_prefixes=("src/",)),
            config=AnalyzerConfig(strategy="score_rank"),
        )

        assert ranked
        assert all(str(row["path"]).startswith("src/") for row in ranked)
        assert [int(row["rank"]) for row in ranked] == list(range(1, len(ranked) + 1))
        assert [float(row["score"]) for row in ranked] == sorted(
            [float(row["score"]) for row in ranked],
            reverse=True,
        )
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_analyze_repo_search_results_reads_rows_via_wendao_search_flight_server(tmp_path) -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_wendao_search_flight_server(host, port, str(tmp_path))
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        ranked = analyze_repo_search_results(
            client,
            repo_search_request("alpha", limit=3, path_prefixes=("src/",)),
            analyzer=_RepoSearchScoreAnalyzer(),
        )

        assert ranked
        assert all((row.path or "").startswith("src/") for row in ranked)
        assert [row.rank for row in ranked] == list(range(1, len(ranked) + 1))
        assert [row.score for row in ranked] == sorted(
            [row.score for row in ranked if row.score is not None],
            reverse=True,
        )
    finally:
        _terminate_process(process)
