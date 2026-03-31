from __future__ import annotations

import pyarrow as pa

from xiuxian_wendao_py import WendaoAnalyzerPlugin
from xiuxian_wendao_py.plugin import (
    PluginTransportKind,
    build_arrow_flight_binding,
    build_profiled_analyzer_plugin,
    build_starter_payload,
    load_manifest_entrypoint,
    load_plugin_manifest,
    plugin_from_manifest,
    starter_from_manifest,
    validate_plugin_manifest,
)
from xiuxian_wendao_py.scaffold import (
    WendaoAnalyzerProfile,
    WendaoSampleTemplate,
    write_scaffold_project,
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
        starter={
            "profile": "repo_search",
            "sample_template": "repo_search",
            "display_name": "Repo Search Analyzer",
        },
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
    assert binding.starter == {
        "profile": "repo_search",
        "sample_template": "repo_search",
        "display_name": "Repo Search Analyzer",
    }
    assert binding.to_dict()["transport"] == "arrow_flight"
    assert binding.to_dict()["starter"]["profile"] == "repo_search"


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
        starter={
            "profile": "repo_search",
            "sample_template": "repo_search",
            "display_name": "Repo Search Analyzer",
        },
        analyzer=lambda table, context: {
            "rows": table.num_rows,
            "route": context.query.normalized_route(),
        },
    )

    binding = plugin.binding_for_client(client, query)
    result = plugin.run(client, query)

    assert binding.endpoint.route == "/search/repos/main"
    assert binding.transport is PluginTransportKind.ARROW_FLIGHT
    assert binding.starter == {
        "profile": "repo_search",
        "sample_template": "repo_search",
        "display_name": "Repo Search Analyzer",
    }
    assert result == {"rows": 1, "route": "/search/repos/main"}
    assert captured["query"] == query


def test_build_starter_payload_uses_profile_defaults() -> None:
    starter = build_starter_payload(WendaoAnalyzerProfile.REPO_SEARCH)

    assert starter == {
        "profile": "repo_search",
        "sample_template": "repo_search",
        "display_name": "Repo Search Analyzer",
        "summary": "Analyze repository search rows from Wendao Arrow Flight queries.",
        "tags": ("repo_search", "search", "arrow_flight"),
    }


def test_build_profiled_analyzer_plugin_links_starter_payload(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
        )
    )
    query = WendaoFlightRouteQuery(route="/search/symbols/main")
    expected = pa.table({"id": ["symbol-1"], "score": [0.9]})

    def fake_get_query_info(self, resolved_query, **kwargs):  # type: ignore[no-untyped-def]
        return {"route": resolved_query.normalized_route()}

    def fake_read_query_table(self, resolved_query, **kwargs):  # type: ignore[no-untyped-def]
        return expected

    monkeypatch.setattr(WendaoTransportClient, "get_query_info", fake_get_query_info)
    monkeypatch.setattr(WendaoTransportClient, "read_query_table", fake_read_query_table)

    plugin = build_profiled_analyzer_plugin(
        capability_id="code_symbol",
        provider="acme.python.analyzer",
        profile=WendaoAnalyzerProfile.CODE_SYMBOL,
        sample_template=WendaoSampleTemplate.CODE_SYMBOL,
        analyzer=lambda table, context: {
            "rows": table.num_rows,
            "route": context.query.normalized_route(),
        },
    )

    binding = plugin.binding_for_client(client, query)
    result = plugin.run(client, query)

    assert binding.starter == {
        "profile": "code_symbol",
        "sample_template": "code_symbol",
        "display_name": "Code Symbol Analyzer",
        "summary": "Analyze code-symbol rows from Wendao Arrow Flight queries.",
        "tags": ("code_symbol", "symbols", "arrow_flight"),
    }
    assert result == {"rows": 1, "route": "/search/symbols/main"}


def test_starter_from_manifest_extracts_starter_block() -> None:
    starter = starter_from_manifest(
        {
            "plugin": {
                "capability_id": "repo_search",
                "provider": "acme.python.analyzer",
                "route": "/search/repos/main",
                "transport": "arrow_flight",
            },
            "entrypoint": {
                "module": "analyzer",
                "callable": "analyze",
            },
            "starter": {
                "profile": "repo_search",
                "sample_template": "repo_search",
                "display_name": "Repo Search Analyzer",
            },
        }
    )

    assert starter == {
        "profile": "repo_search",
        "sample_template": "repo_search",
        "display_name": "Repo Search Analyzer",
    }


def test_validate_plugin_manifest_accepts_scaffold_shape() -> None:
    validated = validate_plugin_manifest(
        {
            "plugin": {
                "capability_id": "repo_search",
                "provider": "acme.python.analyzer",
                "route": "/search/repos/main",
                "transport": "arrow_flight",
                "health_route": "/healthz",
            },
            "entrypoint": {
                "module": "analyzer",
                "callable": "analyze",
            },
            "starter": {
                "profile": "repo_search",
                "sample_template": "repo_search",
                "display_name": "Repo Search Analyzer",
                "summary": "Analyze repository search rows from Wendao Arrow Flight queries.",
                "tags": ["repo_search", "search", "arrow_flight"],
            },
        }
    )

    assert validated["plugin"]["transport"] == "arrow_flight"


