# xiuxian-wendao-julia

`xiuxian-wendao-julia` is the external Julia Repo Intelligence plugin crate for `xiuxian-wendao`.

## Ownership Boundary

- `xiuxian-vector` owns the reusable Arrow IPC transport substrate in `src/arrow_transport/`.
- `xiuxian-wendao-julia` owns Julia-specific interpretation of repository plugin options and builds a transport client from them.
- `xiuxian-wendao-julia` also owns the legacy Julia link-graph compatibility semantics under `src/compatibility/link_graph/`, including Julia selector ids, the default analyzer package dir, launcher path, example-config path, the Julia rerank runtime record, service-descriptor and CLI-arg meaning, launch-manifest meaning, deployment-artifact meaning, and conversions to and from Wendao core plugin contracts.
- `xiuxian-wendao` hosts the analyzer registry and loads repository config, but it does not own a second Arrow transport implementation.
- `xiuxian-wendao` now consumes this crate through a normal Cargo dependency instead of sibling-source inclusion.

## Public Surface

- `JuliaRepoIntelligencePlugin`
- `register_into`
- `build_julia_arrow_transport_client`
- `process_julia_arrow_batches`
- `validate_julia_arrow_response_batches`
- `compatibility::link_graph::*` for Julia-owned legacy launch/deployment compatibility DTOs, the Julia rerank runtime record, selector helpers, and analyzer package-path defaults

The transport builder consumes repository plugin entries that resolve to:

```toml
[link_graph.projects.sample]
root = "/path/to/repo"
plugins = [
  "julia",
  { id = "julia", arrow_transport = { base_url = "http://127.0.0.1:8080", route = "/arrow-ipc", health_route = "/health", timeout_secs = 15 } }
]
```

The inline object is materialized by `xiuxian-wendao` as `RepositoryPluginConfig::Config`, then interpreted here to construct `xiuxian_vector::ArrowTransportClient`.

The transport client now sends `x-wendao-schema-version` and defaults to the
`v1` WendaoArrow contract unless the repository plugin config overrides
`schema_version`.

`validate_julia_arrow_response_batches` enforces the current `v1` response
shape before a future gateway integration accepts analyzer output:

- required columns: `doc_id`, `analyzer_score`, `final_score`
- `doc_id` must be unique and non-null
- `final_score` must be finite

`process_julia_arrow_batches` is the thin runtime hook for future gateway
integration. It performs:

- Arrow IPC HTTP roundtrip via `xiuxian_vector::ArrowTransportClient`
- response schema-version enforcement
- `v1` Julia response validation before returning decoded record batches

## Validation

- `direnv exec . cargo test -p xiuxian-wendao-julia transport --lib`
- `direnv exec . cargo test -p xiuxian-wendao-julia process_julia_arrow_batches_against_real_wendaoarrow_service --lib`
- `direnv exec . cargo test -p xiuxian-wendao-julia real_wendaoarrow_metadata_example_roundtrip_decodes_trace_id_column --lib`
- `direnv exec . cargo check -p xiuxian-wendao-julia --lib`

The real loopback test does not use an Axum mock for the processor path. It
spawns `.data/WendaoArrow/scripts/run_stream_scoring_server.sh`, waits for
`/health`, then posts Arrow IPC batches through
`xiuxian_vector::ArrowTransportClient` and asserts the Rust side can decode the
returned Arrow response contract and scoring values. Those fixtures now use the
shared `julia_arrow_request_schema(...)` builder as well, so the official
example roundtrip receives the full WendaoArrow `v1` request shape instead of a
test-local reduced schema.

There is also a metadata-aware real loopback that targets
`.data/WendaoArrow/scripts/run_stream_metadata_server.sh`, sends a request
whose Arrow schema metadata includes `trace_id`, and asserts the Rust side can
decode the additive `trace_id` response column. That path now goes through the
production `xiuxian_vector::ArrowTransportClient`, so the test verifies request
schema metadata survives the real transport API instead of only a hand-written
HTTP fixture.

The corresponding test support is now split under `src/plugin/test_support/`
into shared child/path helpers and official-example helpers, mirroring the
same semantic split used by `xiuxian-wendao` integration support.
