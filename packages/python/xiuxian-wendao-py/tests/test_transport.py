from __future__ import annotations

import os
import socket
import subprocess
import time
from collections.abc import Mapping, Sequence

import pyarrow as pa
import pyarrow.flight as flight
import pytest

from xiuxian_wendao_py.transport import (
    REPO_SEARCH_COLUMNS,
    REPO_SEARCH_HIERARCHY_COLUMN,
    RERANK_RESPONSE_COLUMNS,
    RERANK_RESPONSE_SEMANTIC_SCORE_COLUMN,
    RERANK_RESPONSE_VECTOR_SCORE_COLUMN,
    REPO_SEARCH_MATCH_REASON_COLUMN,
    REPO_SEARCH_NAVIGATION_CATEGORY_COLUMN,
    REPO_SEARCH_NAVIGATION_LINE_COLUMN,
    REPO_SEARCH_NAVIGATION_LINE_END_COLUMN,
    REPO_SEARCH_NAVIGATION_PATH_COLUMN,
    WENDAO_REPO_SEARCH_FILENAME_FILTERS_HEADER,
    WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER,
    WENDAO_RERANK_DIMENSION_HEADER,
    WENDAO_RERANK_TOP_K_HEADER,
    WENDAO_REPO_SEARCH_LIMIT_HEADER,
    WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER,
    WENDAO_REPO_SEARCH_QUERY_HEADER,
    WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER,
    WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER,
    WendaoFlightRouteQuery,
    WendaoRepoSearchRequest,
    WendaoRepoSearchResultRow,
    WendaoRerankRequestRow,
    WendaoRerankResultRow,
    WendaoTransportClient,
    WendaoTransportConfig,
    WendaoTransportEndpoint,
    WendaoTransportMode,
    build_rerank_request_table,
    repo_search_request,
    repo_search_query,
)


def _project_root() -> str:
    project_root = os.environ.get("PRJ_ROOT")
    if not project_root:
        pytest.skip("set PRJ_ROOT before running Wendao Flight integration tests")
    return project_root


def _wendao_search_flight_server_binary() -> str:
    return os.path.join(
        _project_root(),
        ".cache",
        "pyflight-f56-target",
        "debug",
        "wendao_search_flight_server",
    )


def _wendao_runtime_flight_server_binary() -> str:
    return os.path.join(
        _project_root(),
        ".cache",
        "pyflight-rust-contract-target",
        "debug",
        "wendao_flight_server",
    )


def _spawn_rust_mock_flight_server(
    host: str,
    port: int,
    *,
    binary_env_var: str = "WENDAO_MOCK_SERVER_BINARY",
    default_binary: str | None = None,
    schema_version: str | None = "v2",
    schema_version_uses_flag: bool = False,
    rerank_dimension: int | None = None,
    rerank_dimension_uses_flag: bool = False,
    extra_args: Sequence[str] = (),
    extra_env: Mapping[str, str] | None = None,
    cwd: str | None = None,
) -> subprocess.Popen[str]:
    repo_root = _project_root()
    cache_target_dir = os.path.join(
        repo_root,
        ".cache",
        "pyflight-rust-contract-target",
    )
    if default_binary is None:
        default_binary = os.path.join(
            cache_target_dir,
            "debug",
            "examples",
            "mock_flight_exchange_server",
        )
    binary = os.environ.get(
        binary_env_var,
        default_binary,
    )
    if not os.path.exists(binary):
        pytest.skip(f"build {binary} before running the Rust Flight integration smoke tests")

    process_env = os.environ.copy()
    if extra_env is not None:
        process_env.update(extra_env)

    command = [binary, f"{host}:{port}"]
    if schema_version is not None:
        if schema_version_uses_flag:
            command.append(f"--schema-version={schema_version}")
        else:
            command.append(schema_version)
    if rerank_dimension is not None:
        if rerank_dimension_uses_flag:
            command.append(f"--rerank-dimension={rerank_dimension}")
        else:
            command.append(str(rerank_dimension))
    process = subprocess.Popen(
        [*command, *extra_args],
        cwd=cwd or repo_root,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env=process_env,
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
            raise AssertionError(f"Rust mock Flight server exited before becoming ready:\n{stderr}")
    if not ready_line:
        raise AssertionError("timed out waiting for Rust mock Flight server readiness")
    time.sleep(1.0)
    return process


def _run_rust_search_plane_seed_binary(
    project_root: str,
    *,
    repo_id: str = "alpha/repo",
    binary_env_var: str = "WENDAO_SEARCH_SEED_BINARY",
) -> None:
    repo_root = _project_root()
    cache_target_dir = os.path.join(
        repo_root,
        ".cache",
        "pyflight-rust-contract-target",
    )
    default_binary = os.path.join(
        cache_target_dir,
        "debug",
        "wendao_search_seed_sample",
    )
    binary = os.environ.get(binary_env_var, default_binary)
    if not os.path.exists(binary):
        pytest.skip(f"build {binary} before running the Wendao search-plane seed smoke")

    result = subprocess.run(
        [binary, repo_id, project_root],
        cwd=repo_root,
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


def _terminate_process(process: subprocess.Popen[str]) -> None:
    process.terminate()
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=10)


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


def test_transport_client_fetches_repo_search_info_with_typed_request(monkeypatch) -> None:
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
            return "repo-search-info"

    monkeypatch.setattr(
        WendaoTransportClient,
        "connect_flight",
        lambda self, **kwargs: FakeClient(),
    )

    info = client.get_repo_search_info(
        WendaoRepoSearchRequest(
            query_text="rerank rust traits",
            limit=25,
            language_filters=("rust", "markdown"),
            path_prefixes=("src/", "README"),
            title_filters=("README", "overview"),
            tag_filters=("code", "lang:rust"),
            filename_filters=("README.md", "lib.rs"),
        )
    )

    assert info == "repo-search-info"
    assert captured["descriptor"].path == [b"search", b"repos", b"main"]
    assert captured["options"].headers == [
        (b"x-wendao-schema-version", b"v1"),
        (b"x-wendao-repo-search-query", b"rerank rust traits"),
        (b"x-wendao-repo-search-limit", b"25"),
        (b"x-wendao-repo-search-language-filters", b"markdown,rust"),
        (b"x-wendao-repo-search-path-prefixes", b"README,src/"),
        (b"x-wendao-repo-search-title-filters", b"README,overview"),
        (b"x-wendao-repo-search-tag-filters", b"code,lang:rust"),
        (b"x-wendao-repo-search-filename-filters", b"README.md,lib.rs"),
    ]


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


def test_transport_client_reads_repo_search_rows(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
        )
    )
    expected = pa.table(
        {
            "doc_id": ["doc-1"],
            "path": ["src/lib.rs"],
            "title": ["Repo Search Result"],
            "best_section": ["7: Repo Search Result section"],
            "match_reason": ["repo_content_search"],
            "navigation_path": ["alpha/repo/src/lib.rs"],
            "navigation_category": ["repo_code"],
            "navigation_line": [7],
            "navigation_line_end": [7],
            "hierarchy": [["src", "lib.rs"]],
            "tags": [["code", "file", "lang:rust"]],
            "score": [0.91],
            "language": ["rust"],
        }
    )

    request = repo_search_request("rerank rust traits")
    captured: dict[str, object] = {}

    def fake_read_repo_search_table(self, request_arg, **kwargs):  # type: ignore[no-untyped-def]
        captured["request"] = request_arg
        return expected

    monkeypatch.setattr(
        WendaoTransportClient, "read_repo_search_table", fake_read_repo_search_table
    )

    rows = client.read_repo_search_rows(request)

    assert rows == [
        WendaoRepoSearchResultRow(
            doc_id="doc-1",
            path="src/lib.rs",
            title="Repo Search Result",
            best_section="7: Repo Search Result section",
            match_reason="repo_content_search",
            navigation_path="alpha/repo/src/lib.rs",
            navigation_category="repo_code",
            navigation_line=7,
            navigation_line_end=7,
            hierarchy=("src", "lib.rs"),
            tags=("code", "file", "lang:rust"),
            score=0.91,
            language="rust",
        )
    ]
    assert rows[0].match_reason == str(expected[REPO_SEARCH_MATCH_REASON_COLUMN][0].as_py())
    assert rows[0].navigation_path == str(expected[REPO_SEARCH_NAVIGATION_PATH_COLUMN][0].as_py())
    assert rows[0].navigation_category == str(
        expected[REPO_SEARCH_NAVIGATION_CATEGORY_COLUMN][0].as_py()
    )
    assert rows[0].navigation_line == int(expected[REPO_SEARCH_NAVIGATION_LINE_COLUMN][0].as_py())
    assert rows[0].navigation_line_end == int(
        expected[REPO_SEARCH_NAVIGATION_LINE_END_COLUMN][0].as_py()
    )
    assert rows[0].hierarchy == tuple(expected[REPO_SEARCH_HIERARCHY_COLUMN][0].as_py())
    assert captured["request"] == request


