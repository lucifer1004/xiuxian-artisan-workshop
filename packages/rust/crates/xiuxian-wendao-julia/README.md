# xiuxian-wendao-julia

`xiuxian-wendao-julia` is the external Julia Repo Intelligence plugin crate for `xiuxian-wendao`.

## Ownership Boundary

- `xiuxian-wendao-runtime` owns the reusable Arrow Flight runtime client and negotiation seam.
- `xiuxian-wendao-julia` owns Julia-specific interpretation of repository plugin options and translates them into the runtime-owned Flight binding.
- `xiuxian-wendao-julia` owns the Julia Arrow rerank exchange seam, including typed request or score rows, request-batch assembly, response decoding, repository fetch helpers, and plugin-local tests.
- `xiuxian-wendao-julia` also owns Julia-specific graph-structural transport option parsing, route-kind dispatch defaults, and staged request or response validation for promoted structural-search downcalls.
- `xiuxian-wendao-julia` also owns Julia-specific graph-structural route names, draft schema-version defaults, semantic projection DTOs, typed request or response row helpers, and Arrow batch validation for the mixed-graph structural plugin lane.
- `xiuxian-wendao-julia` also owns stable two-node pair projection helpers for that lane, including pair candidate id normalization, pair candidate subgraph projection, and pair-to-request-row builders.
- `xiuxian-wendao-julia` also owns simple keyword-or-tag query-context builders and binary keyword-or-tag rerank-signal builders for that lane, so host consumers do not manually create anchor DTOs or convert boolean matches into staged plane scores.
- `xiuxian-wendao-julia` also owns the next convenience layer above those helpers: combined keyword-or-tag pair-rerank request-row builders that compose query-context, rerank-signal, and pair-row projection in one plugin-owned call.
- `xiuxian-wendao-julia` also owns shared-tag overlap discovery for that lane, including normalized shared-tag anchor extraction and a tag-overlap-aware combined pair-rerank helper.
- `xiuxian-wendao-julia` also owns the metadata-aware convenience layer above that seam, including node-metadata input bundles and a metadata-aware overlap helper that keeps host consumers from passing ad hoc tag vectors into request projection.
- `xiuxian-wendao-julia` also owns the metadata-aware batch-assembly layer above that seam, including scored metadata-aware rerank input bundles and a batch helper that composes metadata projection and Arrow request materialization inside the plugin crate.
- `xiuxian-wendao-julia` also owns the higher-level candidate-input layer above that seam, including single-bundle keyword-overlap request inputs and a batch helper that composes query, metadata, pair, and score staging inside the plugin crate.
- `xiuxian-wendao-julia` also owns the shared-query and candidate-bundle layer above that seam, including one shared keyword-overlap query bundle, one plugin-owned per-pair candidate bundle, and a batch helper that derives the higher-level request inputs inside the plugin crate.
- `xiuxian-wendao-julia` also owns the raw-to-candidate staging helper above that seam, so host callers can hand over one pair-input DTO plus raw tag vectors and scores without manually constructing the node-metadata or candidate-bundle DTO layers first.
- `xiuxian-wendao-julia` also owns the raw-to-query staging helper above that seam, so host callers can hand over raw query identity, layer bounds, keyword anchors, and edge constraints without manually constructing the shared-query DTO layer first.
- `xiuxian-wendao-julia` also owns the raw-to-pair staging helper above that seam, so host callers can hand over raw pair ids and edge kinds without manually constructing the pair-input DTO layer first.
- `xiuxian-wendao-julia` also owns the raw pair-metadata-to-candidate staging helper above that seam, so host callers can hand over raw pair ids, edge kinds, left or right tags, and scores without manually composing the metadata-bundle helper and the candidate-bundle helper in sequence.
- `xiuxian-wendao-julia` also owns the raw-candidate collection batch or fetch seam above that layer, so host callers can hand over one shared query plus raw candidate bundles without manually normalizing each candidate before request-batch or repository-fetch dispatch.
- `xiuxian-wendao-julia` also owns the legacy Julia link-graph compatibility semantics under `src/compatibility/link_graph/`, including Julia selector ids, the default analyzer package dir, launcher path, example-config path, the Julia rerank runtime record, service-descriptor and CLI-arg meaning, launch-manifest meaning, deployment-artifact meaning, and conversions to and from Wendao core plugin contracts.
- `xiuxian-wendao` hosts the analyzer registry and loads repository config, but it does not own a second transport implementation or a second graph-structural adapter layer.
- `xiuxian-wendao` now consumes this crate through a normal Cargo dependency instead of sibling-source inclusion.

