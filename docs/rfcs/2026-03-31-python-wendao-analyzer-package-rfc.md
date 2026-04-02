---
type: knowledge
title: "RFC: Python Wendao Analyzer Package Boundary"
category: "rfc"
status: "implemented"
authors:
  - codex
created: 2026-03-31
tags:
  - rfc
  - wendao
  - python
  - analyzer
metadata:
  title: "RFC: Python Wendao Analyzer Package Boundary"
---

# RFC: Python Wendao Analyzer Package Boundary

## 1. Summary

This RFC defines the package boundary between the transport substrate and any
future Python-local analyzer implementation.

The decision is:

1. `xiuxian-wendao-py` remains the Python transport and contract package
2. a future `xiuxian-wendao-analyzer` package is the correct home for
   Python-local analyzer logic
3. `xiuxian-wendao-analyzer` must depend on `xiuxian-wendao-py`
4. downstream users building custom analyzers over Arrow tables should depend
   directly on `xiuxian-wendao-py`

## 1.1 Operational Status

This RFC is now operationally implemented in beta form.

Implemented state:

1. `packages/python/xiuxian-wendao-analyzer/` exists as a real package
2. the package exposes analyzer-owned local and host-backed workflows
3. the package has runnable examples and onboarding docs
4. the package has real-host repo-search validation through
   `wendao_search_flight_server`

Still intentionally deferred:

1. plugin migration out of `xiuxian-wendao-py`
2. any live-host claim for analyzer-shaped rerank input rows
3. package-local Flight runtime ownership

## 2. Alignment

This RFC is governed by:

1. [2026-03-31-wendao-flight-query-contract-evolution-rfc.md](./2026-03-31-wendao-flight-query-contract-evolution-rfc.md)
2. [2026-03-29-python-arrow-flight-boundary-rfc.md](./2026-03-29-python-arrow-flight-boundary-rfc.md)
3. [2026-03-27-wendao-arrow-plugin-flight-rfc.md](./2026-03-27-wendao-arrow-plugin-flight-rfc.md)
4. [.data/WendaoArrow/README.md](../../.data/WendaoArrow/README.md)
5. [.data/WendaoAnalyzer/README.md](../../.data/WendaoAnalyzer/README.md)

## 3. Problem Statement

The repository now has a clear Julia package split:

1. `WendaoArrow` owns Arrow and Flight transport helpers plus shared contract
   handling
2. `WendaoAnalyzer` owns analyzer logic layered on top of that transport
   substrate

Python does not yet have that same boundary written down explicitly. Without
an explicit package split, `xiuxian-wendao-py` can drift from a transport
package into an analyzer runtime, which would blur ownership and make it
harder for downstream users to know which dependency they actually need.

## 4. Goals

This RFC has the following goals:

1. keep `xiuxian-wendao-py` thin and transport-first
2. reserve a clean sibling-package boundary for Python-local analyzer logic
3. give downstream Python analyzer authors one stable substrate dependency
4. align the Python package model with the existing Julia
   `WendaoArrow`/`WendaoAnalyzer` split

## 5. Non-Goals

This RFC does not:

1. require immediate implementation of `xiuxian-wendao-analyzer`
2. move Rust-owned query or rerank semantics into Python
3. redefine the current `xiuxian-wendao-py` transport contracts
4. force downstream users to depend on an official analyzer package

## 6. Package Roles

### `xiuxian-wendao-py`

`xiuxian-wendao-py` owns:

1. Arrow and Flight transport access for Python consumers
2. typed request and response helpers for Rust-owned Wendao routes
3. schema and metadata helpers needed to work with those routes
4. a stable Python substrate for downstream analyzer packages

`xiuxian-wendao-py` does not own:

1. Python-local rerank strategies
2. Python-local analyzer runtime configuration
3. scientific-stack scoring implementations
4. analyzer product semantics that are not already Rust-owned contracts

### `xiuxian-wendao-analyzer`

A future `xiuxian-wendao-analyzer` package should own:

1. Python-local analyzer strategies
2. Python scientific ecosystem integration
   - for example `numpy`, `scipy`, `torch`, `jax`, or `sklearn`
3. analyzer runtime configuration local to Python execution
4. analyzer-facing convenience APIs built on top of `xiuxian-wendao-py`

`xiuxian-wendao-analyzer` should not own:

1. Rust host orchestration
2. Rust transport headers or canonical route naming
3. Rust fallback policy or host-owned timeout policy
4. the shared transport substrate itself

## 7. Dependency Rule

The dependency direction must be:

1. `xiuxian-wendao-analyzer -> xiuxian-wendao-py`
2. `xiuxian-wendao-py` must not depend on `xiuxian-wendao-analyzer`

This preserves a clean layering model:

1. Rust host/runtime owns the service contract
2. `xiuxian-wendao-py` exposes that contract to Python
3. analyzer packages build on top of that substrate

## 8. Downstream User Guidance

Downstream users fall into two groups:

1. users building custom analyzers
   - depend on `xiuxian-wendao-py`
2. users wanting an official Python analyzer implementation
   - depend on `xiuxian-wendao-analyzer` once it exists

This means the official analyzer package is optional, not mandatory, for the
Python ecosystem.

## 9. Initial Package Proposal

The first `xiuxian-wendao-analyzer` package should stay narrow.

### 9.1 Minimal Layout

The initial package layout should be:

1. `packages/python/xiuxian-wendao-analyzer/`
2. `src/xiuxian_wendao_analyzer/`
3. `tests/`
4. `README.md`

The first module split should be:

1. `config.py`
   - analyzer-local strategy and runtime settings
2. `strategies.py`
   - analyzer implementations such as linear blend or similarity-first