def test_transport_client_exchanges_typed_rerank_rows(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
        )
    )
    expected = pa.table({"doc_id": ["doc-1"]})
    captured: dict[str, object] = {}

    def fake_exchange_rerank_table(self, table, **kwargs):  # type: ignore[no-untyped-def]
        captured["table"] = table
        captured["kwargs"] = kwargs
        return expected

    monkeypatch.setattr(
        WendaoTransportClient,
        "exchange_rerank_table",
        fake_exchange_rerank_table,
    )

    response = client.exchange_rerank_rows(
        [
            WendaoRerankRequestRow(
                doc_id="doc-0",
                vector_score=0.5,
                embedding=(0.1, 0.2, 0.3),
                query_embedding=(0.4, 0.5, 0.6),
            )
        ],
        tls_root_certs=b"roots",
    )

    assert response == expected
    assert captured["table"].to_pylist() == [
        {
            "doc_id": "doc-0",
            "vector_score": pytest.approx(0.5),
            "embedding": [pytest.approx(0.1), pytest.approx(0.2), pytest.approx(0.3)],
            "query_embedding": [
                pytest.approx(0.4),
                pytest.approx(0.5),
                pytest.approx(0.6),
            ],
        }
    ]
    assert captured["kwargs"] == {
        "tls_root_certs": b"roots",
        "embedding_dimension": 3,
    }
    assert captured["table"].schema.field("embedding").type == pa.list_(pa.float32(), 3)


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


def test_transport_client_exchanges_table_via_do_exchange(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(
                host="127.0.0.1",
                port=50051,
                path="/rerank/flight",
                metadata={"authorization": "Bearer token"},
            ),
            schema_version="v2",
            request_timeout_seconds=7.5,
        )
    )
    request = pa.table({"id": ["doc-0"], "score": [0.5]})
    expected = pa.table({"id": ["doc-1"], "score": [0.9]})
    captured: dict[str, object] = {}

    class FakeWriter:
        def begin(self, schema: pa.Schema) -> None:
            captured["schema"] = schema

        def write_table(self, table: pa.Table) -> None:
            captured["table"] = table

        def done_writing(self) -> None:
            captured["done"] = True

    class FakeReader:
        def read_all(self) -> pa.Table:
            return expected

    class FakeClient:
        def do_exchange(self, descriptor: object, options: object) -> tuple[FakeWriter, FakeReader]:
            captured["descriptor"] = descriptor
            captured["options"] = options
            return FakeWriter(), FakeReader()

    monkeypatch.setattr(
        WendaoTransportClient,
        "connect_flight",
        lambda self, **kwargs: FakeClient(),
    )

    table = client.exchange_table(request)

    assert table == expected
    assert captured["schema"] == request.schema
    assert captured["table"] == request
    assert captured["done"] is True
    assert captured["descriptor"].path == [b"rerank", b"flight"]
    assert captured["options"].timeout == 7.5
    assert captured["options"].headers == [
        (b"x-wendao-schema-version", b"v2"),
        (b"authorization", b"Bearer token"),
    ]


def test_transport_client_exchanges_table_for_typed_query(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
        )
    )
    request = pa.table({"id": ["doc-0"], "score": [0.5]})
    expected = pa.table({"id": ["doc-2"], "score": [0.8]})
    captured: dict[str, object] = {}

    class FakeWriter:
        def begin(self, schema: pa.Schema) -> None:
            captured["schema"] = schema

        def write_table(self, table: pa.Table) -> None:
            captured["table"] = table

    class FakeReader:
        def read_all(self) -> pa.Table:
            return expected

    class FakeClient:
        def do_exchange(self, descriptor: object, options: object) -> tuple[FakeWriter, FakeReader]:
            captured["descriptor"] = descriptor
            captured["options"] = options
            return FakeWriter(), FakeReader()

    monkeypatch.setattr(
        WendaoTransportClient,
        "connect_flight",
        lambda self, **kwargs: FakeClient(),
    )

    table = client.exchange_query_table(
        WendaoFlightRouteQuery(route="/rerank/flight"),
        request,
    )

    assert table == expected
    assert captured["schema"] == request.schema
    assert captured["table"] == request
    assert captured["descriptor"].path == [b"rerank", b"flight"]
    assert captured["options"].headers == [(b"x-wendao-schema-version", b"v1")]


