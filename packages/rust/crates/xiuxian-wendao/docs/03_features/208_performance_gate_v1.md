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

Suite layout:

- `latency_*`: PR-fast p95 latency gates.
- `throughput_*`: PR-fast throughput floor gates.
- `stress/*`: Nightly-only ignored stress gates.

## Budget Strategy

Default budgets are auditable Rust constants in test code and can be overridden
by environment variables in CI.

This supports SLO-driven tightening without changing test wiring.

## Reporting Contract

Each run persists a JSON report under:

- `.run/reports/xiuxian-wendao/perf/*`
- `.run/reports/xiuxian-wendao/perf/stress/*`

## Criterion Layer

A Criterion bench target is available at:

- `benches/wendao_performance.rs`

It mirrors gate themes (`related_ppr`, `narration`) for trend analysis but is
not used as a PR blocker.

## Validation Commands

- PR quick gate:
  `direnv exec . cargo nextest run -p xiuxian-wendao --features performance --test xiuxian-testing-gate`
- Nightly stress gate:
  `direnv exec . cargo nextest run -p xiuxian-wendao --features "performance performance-stress" --test xiuxian-testing-gate --run-ignored ignored-only`
- Bench compile lane:
  `direnv exec . cargo bench -p xiuxian-wendao --features performance --no-run`