3. `runtime.py`
   - analyzer-facing entrypoints that consume `xiuxian-wendao-py`
4. `plugin.py`
   - optional analyzer-package plugin convenience layer if needed

### 9.2 First Public API

The initial public API should favor a small surface:

1. `AnalyzerConfig`
2. `build_analyzer(config)`
3. `analyze_table(table, *, analyzer)`
4. `analyze_rows(rows, *, analyzer)`
5. one transport-facing integration helper built on `xiuxian-wendao-py`
   rather than raw Flight assembly

The first package should not try to publish every possible analyzer helper in
its first revision.

### 9.3 Integration Seam

The first integration seam should be:

1. `xiuxian-wendao-analyzer` consumes Arrow tables or typed rows coming from
   `xiuxian-wendao-py`
2. `xiuxian-wendao-analyzer` may use `WendaoTransportClient`,
   typed repo-search rows, typed rerank rows, and typed rerank responses from
   `xiuxian-wendao-py`
3. `xiuxian-wendao-analyzer` should not reimplement raw Arrow/Flight metadata
   assembly that already exists in `xiuxian-wendao-py`

### 9.4 Transitional Rule for Existing Python Surface

The current `xiuxian-wendao-py` modules:

1. `analyzer.py`
2. `plugin.py`
3. `scaffold.py`

should be treated as transitional authoring and compatibility surface, not as
the long-term home for new analyzer semantics.

Until `xiuxian-wendao-analyzer` exists:

1. they may remain for compatibility
2. they should not absorb new analyzer-scoring product semantics by default
3. new Python-local analyzer strategy work should target the sibling package
   proposal in this RFC

### 9.5 Initial Migration Map

The initial migration map should be:

1. keep in `xiuxian-wendao-py` as substrate:
   - `WendaoTransportClient`
   - typed repo-search request and response helpers
   - typed rerank request and response helpers
   - Arrow/Flight metadata and schema helpers
2. keep in `xiuxian-wendao-py` as transitional compatibility:
   - `run_analyzer(...)`
   - `run_analyzer_with_table(...)`
   - `run_analyzer_with_rows(...)`
   - `run_analyzer_with_mock_rows(...)`
   - `build_mock_flight_info(...)`
   - `WendaoAnalyzerPlugin`
   - `WendaoAnalyzerPluginManifest`
3. future analyzer-package ownership:
   - strategy objects such as linear blend or similarity-first analyzers
   - analyzer-local scoring and ranking helpers
   - analyzer-local runtime config
   - analyzer-package convenience entrypoints built on the substrate
4. keep compatibility-only until a migration actually lands:
   - `plugin.py`
   - `scaffold.py`
   - analyzer authoring convenience surface currently exported from
     `xiuxian_wendao_py.__init__`

The rule is:

1. transport and typed contract access stay in `xiuxian-wendao-py`
2. analyzer semantics move to `xiuxian-wendao-analyzer`
3. convenience wrappers already published from `xiuxian-wendao-py` may remain
   for compatibility, but should stop being the default growth path

### 9.6 V0 Scaffold Plan

The first implementation slice for `xiuxian-wendao-analyzer` should be planned
as a narrow scaffold, not as a full migration.

#### V0 Substrate Reuse

The first package revision should reuse these `xiuxian-wendao-py` symbols
directly instead of wrapping raw Flight behavior again:

1. `WendaoTransportClient`
2. `WendaoTransportConfig`
3. `WendaoTransportEndpoint`
4. `WendaoFlightRouteQuery`
5. `WendaoRepoSearchRequest`
6. `WendaoRepoSearchResultRow`
7. `WendaoRerankRequestRow`
8. `WendaoRerankResultRow`
9. `repo_search_request(...)`
10. `parse_repo_search_rows(...)`
11. `build_rerank_request_table(...)`
12. `parse_rerank_response_rows(...)`

#### V0 Compatibility Surface

The first package revision should not migrate these surfaces yet:

1. `run_analyzer(...)`
2. `run_analyzer_with_table(...)`
3. `run_analyzer_with_rows(...)`
4. `run_analyzer_with_mock_rows(...)`
5. `WendaoAnalyzerPlugin`
6. `build_profiled_analyzer_plugin(...)`
7. `plugin_from_manifest(...)`
8. `WendaoAnalyzerPluginManifest`
9. scaffold profile and sample-row helpers

Those surfaces may remain published from `xiuxian-wendao-py` until a later
migration RFC or implementation slice moves them deliberately.

#### V0 Test Shape

The first package revision should prove only:

1. one analyzer strategy can score a local Arrow table
2. one analyzer strategy can consume typed rerank rows or a rerank-shaped
   table built through `xiuxian-wendao-py`
3. one integration helper can fetch through `WendaoTransportClient` and invoke
   the analyzer without reimplementing transport metadata assembly

The first package revision should not require:

1. package-local Flight server ownership
2. a full plugin scaffold migration
3. parity with every current compatibility helper in `xiuxian-wendao-py`

## 10. Acceptance Criteria

This boundary is considered adopted when:

1. `xiuxian-wendao-py` documentation explicitly describes itself as the
   transport substrate
2. the successor Flight RFC explicitly reserves Python-local analyzer logic
   for a sibling package
3. this RFC exists as the normative package-boundary reference
4. future Python analyzer work cites this RFC instead of expanding
   `xiuxian-wendao-py` by default

## 11. Decision

Adopt the following package boundary:

1. `xiuxian-wendao-py` is the Python-side analogue of `WendaoArrow`
2. `xiuxian-wendao-analyzer` is the planned Python-side analogue of
   `WendaoAnalyzer`
3. downstream custom analyzers should build on `xiuxian-wendao-py`
4. official Python-local analyzer logic belongs in the sibling analyzer package