## Public Surface

- `JuliaRepoIntelligencePlugin`
- `register_into`
- `build_julia_flight_transport_client`
- `process_julia_flight_batches`
- `validate_julia_arrow_response_batches`
- `GraphStructuralRouteKind`
- `JULIA_GRAPH_STRUCTURAL_SCHEMA_VERSION`
- `graph_structural_route_kind`
- `is_graph_structural_route`
- `validate_graph_structural_*`
- `GraphStructuralQueryAnchor`
- `GraphStructuralQueryContext`
- `GraphStructuralCandidateSubgraph`
- `GraphStructuralKeywordTagQueryInputs`
- `GraphStructuralNodeMetadataInputs`
- `GraphStructuralKeywordOverlapPairInputs`
- `GraphStructuralKeywordOverlapPairRerankInputs`
- `GraphStructuralKeywordOverlapPairRequestInputs`
- `GraphStructuralPairCandidateInputs`
- `GraphStructuralKeywordOverlapQueryInputs`
- `GraphStructuralKeywordOverlapRawCandidateInputs`
- `GraphStructuralKeywordOverlapCandidateInputs`
- `GraphStructuralRerankSignals`
- `GraphStructuralFilterConstraint`
- `GraphStructural*RequestRow`
- `GraphStructural*ScoreRow`
- `graph_structural_pair_candidate_id`
- `graph_structural_shared_tag_anchors`
- `build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_inputs`
- `build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_metadata`
- `build_graph_structural_keyword_overlap_candidate_inputs`
- `build_graph_structural_keyword_overlap_raw_candidate_inputs`
- `build_graph_structural_keyword_overlap_pair_candidate_inputs_from_raw`
- `build_graph_structural_keyword_overlap_query_inputs`
- `build_graph_structural_pair_candidate_inputs`
- `build_graph_structural_keyword_overlap_pair_request_input`
- `build_graph_structural_keyword_overlap_pair_rerank_request_batch`
- `build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_raw_candidates`
- `build_graph_structural_keyword_overlap_pair_rerank_request_row`
- `build_graph_structural_keyword_overlap_pair_rerank_request_row_from_metadata`
- `build_graph_structural_keyword_tag_query_context`
- `build_graph_structural_keyword_tag_pair_rerank_request_row`
- `build_graph_structural_keyword_tag_rerank_signals`
- `build_graph_structural_pair_candidate_subgraph`
- `build_graph_structural_pair_*_request_row`
- `build_graph_structural_*_request_row`
- `build_graph_structural_*_request_batch`
- `decode_graph_structural_*_score_rows`
- `fetch_graph_structural_*_rows_for_repository`
- `fetch_graph_structural_keyword_overlap_pair_rerank_rows_for_repository_from_raw_candidates`
- `build_graph_structural_flight_transport_client`
- `process_graph_structural_flight_batches`
- `process_graph_structural_flight_batches_for_repository`
- `compatibility::link_graph::*` for Julia-owned legacy launch/deployment compatibility DTOs, the Julia rerank runtime record, selector helpers, and analyzer package-path defaults

The transport builder consumes repository plugin entries that resolve to:

```toml
[link_graph.projects.sample]
root = "/path/to/repo"
plugins = [
  "julia",
  { id = "julia", flight_transport = { base_url = "http://127.0.0.1:8815", route = "/rerank", health_route = "/healthz", timeout_secs = 15 } }
]
```

The inline object is materialized by `xiuxian-wendao` as
`RepositoryPluginConfig::Config`, then interpreted here to construct a
runtime-owned Arrow Flight binding and negotiated Flight client.

The graph-structural transport surface now stages from a separate repository
plugin option block so Search downcalls can stay Julia-plugin-owned as well:

```toml
[link_graph.projects.sample]
root = "/path/to/repo"
plugins = [
  "julia",
  { id = "julia", graph_structural_transport = { base_url = "http://127.0.0.1:8815", structural_rerank = { route = "/graph/structural/rerank", schema_version = "v0-draft" }, constraint_filter = { route = "/graph/structural/filter", timeout_secs = 20 } } }
]
```

That block is interpreted in `xiuxian-wendao-julia` rather than in
`xiuxian-wendao-runtime`. The runtime still owns generic Arrow Flight
negotiation only.

The same ownership rule now applies to the typed Rust exchange helpers for
these structural routes:

