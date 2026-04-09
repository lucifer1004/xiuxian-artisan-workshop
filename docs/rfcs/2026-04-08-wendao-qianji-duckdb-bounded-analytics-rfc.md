---
type: knowledge
title: "RFC: DuckDB as a Bounded In-Process Analytic Lane for Wendao and Qianji"
category: "rfc"
status: "draft"
authors:
  - codex
created: 2026-04-08
tags:
  - rfc
  - wendao
  - qianji
  - duckdb
  - duckdb-rs
  - arrow
  - datafusion
  - valkey
metadata:
  title: "RFC: DuckDB as a Bounded In-Process Analytic Lane for Wendao and Qianji"
---

# RFC: DuckDB as a Bounded In-Process Analytic Lane for Wendao and Qianji

## 1. Summary

This RFC proposes adding `duckdb-rs` as a bounded in-process analytic lane for
Wendao and Qianji.

The primary decision is:

1. DuckDB may be used for request-scoped or bounded-lived SQL analytics over
   Arrow-first relations
2. Wendao keeps Arrow Flight as the external business boundary
3. the shared query system inside `xiuxian-wendao` remains DataFusion-led for
   now
4. Valkey remains the hot-cache and transient-state layer
5. the vector store remains the embedding and ANN layer

This RFC does not make DuckDB the new primary database, the new vector store,
or the new checkpoint coordinator.

## 2. Alignment

This RFC aligns with the following stable references:

1. [RFC: Wendao Query Engine on DataFusion, LanceDB, and link_graph](2026-03-26-wendao-query-engine-rfc.md)
2. [RFC: Wendao Arrow-First Plugin Protocol with Flight-First Transport](2026-03-27-wendao-arrow-plugin-flight-rfc.md)
3. [Search Queries Architecture](../../packages/rust/crates/xiuxian-wendao/docs/03_features/210_search_queries_architecture.md)
4. [RFC: Data-Centric Workflow Orchestration on Wendao Relations](../../packages/rust/crates/xiuxian-qianji/docs/rfcs/2026-03-26-qianji-data-centric-workflow-rfc.md)
5. [Spec: Qianji Runtime Config Layering](../../packages/rust/crates/xiuxian-qianji/docs/20_specs/2026-04-07-qianji-runtime-config-layering.md)
6. [xiuxian-wendao-runtime README](../../packages/rust/crates/xiuxian-wendao-runtime/README.md)

The paired execution tracking for this RFC follows an active blueprint and
ExecPlan, but canonical RFCs do not link hidden workspace tracking paths
directly.

## 3. Audit Snapshot

### 3.1 Flight Is the Current Wendao Business Boundary

The current repository already encodes stable Flight business routes through
`xiuxian-wendao-runtime` query-contract constants and route tests. The active
business family includes:

1. `/search/intent`
2. `/search/attachments`
3. `/search/references`
4. `/search/symbols`
5. `/search/ast`
6. `/analysis/markdown`
7. `/analysis/code-ast`

DuckDB must not change that external contract.

### 3.2 The Shared Query System Is DataFusion-Based Today

`xiuxian-wendao` currently centralizes shared query translation under
`src/search/queries/`, where SQL, FlightSQL, GraphQL, REST-style query
adapters, and CLI query entrypoints are described as one DataFusion-backed
query family.

This matters because the DuckDB lane proposed here is intentionally narrower:
it is not a silent replacement of the current shared query core.

### 3.3 A Bounded Local Markdown SQL Lane Already Exists

The repository already contains a concrete bounded local relation workflow:

1. `xiuxian-wendao::search::queries::sql::bounded_work_markdown`
2. `xiuxian-qianji::workdir::query`

The default bounded path still uses DataFusion over an in-memory `markdown`
table for bounded work surfaces. The same lane now also has a feature-gated
`DuckDbLocalRelationEngine` pilot helper for request-scoped local execution.
That keeps the correctness baseline and the DuckDB pilot in the same bounded
workload, which makes it the most credible first pilot shape because the
workflow is already local, bounded, relation-oriented, and Arrow-friendly.

### 3.4 Qianji Runtime Config Already Keeps Checkpoint Ownership Explicit

`xiuxian-qianji` currently documents and resolves checkpoint persistence as a
Valkey-backed runtime-config lane with TOML-first precedence.

This RFC therefore must not repurpose DuckDB into checkpoint storage or a
replacement for Qianji runtime-state coordination.

