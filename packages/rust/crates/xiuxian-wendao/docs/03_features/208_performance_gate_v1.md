# Wendao Performance Gate V1

:PROPERTIES:
:ID: feat-wendao-performance-gate-v1
:PARENT: [[index]]
:TAGS: feature, performance, gate, nextest, criterion
:STATUS: ACTIVE
:VERSION: 1.0
:END:

## Overview

`xiuxian-wendao` now integrates the `xiuxian-testing` performance kernel behind
crate features and keeps a single gate entrypoint at
`tests/xiuxian-testing-gate.rs`.

No default test semantics were changed: performance suites run only when
feature flags are enabled.

## Feature Flags

- `performance`: enables performance gate tests and forwards
  `xiuxian-testing/performance`.
- `performance-stress`: depends on `performance` and enables long-running
  ignored stress suites.

## Test Mounting Strategy

The unified gate mounts:

- `tests/performance/*` under `#[cfg(feature = "performance")]`
- `tests/performance/stress/*` under `#[cfg(feature = "performance-stress")]`
- required integration suites via `#[path = "integration/*.rs"]` under
  `#[cfg(not(feature = "performance"))]`
- a source modularity contract gate (`ModularityRulePack`) that runs in default
  mode and fails on `Error/Critical` findings

Root-level test wrappers are intentionally minimized. Integration tests are
mounted from `tests/xiuxian-testing-gate.rs` instead of duplicated
`tests/*_test.rs` pass-through files.
Current root Rust entry files are:
`tests/xiuxian-testing-gate.rs`.

Suite layout:

- `latency_*`: PR-fast p95 latency gates.
- `throughput_*`: PR-fast throughput floor gates.
- `stress/*`: Nightly-only ignored stress gates.
- `gateway_search`: formal `tests/performance/gateway_search.rs` now mounts six
  serialized warm-cache gateway cases under the `performance` feature
  (`repo_module_search`, `repo_symbol_search`, `repo_example_search`,
  `repo_projected_page_search`, `studio_code_search`, and
  `search_index_status`) through the narrow
  `gateway::studio::perf_support` fixture surface.
- `studio_gateway_search_perf`: the feature-gated lib calibration lane keeps
  the same six warm-cache cases reportable inside the crate test module, while
  only the aggregate smoke suite remains `#[ignore]`.

## Budget Strategy

Default budgets are auditable Rust constants in test code and can be overridden
by environment variables in CI.

This supports SLO-driven tightening without changing test wiring.

The gateway warm-cache lane now resolves defaults through `RUNNER_OS` runner
profiles and accepts per-case overrides via:

- `XIUXIAN_WENDAO_GATEWAY_PERF_<CASE>_P95_MS`
- `XIUXIAN_WENDAO_GATEWAY_PERF_<CASE>_MIN_QPS`
- `XIUXIAN_WENDAO_GATEWAY_PERF_<CASE>_MAX_ERROR_RATE`

`<CASE>` is the uppercase gateway case id such as
`REPO_MODULE_SEARCH`, `REPO_SYMBOL_SEARCH`, `REPO_EXAMPLE_SEARCH`,
`REPO_PROJECTED_PAGE_SEARCH`, `STUDIO_CODE_SEARCH`, or
`STUDIO_SEARCH_INDEX_STATUS`.

## Reporting Contract

Each run persists a JSON report under:

- `.run/reports/xiuxian-wendao/perf/*`
- `.run/reports/xiuxian-wendao/perf/stress/*`

## Criterion Layer

A Criterion bench target is available at:

- `benches/wendao_performance.rs`

It mirrors gate themes (`related_ppr`, `narration`) for trend analysis but is
not used as a PR blocker.

## CI Topology

- PR mainline CI (`.github/workflows/ci.yaml`, `.github/workflows/checks.yaml`)
  intentionally does not run Wendao performance lanes.
- Wendao performance gates run in the dedicated workflow:
  `.github/workflows/xiuxian-wendao-performance-gates.yaml`.
- `quick` profile is manual-only (`workflow_dispatch`).
- `nightly` profile runs on schedule and supports manual dispatch.
- Bench compile in nightly uses the fast lane and stays advisory
  (`continue-on-error`) to avoid blocking stability gates on runner noise.

## Validation Commands

- Preferred quick entrypoint:
  `direnv exec . just rust-wendao-performance-gate`
- PR quick gate:
  `direnv exec . cargo nextest run -p xiuxian-wendao --features performance --test xiuxian-testing-gate`
- Formal gateway perf listing:
  `direnv exec . cargo test -p xiuxian-wendao --features performance --test xiuxian-testing-gate -- --list`
- Formal gateway targeted proof:
  `direnv exec . cargo nextest run -p xiuxian-wendao --features performance --test xiuxian-testing-gate -E 'test(performance::gateway_search::studio_code_search_perf_gate_reports_warm_cache_latency_formal_gate) | test(performance::gateway_search::search_index_status_perf_gate_reports_query_telemetry_summary_formal_gate)'`
- Feature-gated lib perf lane:
  `direnv exec . cargo test -p xiuxian-wendao --features performance gateway::studio::studio_gateway_search_perf_tests --lib`
- Default integration + structure gate:
  `direnv exec . cargo test -p xiuxian-wendao --test xiuxian-testing-gate`
- Nightly stress gate:
  `direnv exec . cargo nextest run -p xiuxian-wendao --features "performance performance-stress" --test xiuxian-testing-gate --run-ignored ignored-only`
- Bench fast compile proof (recommended):
  `direnv exec . env CARGO_PROFILE_BENCH_LTO=off CARGO_PROFILE_BENCH_CODEGEN_UNITS=16 CARGO_PROFILE_BENCH_DEBUG=0 cargo check -p xiuxian-wendao --features performance --benches`
- Bench no-run lane (heavy, advisory):
  `direnv exec . env CARGO_PROFILE_BENCH_LTO=off CARGO_PROFILE_BENCH_CODEGEN_UNITS=16 CARGO_PROFILE_BENCH_DEBUG=0 CARGO_TARGET_DIR=.cache/cargo-target/xiuxian-wendao-bench cargo bench -p xiuxian-wendao --features performance --bench wendao_performance --no-run`