def test_transport_client_parses_typed_rerank_result_rows(monkeypatch) -> None:
    client = WendaoTransportClient(
        WendaoTransportConfig(
            endpoint=WendaoTransportEndpoint(host="127.0.0.1", port=50051),
        )
    )

    monkeypatch.setattr(
        WendaoTransportClient,
        "exchange_rerank_rows",
        lambda self, rows, **kwargs: pa.table(
            {
                "doc_id": ["doc-1"],
                "vector_score": [0.91],
                "semantic_score": [0.95],
                "final_score": [0.97],
                "rank": [1],
            }
        ),
    )

    response = client.exchange_rerank_result_rows(
        [
            WendaoRerankRequestRow(
                doc_id="doc-0",
                vector_score=0.5,
                embedding=(0.1, 0.2, 0.3),
                query_embedding=(0.4, 0.5, 0.6),
            )
        ]
    )

    assert response == [
        WendaoRerankResultRow(
            doc_id="doc-1",
            vector_score=0.91,
            semantic_score=0.95,
            final_score=0.97,
            rank=1,
        )
    ]


@pytest.mark.integration
def test_transport_client_exchanges_table_with_rust_mock_flight_server() -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(host, port)
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(
                    host=host,
                    port=port,
                    path="/rerank/flight",
                ),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        request = pa.table(
            {
                "doc_id": pa.array(["doc-0", "doc-1"], type=pa.string()),
                "vector_score": pa.array([0.5, 0.8], type=pa.float32()),
                "embedding": pa.array(
                    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
                    type=pa.list_(pa.float32(), 3),
                ),
                "query_embedding": pa.array(
                    [[1.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
                    type=pa.list_(pa.float32(), 3),
                ),
            }
        )

        response = client.exchange_query_table(
            WendaoFlightRouteQuery(route="/rerank/flight"),
            request,
            extra_metadata={WENDAO_RERANK_DIMENSION_HEADER: "3"},
        )

        assert tuple(response.column_names) == RERANK_RESPONSE_COLUMNS
        rows = response.to_pylist()
        assert [row["doc_id"] for row in rows] == ["doc-0", "doc-1"]
        assert [row["vector_score"] for row in rows] == pytest.approx([0.5, 0.8])
        assert [row["semantic_score"] for row in rows] == pytest.approx([1.0, 0.5])
        assert [row["rank"] for row in rows] == [1, 2]
        assert [row["final_score"] for row in rows] == pytest.approx([0.8, 0.62])
    finally:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=10)


@pytest.mark.integration
def test_transport_client_exchanges_typed_rerank_rows_with_rust_mock_flight_server() -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(host, port)
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(
                    host=host,
                    port=port,
                ),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )

        response = client.exchange_rerank_rows(
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
            ]
        )

        assert tuple(response.column_names) == RERANK_RESPONSE_COLUMNS
        rows = response.to_pylist()
        assert [row["doc_id"] for row in rows] == ["doc-0", "doc-1"]
        assert [row["vector_score"] for row in rows] == pytest.approx([0.5, 0.8])
        assert [row["semantic_score"] for row in rows] == pytest.approx([1.0, 0.5])
        assert [row["rank"] for row in rows] == [1, 2]
        assert [row["final_score"] for row in rows] == pytest.approx([0.8, 0.62])
    finally:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=10)


@pytest.mark.integration
def test_transport_client_parses_typed_rerank_rows_with_rust_mock_flight_server() -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(host, port)
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(
                    host=host,
                    port=port,
                ),
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
            ]
        )

        assert [row.doc_id for row in rows] == ["doc-0", "doc-1"]
        assert [row.vector_score for row in rows] == pytest.approx([0.5, 0.8])
        assert [row.semantic_score for row in rows] == pytest.approx([1.0, 0.5])
        assert [row.rank for row in rows] == [1, 2]
        assert [row.final_score for row in rows] == pytest.approx([0.8, 0.62])
    finally:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=10)


@pytest.mark.integration
def test_transport_client_rejects_malformed_rerank_request_with_rust_mock_flight_server() -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(host, port)
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(
                    host=host,
                    port=port,
                ),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        malformed = pa.table(
            {
                "doc_id": ["doc-0"],
                "vector_score": pa.array([0.5], type=pa.float64()),
                "embedding": pa.array([[0.1, 0.2, 0.3]], type=pa.list_(pa.float32(), 3)),
                "query_embedding": pa.array(
                    [[0.4, 0.5, 0.6]],
                    type=pa.list_(pa.float32(), 3),
                ),
            }
        )

        with pytest.raises(pa.ArrowInvalid, match="x-wendao-rerank-embedding-dimension"):
            client.exchange_rerank_table(malformed)
    finally:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=10)


@pytest.mark.integration
def test_transport_client_rejects_query_embedding_drift_with_rust_mock_flight_server() -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(host, port)
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(
                    host=host,
                    port=port,
                ),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )

        with pytest.raises(
            pa.ArrowInvalid,
            match="query_embedding` must remain stable across all rows",
        ):
            client.exchange_rerank_rows(
                [
                    WendaoRerankRequestRow(
                        doc_id="doc-0",
                        vector_score=0.5,
                        embedding=(0.1, 0.2, 0.3),
                        query_embedding=(0.4, 0.5, 0.6),
                    ),
                    WendaoRerankRequestRow(
                        doc_id="doc-1",
                        vector_score=0.4,
                        embedding=(0.7, 0.8, 0.9),
                        query_embedding=(1.0, 1.1, 1.2),
                    ),
                ]
            )
    finally:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=10)


@pytest.mark.integration
def test_transport_client_rejects_duplicate_doc_ids_with_rust_mock_flight_server() -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(host, port)
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(
                    host=host,
                    port=port,
                ),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )

        with pytest.raises(
            pa.ArrowInvalid,
            match="doc_id` must be unique across one batch",
        ):
            client.exchange_rerank_rows(
                [
                    WendaoRerankRequestRow(
                        doc_id="doc-0",
                        vector_score=0.5,
                        embedding=(0.1, 0.2, 0.3),
                        query_embedding=(0.4, 0.5, 0.6),
                    ),
                    WendaoRerankRequestRow(
                        doc_id="doc-0",
                        vector_score=0.4,
                        embedding=(0.7, 0.8, 0.9),
                        query_embedding=(0.4, 0.5, 0.6),
                    ),
                ]
            )
    finally:
        process.terminate()
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait(timeout=10)