- semantic projection DTOs live in `xiuxian-wendao-julia`
- request-row structs and Arrow batch builders live in `xiuxian-wendao-julia`
- response-row structs and Arrow batch decoders live in `xiuxian-wendao-julia`
- repository-configured fetch helpers also live in `xiuxian-wendao-julia`
- `xiuxian-wendao` should consume or re-export that surface rather than grow a
  host-local graph-structural adapter module

The same rule also now has a bounded host-side proof in
`xiuxian-wendao`: the integration target
`packages/rust/crates/xiuxian-wendao/tests/xiuxian-testing-gate.rs`
through the `link_graph_agentic_expansion` unit module projects a real
`LinkGraphIndex` agentic-expansion pair through these Julia-owned pair helpers
and DTOs, then into a validated structural-rerank request batch, without
introducing a new production graph-structural adapter in the host crate.

That bounded proof now also consumes Julia-owned keyword-or-tag query and
binary rerank-signal helpers, so the host no longer manually creates
`GraphStructuralQueryAnchor` rows or converts boolean keyword-or-tag matches
into `1.0` or `0.0` plane scores by hand.

The same proof now also consumes a single Julia-owned combined helper for the
final staged rerank row, so the host no longer manually composes
`query context -> rerank signals -> pair rerank row` as three separate steps.
That convenience helper now accepts dedicated query and pair input bundles,
which keeps the public surface below the clippy argument-count ceiling without
moving the normalization logic back into host crates.

The same proof now also leaves shared-tag overlap discovery inside
`xiuxian-wendao-julia`, so the host only forwards raw left or right tag
metadata instead of finding the overlap itself.

The same proof now also stages those raw metadata slices through
plugin-owned metadata input bundles before building the staged rerank row, so
the host no longer threads raw tag vectors directly into the overlap helper.

The same proof now also consumes a plugin-owned metadata-aware batch helper,
so the host no longer assembles `Vec<GraphStructuralRerankRequestRow>` before
calling the staged Arrow batch builder.

The same proof now also consumes a single higher-level candidate-input bundle
per pair, so the host no longer manually composes query-input, metadata-input,
pair-input, and scored-rerank-input DTOs before building the staged request
batch.

The same proof now also consumes one shared query bundle plus one
plugin-owned per-candidate bundle per pair, so the host no longer constructs
`GraphStructuralKeywordOverlapPairRequestInputs` by hand before staging the
request batch.

The same Julia-owned seam now also has a repository-fetch convenience helper,
`fetch_graph_structural_keyword_overlap_pair_rerank_rows_for_repository(...)`,
so a future host caller with query-plus-candidate DTOs can skip manual batch
materialization before calling the configured structural-rerank transport.
That bounded proof now consumes the graph-structural helper surface through
`xiuxian_wendao::analyzers::languages`, which keeps the host on the intended
thin language seam instead of importing the Julia crate directly.
That same proof now also consumes
`build_graph_structural_keyword_overlap_candidate_inputs(...)`, so the host
no longer manually constructs `GraphStructuralNodeMetadataInputs` or
`GraphStructuralKeywordOverlapCandidateInputs` before staging the rerank
request or repository fetch.
That same proof now also consumes
`build_graph_structural_keyword_overlap_query_inputs(...)`, so the host no
longer manually constructs `GraphStructuralKeywordOverlapQueryInputs` before
staging the rerank request or repository fetch.
That same proof now also consumes
`build_graph_structural_pair_candidate_inputs(...)`, so the host no longer
manually constructs `GraphStructuralPairCandidateInputs` before staging the
rerank request or repository fetch.
That same proof now also consumes
`build_graph_structural_keyword_overlap_pair_candidate_inputs_from_raw(...)`,
so the host no longer manually composes
`build_graph_structural_keyword_overlap_pair_candidate_metadata_inputs(...)`
and `build_graph_structural_keyword_overlap_candidate_inputs(...)` before
staging the rerank request or repository fetch.
That same proof now also consumes
`build_graph_structural_keyword_overlap_raw_candidate_inputs(...)`,
`build_graph_structural_keyword_overlap_pair_rerank_request_batch_from_raw_candidates(...)`,
and
`fetch_graph_structural_keyword_overlap_pair_rerank_rows_for_repository_from_raw_candidates(...)`,
so the host no longer manually normalizes each raw candidate before batch or
repository-fetch dispatch.
That bounded host-side proof now also exercises that public fetch helper
directly and confirms that the missing-transport failure still resolves through
the Julia-owned structural-rerank route instead of a host-local adapter layer.
This crate now also has a plugin-owned live loopback for that same fetch seam:
the `graph_structural_exchange` test module launches the real
`.data/WendaoSearch.jl/scripts/run_search_service.jl` entrypoint in demo mode,
waits for `/graph/structural/rerank` to accept Flight connections, and proves
`fetch_graph_structural_keyword_overlap_pair_rerank_rows_for_repository_from_raw_candidates(...)`
can decode a live structural-rerank response without any host-side adapter.