### 3.5 A Bounded DuckDB Landing Now Exists

There is now a bounded DuckDB integration inside `xiuxian-wendao` and
`xiuxian-wendao-runtime`.

The currently landed Wendao slices are:

1. a narrow local relation-engine seam plus a feature-gated `src/duckdb/`
   bridge
2. typed `search.duckdb` runtime config resolution with TOML-first precedence
3. request-scoped registration policy that can keep small Arrow worksets
   virtual or materialize them through `appender-arrow`
4. a bounded markdown pilot that can execute through DataFusion or DuckDB
   while keeping the default path DataFusion-backed
5. additive bounded execution metadata for engine choice, row and byte counts,
   registration time, local execution time, and materialization state
6. workspace Arrow `58.1.0` alignment and `arrow-flight` `flight-sql`
   enablement across the Wendao crates that participate in this lane

Qianji does not yet have a stage-local DuckDB pilot, and the shared query
system remains DataFusion-led.

## 4. Problem Statement

Wendao and Qianji now have a gap between two realities:

1. both systems already center Arrow-first relation handoff
2. many bounded analytics tasks still fall back to either request-scoped
   DataFusion everywhere or custom Rust row traversal

That gap appears in three places:

1. bounded markdown and diagnostics analytics inside Wendao
2. workflow-stage audit/reduce and consistency checks inside Qianji
3. repo/runtime status or explain-facing local joins that benefit from fast
   in-process SQL without introducing a new external service

Without a clear boundary, DuckDB adoption risks failing in one of two ways:

1. it becomes too small and ad hoc to justify the dependency
2. it grows into a new central storage policy and blurs Valkey, vector, and
   query-core ownership

## 5. Goals

This RFC has the following goals:

1. introduce DuckDB as a bounded in-process analytic helper over Arrow-first
   relations
2. keep Arrow Flight unchanged as the external Wendao business boundary
3. let Qianji consume relation-level analytic results without taking ownership
   of retrieval planning or storage policy
4. preserve DataFusion as the current shared query core until later evidence
   justifies a wider change
5. require explain and telemetry coverage for the DuckDB lane from the start

## 6. Non-Goals

This RFC does not attempt to:

1. replace DataFusion globally in one step
2. make DuckDB the main external query protocol
3. make DuckDB the new cache, state, or checkpoint layer
4. make DuckDB the new vector store
5. commit the workspace to a dedicated shared DuckDB crate before bounded
   pilots exist

## 7. Why `duckdb-rs`

At the time of writing, the upstream `duckdb` crate documents the following
properties that match this lane:

1. ergonomic Rust bindings with in-memory and file-backed connection support
2. `bundled` builds for low-friction local and CI setup
3. `vtab-arrow` for Arrow virtual-table integration
4. `appender-arrow` for efficient Arrow bulk ingest
5. `parquet` and `json` feature flags for file-oriented analytic inputs
6. `vscalar` and `vscalar-arrow` support for custom scalar functions

That combination makes `duckdb-rs` a plausible bounded analytic helper for
Arrow-native relations without adding a second network service.

The current bounded Wendao landing only depends on `bundled` and
`appender-arrow`. Other upstream features such as `vtab-arrow`, `parquet`,
`json`, and custom scalar support remain optional future expansion points
rather than current repository requirements.

## 8. Architectural Decision

### 8.1 Core Decision

Adopt DuckDB only as a bounded internal analytic lane.

The intended shape is:

1. external clients still see Wendao Flight business routes and existing query
   surfaces
2. Wendao and Qianji may register Arrow batches into a local relation engine
   for bounded SQL work
3. DuckDB is one implementation of that local relation-engine seam
4. DataFusion remains the shared query core unless a later bounded slice proves
   a wider change is worth the cost

### 8.2 Package Ownership

#### `xiuxian-wendao`

Wendao owns:

1. search-plane and analysis business semantics
2. relation registration over Wendao-owned corpora and worksets
3. internal selection of a bounded local relation engine for Wendao-owned
   analytics
4. explain and telemetry emitted for those analytics

#### `xiuxian-qianji`

Qianji owns:

1. workflow-stage orchestration
2. stage-level audit/reduce/consistency narratives
3. consumption of relation-engine results inside workflow stages
4. workflow-facing explain binding

Qianji does not gain ownership of Wendao retrieval planning, storage policy,
or external DuckDB exposure.