@pytest.mark.integration
def test_transport_client_reads_query_via_rust_mock_flight_server() -> None:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(host, port)
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(
                    host=host,
                    port=port,
                ),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        query = repo_search_query()
        request = repo_search_request(
            "rerank rust traits",
            limit=25,
            language_filters=("rust",),
            path_prefixes=("src/",),
            title_filters=("Repo Search",),
            tag_filters=("lang:rust",),
        )

        info = client.get_repo_search_info(request)
        response = client.read_query_table(
            query,
            extra_metadata={
                WENDAO_REPO_SEARCH_QUERY_HEADER: request.query_text,
                WENDAO_REPO_SEARCH_LIMIT_HEADER: str(request.limit),
                WENDAO_REPO_SEARCH_LANGUAGE_FILTERS_HEADER: "rust",
                WENDAO_REPO_SEARCH_PATH_PREFIXES_HEADER: "src/",
                WENDAO_REPO_SEARCH_TAG_FILTERS_HEADER: "lang:rust",
                WENDAO_REPO_SEARCH_TITLE_FILTERS_HEADER: "Repo Search",
            },
        )
        rows = client.read_repo_search_rows(request)

        assert info.descriptor.path == [b"search", b"repos", b"main"]
        assert len(info.endpoints) == 1
        assert info.endpoints[0].ticket.ticket == b"/search/repos/main"
        assert tuple(response.column_names) == REPO_SEARCH_COLUMNS
        assert response.to_pylist() == [
            {
                "doc_id": "doc-1",
                "path": "src/lib.rs",
                "title": "Repo Search Result",
                "best_section": "7: Repo Search Result section",
                "match_reason": "repo_content_search",
                "navigation_path": "alpha/repo/src/lib.rs",
                "navigation_category": "repo_code",
                "navigation_line": 7,
                "navigation_line_end": 7,
                "hierarchy": ["src", "lib.rs"],
                "tags": ["code", "file", "lang:rust"],
                "score": 0.91,
                "language": "rust",
            }
        ]
        assert rows == [
            WendaoRepoSearchResultRow(
                doc_id="doc-1",
                path="src/lib.rs",
                title="Repo Search Result",
                best_section="7: Repo Search Result section",
                match_reason="repo_content_search",
                navigation_path="alpha/repo/src/lib.rs",
                navigation_category="repo_code",
                navigation_line=7,
                navigation_line_end=7,
                hierarchy=("src", "lib.rs"),
                tags=("code", "file", "lang:rust"),
                score=0.91,
                language="rust",
            )
        ]
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_reads_query_via_wendao_search_flight_server(tmp_path) -> None:
    binary = _wendao_search_flight_server_binary()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    _run_rust_search_plane_seed_binary(str(tmp_path))
    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_SEARCH_SERVER_BINARY",
        default_binary=binary,
        schema_version="v2",
        schema_version_uses_flag=True,
        extra_args=("alpha/repo", str(tmp_path), "3"),
        cwd=str(tmp_path),
        extra_env={"PRJ_ROOT": str(tmp_path)},
    )
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        alpha_rows = client.read_repo_search_rows(repo_search_request("alpha", limit=10))
        markdown_rows = client.read_repo_search_rows(
            repo_search_request("alpha", limit=10, language_filters=("markdown",))
        )
        rust_only_rows = client.read_repo_search_rows(
            repo_search_request("alpha", limit=10, language_filters=("rust",))
        )
        src_rows = client.read_repo_search_rows(
            repo_search_request("alpha", limit=10, path_prefixes=("src/",))
        )
        readme_title_rows = client.read_repo_search_rows(
            repo_search_request("alpha", limit=10, title_filters=("readme",))
        )
        markdown_tag_rows = client.read_repo_search_rows(
            repo_search_request("alpha", limit=10, tag_filters=("lang:markdown",))
        )
        exact_match_rows = client.read_repo_search_rows(
            repo_search_request("searchonlytoken", limit=10, tag_filters=("match:exact",))
        )
        readme_filename_rows = client.read_repo_search_rows(
            repo_search_request("alpha", limit=10, filename_filters=("readme.md",))
        )
        flight_prefixed_rows = client.read_repo_search_rows(
            repo_search_request("flightbridgetoken", limit=10, path_prefixes=("src/flight",))
        )
        flight_title_rows = client.read_repo_search_rows(
            repo_search_request("flightbridgetoken", limit=10, title_filters=("flight_search",))
        )
        rust_tag_rows = client.read_repo_search_rows(
            repo_search_request("alpha", limit=10, tag_filters=("lang:rust",))
        )
        search_rows = client.read_repo_search_rows(repo_search_request("searchonlytoken", limit=1))
        mixed_case_exact_rows = client.read_repo_search_rows(
            repo_search_request("CamelBridgeToken", limit=2)
        )
        rank_tie_rows = client.read_repo_search_rows(
            repo_search_request("ranktieexacttoken", limit=1)
        )
        flight_search_rows = client.read_repo_search_rows(
            repo_search_request("flightbridgetoken", limit=10)
        )

        assert alpha_rows
        assert any(row.path == "src/lib.rs" for row in alpha_rows)
        assert all(row.language == "rust" for row in alpha_rows if row.path.startswith("src/"))
        assert len(markdown_rows) == 1
        assert markdown_rows[0].path == "README.md"
        assert markdown_rows[0].language == "markdown"
        assert rust_only_rows
        assert all(row.language == "rust" for row in rust_only_rows)
        assert all(row.path != "README.md" for row in rust_only_rows)
        assert src_rows
        assert all(row.path.startswith("src/") for row in src_rows)
        assert all(row.path != "README.md" for row in src_rows)
        assert len(readme_title_rows) == 1
        assert readme_title_rows[0].path == "README.md"
        assert len(markdown_tag_rows) == 1
        assert markdown_tag_rows[0].path == "README.md"
        assert len(exact_match_rows) == 1
        assert exact_match_rows[0].path == "src/search.rs"
        assert "searchonlytoken" in exact_match_rows[0].best_section.lower()
        assert exact_match_rows[0].match_reason == "repo_content_search"
        assert exact_match_rows[0].navigation_path.endswith("src/search.rs")
        assert exact_match_rows[0].navigation_category == "repo_code"
        assert exact_match_rows[0].navigation_line > 0
        assert exact_match_rows[0].navigation_line_end >= exact_match_rows[0].navigation_line
        assert exact_match_rows[0].hierarchy[0] == "src"
        assert "match:exact" in exact_match_rows[0].tags
        assert len(readme_filename_rows) == 1
        assert readme_filename_rows[0].path == "README.md"
        assert "alpha" in readme_filename_rows[0].best_section.lower()
        assert "lang:markdown" in readme_filename_rows[0].tags
        assert flight_prefixed_rows
        assert all(row.path.startswith("src/flight") for row in flight_prefixed_rows)
        assert flight_title_rows
        assert all("flight_search" in row.title.lower() for row in flight_title_rows)
        assert rust_tag_rows
        assert all(row.language == "rust" for row in rust_tag_rows)
        assert all(row.path != "README.md" for row in rust_tag_rows)
        assert len(search_rows) == 1
        assert search_rows[0].path == "src/search.rs"
        assert "searchonlytoken" in search_rows[0].best_section.lower()
        assert len(mixed_case_exact_rows) == 2
        assert mixed_case_exact_rows[0].path == "docs/CamelBridge.md"
        assert mixed_case_exact_rows[1].path == "src/camelbridge.rs"
        assert mixed_case_exact_rows[0].score > mixed_case_exact_rows[1].score
        assert "camelbridgetoken" in mixed_case_exact_rows[0].best_section.lower()
        assert mixed_case_exact_rows[0].match_reason == "repo_content_search"
        assert mixed_case_exact_rows[0].navigation_path.endswith("docs/CamelBridge.md")
        assert mixed_case_exact_rows[0].hierarchy[0] == "docs"
        assert "lang:markdown" in mixed_case_exact_rows[0].tags
        assert "lang:rust" in mixed_case_exact_rows[1].tags
        assert len(rank_tie_rows) == 1
        assert rank_tie_rows[0].path == "src/a_rank.rs"
        assert "ranktieexacttoken" in rank_tie_rows[0].best_section.lower()
        assert "lang:rust" in rank_tie_rows[0].tags
        assert any(row.path == "src/flight_search.rs" for row in flight_search_rows)
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_reads_query_via_restarted_wendao_search_flight_server(tmp_path) -> None:
    binary = _wendao_search_flight_server_binary()
    _run_rust_search_plane_seed_binary(str(tmp_path))

    def next_bind() -> tuple[str, int]:
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
            sock.bind(("127.0.0.1", 0))
            host, port = sock.getsockname()
        return host, port

    alpha_request = repo_search_request("alpha", limit=10)
    markdown_request = repo_search_request("alpha", limit=10, language_filters=("markdown",))
    src_request = repo_search_request("alpha", limit=10, path_prefixes=("src/",))
    readme_title_request = repo_search_request("alpha", limit=10, title_filters=("readme",))
    markdown_tag_request = repo_search_request("alpha", limit=10, tag_filters=("lang:markdown",))
    exact_match_request = repo_search_request(
        "searchonlytoken", limit=10, tag_filters=("match:exact",)
    )
    readme_filename_request = repo_search_request(
        "alpha", limit=10, filename_filters=("readme.md",)
    )
    search_request = repo_search_request("searchonlytoken", limit=1)
    mixed_case_exact_request = repo_search_request("CamelBridgeToken", limit=2)
    rank_tie_request = repo_search_request("ranktieexacttoken", limit=1)

    for _ in range(2):
        host, port = next_bind()
        process = _spawn_rust_mock_flight_server(
            host,
            port,
            binary_env_var="WENDAO_SEARCH_SERVER_BINARY",
            default_binary=binary,
            extra_args=("alpha/repo", str(tmp_path), "3"),
        )
        try:
            client = WendaoTransportClient(
                WendaoTransportConfig(
                    endpoint=WendaoTransportEndpoint(host=host, port=port),
                    schema_version="v2",
                    request_timeout_seconds=10.0,
                )
            )
            info = client.get_repo_search_info(alpha_request)
            alpha_rows = client.read_repo_search_rows(alpha_request)
            markdown_rows = client.read_repo_search_rows(markdown_request)
            src_rows = client.read_repo_search_rows(src_request)
            readme_title_rows = client.read_repo_search_rows(readme_title_request)
            markdown_tag_rows = client.read_repo_search_rows(markdown_tag_request)
            exact_match_rows = client.read_repo_search_rows(exact_match_request)
            readme_filename_rows = client.read_repo_search_rows(readme_filename_request)
            search_rows = client.read_repo_search_rows(search_request)
            mixed_case_exact_rows = client.read_repo_search_rows(mixed_case_exact_request)
            rank_tie_rows = client.read_repo_search_rows(rank_tie_request)

            assert info.descriptor.path == [b"search", b"repos", b"main"]
            assert alpha_rows
            assert any(row.path == "src/lib.rs" for row in alpha_rows)
            assert len(markdown_rows) == 1
            assert markdown_rows[0].path == "README.md"
            assert src_rows
            assert all(row.path.startswith("src/") for row in src_rows)
            assert len(readme_title_rows) == 1
            assert readme_title_rows[0].path == "README.md"
            assert len(markdown_tag_rows) == 1
            assert markdown_tag_rows[0].path == "README.md"
            assert len(exact_match_rows) == 1
            assert exact_match_rows[0].path == "src/search.rs"
            assert "searchonlytoken" in exact_match_rows[0].best_section.lower()
            assert exact_match_rows[0].match_reason == "repo_content_search"
            assert exact_match_rows[0].navigation_path.endswith("src/search.rs")
            assert exact_match_rows[0].navigation_category == "repo_code"
            assert exact_match_rows[0].navigation_line > 0
            assert exact_match_rows[0].navigation_line_end >= exact_match_rows[0].navigation_line
            assert exact_match_rows[0].hierarchy[0] == "src"
            assert "match:exact" in exact_match_rows[0].tags
            assert len(readme_filename_rows) == 1
            assert readme_filename_rows[0].path == "README.md"
            assert "alpha" in readme_filename_rows[0].best_section.lower()
            assert "lang:markdown" in readme_filename_rows[0].tags
            assert len(search_rows) == 1
            assert search_rows[0].path == "src/search.rs"
            assert "searchonlytoken" in search_rows[0].best_section.lower()
            assert len(mixed_case_exact_rows) == 2
            assert mixed_case_exact_rows[0].path == "docs/CamelBridge.md"
            assert mixed_case_exact_rows[1].path == "src/camelbridge.rs"
            assert mixed_case_exact_rows[0].score > mixed_case_exact_rows[1].score
            assert "camelbridgetoken" in mixed_case_exact_rows[0].best_section.lower()
            assert mixed_case_exact_rows[0].match_reason == "repo_content_search"
            assert mixed_case_exact_rows[0].navigation_path.endswith("docs/CamelBridge.md")
            assert mixed_case_exact_rows[0].hierarchy[0] == "docs"
            assert "lang:markdown" in mixed_case_exact_rows[0].tags
            assert "lang:rust" in mixed_case_exact_rows[1].tags
            assert len(rank_tie_rows) == 1
            assert rank_tie_rows[0].path == "src/a_rank.rs"
            assert "ranktieexacttoken" in rank_tie_rows[0].best_section.lower()
            assert "lang:rust" in rank_tie_rows[0].tags
        finally:
            _terminate_process(process)


