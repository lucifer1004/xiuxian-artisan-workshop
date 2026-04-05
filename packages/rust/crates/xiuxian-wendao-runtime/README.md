# xiuxian-wendao-runtime

Runtime support crate for Wendao transport contracts, Flight server/client
plumbing, runtime config resolution, and artifact rendering helpers.

## Responsibility

`xiuxian-wendao-runtime` owns generic host behavior for the Wendao split.
If a boundary depends on runtime state, config resolution, transport
negotiation, or live host assembly, it belongs here instead of in
`xiuxian-wendao-core`.

Current ownership:

- runtime config models and resolvers
- settings merge, parse, and directory helpers
- transport negotiation and Flight client/server helpers
- runtime artifact resolve/render helpers

## Non-Goals

Do not use `xiuxian-wendao-runtime` as the home for:

- stable contract record ownership that plugins can share directly
- knowledge-graph, retrieval, or storage semantics
- language-specific intelligence implementation

Those belong in `xiuxian-wendao-core` or `xiuxian-wendao` respectively.

## Selection Rule

If the code reads environment state, touches config files, negotiates
transport, materializes clients/servers, or otherwise depends on deployment
context, prefer `xiuxian-wendao-runtime`.

For the full three-package boundary matrix, see
[`../xiuxian-wendao/docs/06_roadmap/417_wendao_package_boundary_matrix.md`](../xiuxian-wendao/docs/06_roadmap/417_wendao_package_boundary_matrix.md).

## Transport Server Test Layout

The transport server tests now follow a feature-folder layout under
`src/tests/transport/server/` instead of a single flat `server.rs`.

- `assertions.rs`: shared test assertions and Flight decoding helpers
- `construction.rs`: service-construction boundaries
- `fixtures.rs`: shared service builders and Flight batch decode helpers
- `metadata.rs`: request-header validation coverage
- `providers.rs`: recording route-provider doubles
- `request_headers.rs`: shared metadata/header builders
- `rerank.rs`: rerank contract tests
- `routes/`: route-family integration coverage split by concern

## Transport Query Contract Layout

The query-contract surface now follows the same folder-first rule.
`src/transport/query_contract.rs` is only the stable re-export seam, while the
implementation lives under `src/transport/query_contract/` and the contract
tests live under `src/transport/query_contract/tests/`.

- `common.rs`: route normalization plus descriptor helpers
- `search/`: repo search, attachments, definition, autocomplete, and AST
  contract constants
- `query/`: SQL query contract
- `query/sql/headers.rs`: stable SQL route and metadata-header constants
- `query/sql/validation.rs`: DataFusion-backed read-only SQL validation
- `vfs/`: content/resolve/scan contracts
- `graph/`: neighbors and topology contracts
- `analysis/`: markdown and code-AST request validation
- `repo/`: repo analysis and refine-doc contracts
- `rerank/`: rerank schema, batch validation, and scoring
- `tests/`: query contract coverage split by the same feature families

## Verification

Current runtime verification for this lane:

- `direnv exec . cargo clippy -p xiuxian-wendao-runtime --tests --features transport -- -D warnings`
- `direnv exec . cargo test -p xiuxian-wendao-runtime --features transport`
- `direnv exec . cargo test -p xiuxian-wendao-runtime query_contract --features transport`
- `direnv exec . cargo clippy -p xiuxian-wendao -p xiuxian-wendao-runtime --all-targets --all-features -- -D warnings`