#### `xiuxian-wendao-runtime`

The runtime crate owns only the host-side Wendao runtime concerns that would
be needed if Wendao embeds DuckDB:

1. typed host config
2. temp/spill directory policy
3. host bootstrap or long-lived connection helpers

It does not become the owner of search semantics or workflow-stage logic.

#### Valkey

Valkey continues to own:

1. hot cache
2. transient coordination
3. checkpoint-like runtime state
4. other explicit fast-state roles already assigned to it

#### Vector Store

The vector layer continues to own:

1. embeddings
2. ANN retrieval
3. vector-index lifecycle

### 8.3 Module Landing Strategy

The first implementation landing has stayed bounded inside `xiuxian-wendao`:

```text
packages/rust/crates/xiuxian-wendao/src/duckdb/
  mod.rs
  runtime.rs
  connection.rs
  arrow.rs
  engine.rs
```

Responsibilities for the current landed shape are:

1. `runtime.rs`: feature gate and typed policy inputs
2. `connection.rs`: connection bootstrap and lifecycle
3. `arrow.rs`: Arrow batch registration and result decode
4. `engine.rs`: local relation-engine policy, request-scoped registration,
   query execution, and bounded engine metadata exposure

The current bounded landing does not yet need separate `registration.rs`,
`query.rs`, or `telemetry.rs` files. If later slices prove real cross-package
reuse or broader query-surface integration, those separations may be justified
then.

## 9. Execution Model

### 9.1 Narrow Local Relation-Engine Seam

The correct abstraction boundary is a narrow local relation engine, not direct
DuckDB calls everywhere.

One acceptable shape is:

```rust
trait LocalRelationEngine {
    fn register_batches(&self, name: &str, batches: &[RecordBatch]) -> Result<()>;
    fn query_arrow(&self, sql: &str) -> Result<Vec<RecordBatch>>;
}
```

This keeps the current architecture honest:

1. DataFusion remains valid
2. DuckDB can be piloted without a flag day
3. bounded analytics can choose the right internal engine without changing the
   external contract

### 9.2 Registration Strategy

The first two registration modes should be:

1. ephemeral request-scoped Arrow registration for one-shot analytics
2. bounded materialized registration when the same rows are reused across
   multiple joins, windows, or diagnostics queries

The default preference should be Arrow virtual registration first, with
materialization only when repeat use or spill pressure justifies it.

### 9.3 Runtime Policy

The bounded DuckDB host lane should preserve current repo conventions:

1. TOML-first config precedence
2. explicit feature gating
3. request-scoped or bounded-lived usage only
4. project-aware path resolution through the existing runtime/path helpers

Example configuration shape:

```toml
[search.duckdb]
enabled = true
database_path = ":memory:"
temp_directory = "$PRJ_CACHE_HOME/duckdb/tmp"
threads = 4
materialize_threshold_rows = 200000
prefer_virtual_arrow = true

[qianji.duckdb]
enabled = true
database_path = "$PRJ_DATA_HOME/qianji/duckdb/workflow.db"
temp_directory = "$PRJ_CACHE_HOME/qianji/duckdb/tmp"
```

These keys are architectural placeholders in this RFC. Exact naming and
resolution rules must be revalidated when implementation expands further. The
`search.duckdb` keys above now have a bounded Wendao landing, while the
`qianji.duckdb` example remains future-facing until a Qianji stage-local pilot
exists.

## 10. First Pilot Targets

### 10.1 Wendao Bounded Markdown and Diagnostics Analytics

The existing bounded-work markdown SQL lane is the best first Wendao pilot:

1. the workload is already local and bounded
2. the rows are already normalized into a relation-friendly shape
3. the current DataFusion lane provides a correctness baseline
4. the external Flight business contract does not need to change

### 10.2 Qianji Audit and Reduce Stages

The next likely pilot is stage-local relation analytics over workflow-held
Arrow batches, especially:

1. `audit_step`
2. `reduce_step`
3. contradiction or consistency joins
4. explain-support rollups

These are relation-oriented workloads, but they still sit above retrieval and
storage ownership.

### 10.3 Repo and Runtime Diagnostics

Status, maintenance, and explain-facing analytics are a third candidate:

1. repo corpus status
2. cache or degraded-state diagnostics
3. maintenance and compaction summaries
4. workflow-stage statistics

These surfaces often benefit from local joins and aggregations without needing
new external APIs.