@pytest.mark.integration
def test_transport_client_exchanges_rerank_rows_via_wendao_search_flight_server(tmp_path) -> None:
    binary = _wendao_search_flight_server_binary()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_SEARCH_SERVER_BINARY",
        default_binary=binary,
        schema_version="v2",
        schema_version_uses_flag=True,
        extra_args=("alpha/repo", str(tmp_path), "3"),
        cwd=str(tmp_path),
        extra_env={"PRJ_ROOT": str(tmp_path)},
    )
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
            ]
        )

        assert [row.doc_id for row in rows] == ["doc-0", "doc-1"]
        assert [row.vector_score for row in rows] == pytest.approx([0.5, 0.8])
        assert [row.semantic_score for row in rows] == pytest.approx([1.0, 0.5])
        assert [row.rank for row in rows] == [1, 2]
        assert [row.final_score for row in rows] == pytest.approx([0.8, 0.62])
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_limits_rerank_rows_via_wendao_search_flight_server(tmp_path) -> None:
    binary = _wendao_search_flight_server_binary()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_SEARCH_SERVER_BINARY",
        default_binary=binary,
        schema_version="v2",
        schema_version_uses_flag=True,
        extra_args=("alpha/repo", str(tmp_path), "3"),
        cwd=str(tmp_path),
        extra_env={"PRJ_ROOT": str(tmp_path)},
    )
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
            top_k=1,
        )

        assert len(rows) == 1
        assert rows[0].doc_id == "doc-0"
        assert rows[0].rank == 1
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_preserves_full_rerank_rows_when_top_k_exceeds_result_count_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    binary = _wendao_search_flight_server_binary()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_SEARCH_SERVER_BINARY",
        default_binary=binary,
        schema_version="v2",
        schema_version_uses_flag=True,
        extra_args=("alpha/repo", str(tmp_path), "3"),
        cwd=str(tmp_path),
        extra_env={"PRJ_ROOT": str(tmp_path)},
    )
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
            top_k=10,
        )

        assert [row.doc_id for row in rows] == ["doc-0", "doc-1"]
        assert [row.rank for row in rows] == [1, 2]
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_preserves_full_rerank_rows_when_top_k_matches_result_count_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    binary = _wendao_search_flight_server_binary()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_SEARCH_SERVER_BINARY",
        default_binary=binary,
        schema_version="v2",
        schema_version_uses_flag=True,
        extra_args=("alpha/repo", str(tmp_path), "3"),
        cwd=str(tmp_path),
        extra_env={"PRJ_ROOT": str(tmp_path)},
    )
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
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_rejects_invalid_rerank_top_k_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    binary = _wendao_search_flight_server_binary()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_SEARCH_SERVER_BINARY",
        default_binary=binary,
        schema_version="v2",
        schema_version_uses_flag=True,
        extra_args=("alpha/repo", str(tmp_path), "3"),
        cwd=str(tmp_path),
        extra_env={"PRJ_ROOT": str(tmp_path)},
    )
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        request = build_rerank_request_table(
            [
                WendaoRerankRequestRow(
                    doc_id="doc-0",
                    vector_score=0.5,
                    embedding=(1.0, 0.0, 0.0),
                    query_embedding=(1.0, 0.0, 0.0),
                )
            ]
        )

        with pytest.raises(pa.ArrowInvalid, match="invalid rerank top_k header"):
            client.exchange_query_table(
                WendaoFlightRouteQuery(route="/rerank/flight"),
                request,
                extra_metadata={
                    WENDAO_RERANK_DIMENSION_HEADER: "3",
                    WENDAO_RERANK_TOP_K_HEADER: "abc",
                },
            )
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_rejects_blank_rerank_top_k_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    binary = _wendao_search_flight_server_binary()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_SEARCH_SERVER_BINARY",
        default_binary=binary,
        schema_version="v2",
        schema_version_uses_flag=True,
        extra_args=("alpha/repo", str(tmp_path), "3"),
        cwd=str(tmp_path),
        extra_env={"PRJ_ROOT": str(tmp_path)},
    )
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        request = build_rerank_request_table(
            [
                WendaoRerankRequestRow(
                    doc_id="doc-0",
                    vector_score=0.5,
                    embedding=(1.0, 0.0, 0.0),
                    query_embedding=(1.0, 0.0, 0.0),
                )
            ]
        )

        with pytest.raises(pa.ArrowInvalid, match="invalid rerank top_k header"):
            client.exchange_query_table(
                WendaoFlightRouteQuery(route="/rerank/flight"),
                request,
                extra_metadata={
                    WENDAO_RERANK_DIMENSION_HEADER: "3",
                    WENDAO_RERANK_TOP_K_HEADER: "",
                },
            )
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_rejects_zero_rerank_top_k_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    binary = _wendao_search_flight_server_binary()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_SEARCH_SERVER_BINARY",
        default_binary=binary,
        schema_version="v2",
        schema_version_uses_flag=True,
        extra_args=("alpha/repo", str(tmp_path), "3"),
        cwd=str(tmp_path),
        extra_env={"PRJ_ROOT": str(tmp_path)},
    )
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        request = build_rerank_request_table(
            [
                WendaoRerankRequestRow(
                    doc_id="doc-0",
                    vector_score=0.5,
                    embedding=(1.0, 0.0, 0.0),
                    query_embedding=(1.0, 0.0, 0.0),
                )
            ]
        )

        with pytest.raises(
            pa.ArrowInvalid,
            match="rerank top_k header `x-wendao-rerank-top-k` must be greater than zero",
        ):
            client.exchange_query_table(
                WendaoFlightRouteQuery(route="/rerank/flight"),
                request,
                extra_metadata={
                    WENDAO_RERANK_DIMENSION_HEADER: "3",
                    WENDAO_RERANK_TOP_K_HEADER: "0",
                },
            )
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_applies_runtime_rerank_weight_policy_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    binary = _wendao_search_flight_server_binary()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_SEARCH_SERVER_BINARY",
        default_binary=binary,
        extra_args=("alpha/repo", str(tmp_path), "3"),
        extra_env={
            "WENDAO_RERANK_VECTOR_WEIGHT": "0.9",
            "WENDAO_RERANK_SEMANTIC_WEIGHT": "0.1",
        },
    )
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
            ]
        )

        assert [row.doc_id for row in rows] == ["doc-1", "doc-0"]
        assert [row.vector_score for row in rows] == pytest.approx([0.8, 0.5])
        assert [row.semantic_score for row in rows] == pytest.approx([0.5, 1.0])
        assert [row.rank for row in rows] == [1, 2]
        assert [row.final_score for row in rows] == pytest.approx([0.77, 0.55])
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_applies_wendao_toml_rerank_weight_policy_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    binary = _wendao_search_flight_server_binary()
    wendao_toml = tmp_path / "wendao.toml"
    wendao_toml.write_text(
        """
[link_graph.retrieval.julia_rerank]
vector_weight = 0.9
similarity_weight = 0.1
""".strip()
        + "\n",
        encoding="utf-8",
    )

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_SEARCH_SERVER_BINARY",
        default_binary=binary,
        extra_args=("alpha/repo", str(tmp_path), "3"),
        cwd=str(tmp_path),
        extra_env={"PRJ_ROOT": str(tmp_path)},
    )
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
            ]
        )

        assert [row.doc_id for row in rows] == ["doc-1", "doc-0"]
        assert [row.vector_score for row in rows] == pytest.approx([0.8, 0.5])
        assert [row.semantic_score for row in rows] == pytest.approx([0.5, 1.0])
        assert [row.rank for row in rows] == [1, 2]
        assert [row.final_score for row in rows] == pytest.approx([0.77, 0.55])
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_prefers_wendao_toml_over_env_for_rerank_weights_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    binary = _wendao_search_flight_server_binary()
    wendao_toml = tmp_path / "wendao.toml"
    wendao_toml.write_text(
        """
[link_graph.retrieval.julia_rerank]
vector_weight = 0.9
similarity_weight = 0.1
""".strip()
        + "\n",
        encoding="utf-8",
    )

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_SEARCH_SERVER_BINARY",
        default_binary=binary,
        extra_args=("alpha/repo", str(tmp_path), "3"),
        cwd=str(tmp_path),
        extra_env={
            "PRJ_ROOT": str(tmp_path),
            "WENDAO_RERANK_VECTOR_WEIGHT": "0.1",
            "WENDAO_RERANK_SEMANTIC_WEIGHT": "0.9",
        },
    )
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
            ]
        )

        assert [row.doc_id for row in rows] == ["doc-1", "doc-0"]
        assert [row.final_score for row in rows] == pytest.approx([0.77, 0.55])
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_reads_schema_version_from_wendao_toml_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    binary = _wendao_search_flight_server_binary()
    wendao_toml = tmp_path / "wendao.toml"
    wendao_toml.write_text(
        """
[link_graph.retrieval.julia_rerank]
schema_version = "v9"
""".strip()
        + "\n",
        encoding="utf-8",
    )
    _run_rust_search_plane_seed_binary(str(tmp_path))

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_SEARCH_SERVER_BINARY",
        default_binary=binary,
        schema_version=None,
        schema_version_uses_flag=True,
        extra_args=("alpha/repo", str(tmp_path), "3"),
        cwd=str(tmp_path),
        extra_env={"PRJ_ROOT": str(tmp_path)},
    )
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v9",
                request_timeout_seconds=10.0,
            )
        )
        request = repo_search_request("alpha", limit=1)
        info = client.get_repo_search_info(request)
        rows = client.read_repo_search_rows(request)

        assert len(info.endpoints) == 1
        assert rows
        assert rows[0].path == "README.md"
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_prefers_cli_schema_version_via_runtime_wendao_flight_server() -> None:
    binary = _wendao_runtime_flight_server_binary()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_MOCK_SERVER_BINARY",
        default_binary=binary,
        schema_version="v8",
        schema_version_uses_flag=True,
        extra_args=("3",),
    )
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v8",
                request_timeout_seconds=10.0,
            )
        )
        request = repo_search_request("repo", limit=1)
        info = client.get_repo_search_info(request)
        rows = client.read_repo_search_rows(request)

        assert len(info.endpoints) == 1
        assert rows
        assert rows[0].path == "src/lib.rs"
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_prefers_cli_rerank_dimension_via_runtime_wendao_flight_server() -> None:
    binary = _wendao_runtime_flight_server_binary()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_MOCK_SERVER_BINARY",
        default_binary=binary,
        schema_version="v2",
        schema_version_uses_flag=True,
        rerank_dimension=4,
        rerank_dimension_uses_flag=True,
    )
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
                    embedding=(1.0, 0.0, 0.0, 0.0),
                    query_embedding=(1.0, 0.0, 0.0, 0.0),
                )
            ]
        )

        assert rows
        assert rows[0].doc_id == "doc-0"
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_limits_rerank_results_via_runtime_wendao_flight_server() -> None:
    binary = _wendao_runtime_flight_server_binary()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_MOCK_SERVER_BINARY",
        default_binary=binary,
        schema_version="v2",
        schema_version_uses_flag=True,
    )
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
            top_k=1,
        )

        assert len(rows) == 1
        assert rows[0].doc_id == "doc-0"
        assert rows[0].rank == 1
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_preserves_full_rerank_results_when_top_k_exceeds_result_count_via_runtime_wendao_flight_server() -> (
    None
):
    binary = _wendao_runtime_flight_server_binary()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_MOCK_SERVER_BINARY",
        default_binary=binary,
        schema_version="v2",
        schema_version_uses_flag=True,
    )
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
            top_k=10,
        )

        assert [row.doc_id for row in rows] == ["doc-0", "doc-1"]
        assert [row.rank for row in rows] == [1, 2]
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_preserves_full_rerank_results_when_top_k_matches_result_count_via_runtime_wendao_flight_server() -> (
    None
):
    binary = _wendao_runtime_flight_server_binary()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_MOCK_SERVER_BINARY",
        default_binary=binary,
        schema_version="v2",
        schema_version_uses_flag=True,
    )
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
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_rejects_invalid_rerank_top_k_via_runtime_wendao_flight_server() -> None:
    binary = _wendao_runtime_flight_server_binary()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_MOCK_SERVER_BINARY",
        default_binary=binary,
        schema_version="v2",
        schema_version_uses_flag=True,
    )
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        request = build_rerank_request_table(
            [
                WendaoRerankRequestRow(
                    doc_id="doc-0",
                    vector_score=0.5,
                    embedding=(1.0, 0.0, 0.0),
                    query_embedding=(1.0, 0.0, 0.0),
                )
            ]
        )

        with pytest.raises(pa.ArrowInvalid, match="invalid rerank top_k header"):
            client.exchange_query_table(
                WendaoFlightRouteQuery(route="/rerank/flight"),
                request,
                extra_metadata={
                    WENDAO_RERANK_DIMENSION_HEADER: "3",
                    WENDAO_RERANK_TOP_K_HEADER: "abc",
                },
            )
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_rejects_blank_rerank_top_k_via_runtime_wendao_flight_server() -> None:
    binary = _wendao_runtime_flight_server_binary()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_MOCK_SERVER_BINARY",
        default_binary=binary,
        schema_version="v2",
        schema_version_uses_flag=True,
    )
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        request = build_rerank_request_table(
            [
                WendaoRerankRequestRow(
                    doc_id="doc-0",
                    vector_score=0.5,
                    embedding=(1.0, 0.0, 0.0),
                    query_embedding=(1.0, 0.0, 0.0),
                )
            ]
        )

        with pytest.raises(pa.ArrowInvalid, match="invalid rerank top_k header"):
            client.exchange_query_table(
                WendaoFlightRouteQuery(route="/rerank/flight"),
                request,
                extra_metadata={
                    WENDAO_RERANK_DIMENSION_HEADER: "3",
                    WENDAO_RERANK_TOP_K_HEADER: "",
                },
            )
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_rejects_zero_rerank_top_k_via_runtime_wendao_flight_server() -> None:
    binary = _wendao_runtime_flight_server_binary()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_MOCK_SERVER_BINARY",
        default_binary=binary,
        schema_version="v2",
        schema_version_uses_flag=True,
    )
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )
        request = build_rerank_request_table(
            [
                WendaoRerankRequestRow(
                    doc_id="doc-0",
                    vector_score=0.5,
                    embedding=(1.0, 0.0, 0.0),
                    query_embedding=(1.0, 0.0, 0.0),
                )
            ]
        )

        with pytest.raises(
            pa.ArrowInvalid,
            match="rerank top_k header `x-wendao-rerank-top-k` must be greater than zero",
        ):
            client.exchange_query_table(
                WendaoFlightRouteQuery(route="/rerank/flight"),
                request,
                extra_metadata={
                    WENDAO_RERANK_DIMENSION_HEADER: "3",
                    WENDAO_RERANK_TOP_K_HEADER: "0",
                },
            )
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_prefers_cli_schema_version_over_wendao_toml_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    binary = _wendao_search_flight_server_binary()
    wendao_toml = tmp_path / "wendao.toml"
    wendao_toml.write_text(
        """
[link_graph.retrieval.julia_rerank]
schema_version = "v9"
""".strip()
        + "\n",
        encoding="utf-8",
    )
    _run_rust_search_plane_seed_binary(str(tmp_path))

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_SEARCH_SERVER_BINARY",
        default_binary=binary,
        schema_version="v8",
        schema_version_uses_flag=True,
        extra_args=("alpha/repo", str(tmp_path), "3"),
        cwd=str(tmp_path),
        extra_env={"PRJ_ROOT": str(tmp_path)},
    )
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v8",
                request_timeout_seconds=10.0,
            )
        )
        request = repo_search_request("alpha", limit=1)
        info = client.get_repo_search_info(request)
        rows = client.read_repo_search_rows(request)

        assert len(info.endpoints) == 1
        assert rows
        assert rows[0].path == "README.md"
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_prefers_cli_rerank_dimension_over_positional_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    binary = _wendao_search_flight_server_binary()
    _run_rust_search_plane_seed_binary(str(tmp_path))

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_SEARCH_SERVER_BINARY",
        default_binary=binary,
        schema_version="v2",
        schema_version_uses_flag=True,
        rerank_dimension=4,
        rerank_dimension_uses_flag=True,
        extra_args=("alpha/repo", str(tmp_path), "3"),
        cwd=str(tmp_path),
        extra_env={"PRJ_ROOT": str(tmp_path)},
    )
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
                    embedding=(1.0, 0.0, 0.0, 0.0),
                    query_embedding=(1.0, 0.0, 0.0, 0.0),
                )
            ]
        )

        assert rows
        assert rows[0].doc_id == "doc-0"
    finally:
        _terminate_process(process)