def test_validate_plugin_manifest_rejects_unknown_transport() -> None:
    try:
        validate_plugin_manifest(
            {
                "plugin": {
                    "capability_id": "repo_search",
                    "provider": "acme.python.analyzer",
                    "route": "/search/repos/main",
                    "transport": "grpc",
                },
                "entrypoint": {
                    "module": "analyzer",
                    "callable": "analyze",
                },
            }
        )
    except ValueError as exc:
        assert "unsupported plugin transport" in str(exc)
    else:
        raise AssertionError("expected ValueError")


def test_validate_plugin_manifest_rejects_missing_entrypoint() -> None:
    try:
        validate_plugin_manifest(
            {
                "plugin": {
                    "capability_id": "repo_search",
                    "provider": "acme.python.analyzer",
                    "route": "/search/repos/main",
                    "transport": "arrow_flight",
                },
            }
        )
    except TypeError as exc:
        assert "[entrypoint]" in str(exc)
    else:
        raise AssertionError("expected TypeError")


def test_plugin_from_manifest_builds_runtime_plugin(tmp_path, monkeypatch) -> None:
    manifest_path = tmp_path / "plugin.toml"
    manifest_path.write_text(
        "\n".join(
            [
                "[plugin]",
                'plugin_id = "acme.repo_search"',
                'capability_id = "repo_search"',
                'provider = "acme.python.analyzer"',
                'transport = "arrow_flight"',
                'route = "/search/repos/main"',
                'health_route = "/healthz"',
                "",
                "[entrypoint]",
                'module = "analyzer"',
                'callable = "analyze"',
                "",
                "[starter]",
                'profile = "repo_search"',
                'sample_template = "repo_search"',
                'display_name = "Repo Search Analyzer"',
                "",
            ]
        )
        + "\n",
        encoding="utf-8",
    )
    loaded = load_plugin_manifest(manifest_path)
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
        )
    )
    query = WendaoFlightRouteQuery(route="/search/repos/main")
    expected = pa.table({"id": ["doc-1"], "score": [0.9]})

    def fake_get_query_info(self, resolved_query, **kwargs):  # type: ignore[no-untyped-def]
        return {"route": resolved_query.normalized_route()}

    def fake_read_query_table(self, resolved_query, **kwargs):  # type: ignore[no-untyped-def]
        return expected

    monkeypatch.setattr(WendaoTransportClient, "get_query_info", fake_get_query_info)
    monkeypatch.setattr(WendaoTransportClient, "read_query_table", fake_read_query_table)

    plugin = plugin_from_manifest(
        loaded,
        analyzer=lambda table, context: {
            "rows": table.num_rows,
            "route": context.query.normalized_route(),
        },
    )

    binding = plugin.binding_for_client(client, query)
    result = plugin.run(client, query)

    assert plugin.capability_id == "repo_search"
    assert plugin.provider == "acme.python.analyzer"
    assert plugin.health_route == "/healthz"
    assert binding.starter == {
        "profile": "repo_search",
        "sample_template": "repo_search",
        "display_name": "Repo Search Analyzer",
    }
    assert result == {"rows": 1, "route": "/search/repos/main"}


def test_load_manifest_entrypoint_and_plugin_from_scaffolded_project(tmp_path, monkeypatch) -> None:
    project_root = tmp_path / "acme-repo-search"
    write_scaffold_project(
        project_root,
        package_name="acme-repo-search",
        plugin_id="acme.repo_search",
        capability_id="repo_search",
        provider="acme.python.analyzer",
        route="/search/repos/main",
        profile=WendaoAnalyzerProfile.REPO_SEARCH,
        sample_template=WendaoSampleTemplate.REPO_SEARCH,
    )
    manifest_path = project_root / "plugin.toml"
    loaded = load_plugin_manifest(manifest_path)
    entrypoint = load_manifest_entrypoint(loaded, project_root=project_root)
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
        )
    )
    query = WendaoFlightRouteQuery(route="/search/repos/main")
    expected = pa.table(
        {
            "repo": ["xiuxian-artisan-workshop"],
            "path": ["src/search/router.py"],
            "language": ["python"],
            "score": [0.93],
        }
    )

    def fake_get_query_info(self, resolved_query, **kwargs):  # type: ignore[no-untyped-def]
        return {"route": resolved_query.normalized_route()}

    def fake_read_query_table(self, resolved_query, **kwargs):  # type: ignore[no-untyped-def]
        return expected

    monkeypatch.setattr(WendaoTransportClient, "get_query_info", fake_get_query_info)
    monkeypatch.setattr(WendaoTransportClient, "read_query_table", fake_read_query_table)

    plugin = plugin_from_manifest(
        loaded,
        project_root=project_root,
    )
    binding = plugin.binding_for_client(client, query)
    result = plugin.run(client, query)

    assert callable(entrypoint)
    assert plugin.capability_id == "repo_search"
    assert binding.starter == {
        "profile": "repo_search",
        "sample_template": "repo_search",
        "display_name": "Repo Search Analyzer",
        "summary": "Analyze repository search rows from Wendao Arrow Flight queries.",
        "tags": ["repo_search", "search", "arrow_flight"],
    }
    assert result == {
        "rows": 1,
        "route": "/search/repos/main",
        "repos": ["xiuxian-artisan-workshop"],
        "languages": ["python"],
        "top_paths": ["src/search/router.py"],
    }