## 11. Telemetry and Explain

The DuckDB lane must participate in the same explain discipline as the rest of
the stack.

Minimum execution metadata should include:

1. input batch count
2. input rows and bytes
3. registration time
4. SQL execution time
5. output rows and bytes
6. virtual versus materialized registration choice
7. spill or temp usage indicators

The current bounded Wendao pilot already reports input batch count, input
rows and bytes, registration time, local query execution time, output rows and
bytes, and materialization state. Spill or temp-usage indicators are still
future work.

The causal narrative should remain explicit:

1. Wendao explains why a relation exists
2. DuckDB explains what bounded SQL happened over that relation
3. Qianji explains why a workflow stage used that relation

## 12. Gates

### 12.1 Functional Gates

Any later pilot must preserve:

1. unchanged Flight business contracts
2. correct Arrow schema roundtrips
3. explicit Valkey and vector ownership boundaries
4. reproducible Qianji stage outputs

### 12.2 Performance Gates

For a bounded pilot to expand, it should prove at least one of:

1. materially lower latency than the current implementation
2. materially lower peak memory
3. materially lower maintenance complexity while keeping comparable
   performance

### 12.3 Correctness Gates

Pilot outputs must remain auditable:

1. status and diagnostics queries must agree with current canonical surfaces
2. audit and contradiction joins must match current rule outputs
3. row counts, schema, and stage outputs must remain explainable

## 13. Risks and Revisit Triggers

### 13.1 Main Risks

1. two-engine complexity can create maintenance overhead
2. Arrow-friendly does not guarantee zero-copy in every path
3. bundled builds can increase build size or build time
4. scope creep can silently turn DuckDB into a storage-policy catch-all

### 13.2 Revisit Triggers

Revisit this direction if:

1. the first pilots fail to show a meaningful bounded-use benefit
2. the runtime/config burden outweighs the local analytics gain
3. later evidence suggests DataFusion alone is sufficient
4. a future shared crate becomes justified by real cross-package reuse

## 14. Rollout Phases and Current Status

### Phase 0: RFC and Boundaries [landed]

1. the canonical RFC, blueprint, ExecPlans, and nearest package-doc sync
   points are now present
2. the external Flight boundary and DataFusion-led shared query-core rule are
   explicit

### Phase 1: Narrow Relation-Engine Seam [landed in bounded Wendao form]

1. the bounded local relation-engine abstraction is present
2. `search.duckdb` runtime/config policy is landed with TOML-first precedence
3. current DataFusion paths remain intact

### Phase 2: Wendao Pilot [landed in bounded form]

1. the bounded-work markdown lane can execute through DataFusion or DuckDB
2. the request-scoped registration policy is real and engine-visible
3. additive bounded metadata now reports engine choice, rows, bytes, timing,
   and materialization state
4. broader performance gating and broader diagnostics pilots are still open

### Phase 3: Qianji Pilot [future]

1. pilot one audit/reduce-stage relation workload
2. wire stage-level explain and telemetry

### Phase 4: Expand or Hold [future]

Use the gates to decide whether to:

1. expand the DuckDB lane
2. keep it bounded to a few high-value pilots
3. stop at documentation and narrow local experiments

## 15. Final Decision

The final decision of this RFC is:

1. use `duckdb-rs` if DuckDB is adopted in this workspace
2. keep Arrow Flight as the Wendao external business boundary
3. keep Valkey as the cache and transient-state layer
4. keep the vector store as the embedding and ANN layer
5. add DuckDB only as a bounded in-process analytic lane
6. keep DataFusion as the current shared query core until later evidence says
   otherwise

## Appendix A: Current Bounded Dependency Set

The current bounded Wendao landing uses the following DuckDB dependency:

```toml
[dependencies]
duckdb = { version = "=1.10501.0", default-features = false, features = [
  "bundled",
  "appender-arrow",
] }
```

The current workspace Arrow baseline for this lane is `58.1.0`, and the
participating Wendao crates enable `arrow-flight` with `flight-sql`.

## Appendix B: One-Line Ownership Map

1. Arrow Flight: business protocol boundary
2. DuckDB: bounded in-process analytic lane
3. DataFusion: current shared query core
4. Valkey: hot cache and transient state
5. Vector store: embedding and ANN layer
6. Qianji: workflow orchestration
7. Wendao: retrieval, graph, and business semantics