@pytest.mark.integration
def test_transport_client_rejects_duplicate_rerank_doc_ids_via_wendao_search_flight_server(
    tmp_path,
) -> None:
    binary = _wendao_search_flight_server_binary()

    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
        sock.bind(("127.0.0.1", 0))
        host, port = sock.getsockname()

    process = _spawn_rust_mock_flight_server(
        host,
        port,
        binary_env_var="WENDAO_SEARCH_SERVER_BINARY",
        default_binary=binary,
        extra_args=("alpha/repo", str(tmp_path), "3"),
    )
    try:
        client = WendaoTransportClient(
            WendaoTransportConfig(
                endpoint=WendaoTransportEndpoint(host=host, port=port),
                schema_version="v2",
                request_timeout_seconds=10.0,
            )
        )

        with pytest.raises(
            pa.ArrowInvalid,
            match="doc_id` must be unique across one batch",
        ):
            client.exchange_rerank_rows(
                [
                    WendaoRerankRequestRow(
                        doc_id="doc-0",
                        vector_score=0.5,
                        embedding=(0.1, 0.2, 0.3),
                        query_embedding=(0.4, 0.5, 0.6),
                    ),
                    WendaoRerankRequestRow(
                        doc_id="doc-0",
                        vector_score=0.4,
                        embedding=(0.7, 0.8, 0.9),
                        query_embedding=(0.4, 0.5, 0.6),
                    ),
                ]
            )
    finally:
        _terminate_process(process)
