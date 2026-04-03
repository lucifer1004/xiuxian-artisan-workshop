# xiuxian-wendao-runtime

Runtime support crate for Wendao transport contracts, Flight server/client
plumbing, runtime config resolution, and artifact rendering helpers.

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

## Verification

Current runtime verification for this lane:

- `direnv exec . cargo clippy -p xiuxian-wendao-runtime --tests --features julia -- -D warnings`
- `direnv exec . cargo test -p xiuxian-wendao-runtime --features julia`