The transport client now sends `x-wendao-schema-version` and defaults to the
`v1` WendaoArrow contract unless the repository plugin config overrides
`schema_version`. This crate also stamps `wendao.schema_version` onto outgoing
request batch metadata so the managed Julia Flight services see the same
request-side contract boundary as the Rust rerank path.

`validate_julia_arrow_response_batches` enforces the current `v1` response
shape before a future gateway integration accepts analyzer output:

- required columns: `doc_id`, `analyzer_score`, `final_score`
- `doc_id` must be unique and non-null
- `final_score` must be finite

`process_julia_flight_batches` is the thin runtime hook for future gateway
integration. It performs:

- Arrow Flight roundtrip via `xiuxian-wendao-runtime`'s negotiated client
- response schema-version enforcement
- `v1` Julia response validation before returning decoded record batches

## Graph-Structural Draft Contract

The first mixed-graph structural plugin routes now stage from this crate
instead of `xiuxian-wendao-runtime`.

- schema version: `v0-draft`
- structural rerank route: `/graph/structural/rerank`
- constraint filter route: `/graph/structural/filter`

That means:

- `xiuxian-wendao-runtime` still owns generic Flight transport mechanics such
  as route normalization and negotiated clients
- `xiuxian-wendao-julia` owns the Julia-specific semantic contract and
  repository-config interpretation for these structural plugin exchanges
- future host dispatch should import these Julia-owned route and validation
  surfaces from this crate rather than adding another runtime-local contract

## Validation

- `direnv exec . cargo test -p xiuxian-wendao-julia transport --lib`
- `direnv exec . cargo test -p xiuxian-wendao-julia --lib rerank_exchange`
- `direnv exec . cargo test -p xiuxian-wendao-julia graph_structural_exchange --lib`
- `direnv exec . cargo test -p xiuxian-wendao-julia graph_structural_projection --lib`
- `direnv exec . cargo test -p xiuxian-wendao-julia process_julia_flight_batches_against_real_wendaoarrow_service --lib`
- `direnv exec . cargo test -p xiuxian-wendao-julia real_wendaoarrow_metadata_example_roundtrip_decodes_trace_id_column --lib`
- `direnv exec . cargo test -p xiuxian-wendao-julia fetch_graph_structural_keyword_overlap_pair_rerank_rows_for_repository_from_raw_candidates_against_real_wendaosearch_demo_service --lib`
- `direnv exec . cargo check -p xiuxian-wendao-julia --lib`
- `direnv exec . cargo test -p xiuxian-wendao --test xiuxian-testing-gate test_agentic_expansion_pair_projects_into_julia_graph_structural_request`
- `direnv exec . cargo test -p xiuxian-wendao --test xiuxian-testing-gate test_agentic_expansion_pair_uses_julia_graph_structural_fetch_helper`
- `direnv exec . cargo check -p xiuxian-wendao --features julia --test xiuxian-testing-gate`

The real loopback tests now speak only to the Flight examples. They spawn
`.data/WendaoArrow.jl/scripts/run_stream_scoring_flight_server.sh` and
`.data/WendaoArrow.jl/scripts/run_stream_metadata_flight_server.sh`, wait for the
Flight socket to accept connections, then send the canonical request batches
through the runtime-owned negotiated Flight client. Those fixtures now use the
shared `julia_arrow_request_schema(...)` builder as well, so the official
example roundtrip receives the full WendaoArrow `v1` request shape instead of a
test-local reduced schema.

There is also a metadata-aware real loopback that targets
`.data/WendaoArrow.jl/scripts/run_stream_metadata_flight_server.sh`, sends a
request whose Arrow schema metadata includes `trace_id`, and asserts the Rust
side can decode the additive `trace_id` response column. That path now goes
through the production Flight client, so the test verifies request schema
metadata survives the real Flight API instead of only a hand-written HTTP
fixture.

The corresponding test support is now split under `src/plugin/test_support/`
into shared child/path helpers and official-example helpers, mirroring the
same semantic split used by `xiuxian-wendao` integration support.
That official-example layer now includes a real `WendaoSearch.jl` structural
demo launcher as well, so plugin-owned graph-structural fetch helpers can be
proven against a live Search child service without moving route logic back
into `xiuxian-wendao`.
