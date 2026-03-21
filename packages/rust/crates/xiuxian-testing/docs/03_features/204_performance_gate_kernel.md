# Performance Gate Kernel V1

:PROPERTIES:
:ID: xiuxian-testing-performance-gate-kernel-v1
:PARENT: [[../index]]
:TAGS: feature, performance, tests, gates
:STATUS: ACTIVE
:END:

## Overview

`xiuxian-testing` now ships an opt-in `performance` kernel for reusable Rust
performance gate tests.

The kernel is feature-gated and activated with:

```bash
cargo test -p xiuxian-testing --features performance
```

## Public API Surface

The `performance` module exports:

- `PerfBudget`: p50/p95/p99 latency, throughput floor, and error-rate ceiling.
- `PerfRunConfig`: warmup rounds, measured samples, timeout, and concurrency.
- `PerfReport`: summary metrics, latency quantiles, metadata, and report path.
- `run_sync_budget(...)`: sync operation runner with sampling and report output.
- `run_async_budget(...)`: async operation runner with timeout-aware sampling.
- `assert_perf_budget(...)`: unified failure assertion for performance budgets.

## Report Contract

Reports use schema id `xiuxian-testing.perf-report.v1` and are persisted to:

- `${PRJ_RUNTIME_DIR}/reports/<suite>/<case>-<timestamp>.json`
- default fallback: `.run/reports/<suite>/<case>-<timestamp>.json`

This keeps CI and local runs aligned under one stable report location contract.

## Policy Integration

Test-structure validation now permits `tests/performance/` as a first-class
directory under crate test roots, so performance suites can stay organized
without root-level Rust file whitelisting.

## Validation Targets

- `direnv exec . cargo test -p xiuxian-testing --features performance`
