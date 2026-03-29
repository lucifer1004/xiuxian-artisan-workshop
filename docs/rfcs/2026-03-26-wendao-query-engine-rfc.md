---
type: knowledge
title: "RFC: Wendao Query Engine on DataFusion, LanceDB, and link_graph"
category: "rfc"
status: "draft"
authors:
  - codex
created: 2026-03-26
tags:
  - rfc
  - wendao
  - datafusion
  - lancedb
  - link-graph
  - query-engine
metadata:
  title: "RFC: Wendao Query Engine on DataFusion, LanceDB, and link_graph"
---

# RFC: Wendao Query Engine on DataFusion, LanceDB, and link_graph

## 1. Summary

This RFC proposes a **SQL-first, operator-first query engine** for Wendao built around the existing project trajectory:

1. `DataFusion` is the planning and execution kernel for relational and analytical work.
2. `LanceDB` and `Lance` provide vector, full-text, hybrid, reranking, and random-access retrieval primitives.
3. `link_graph` provides graph expansion, neighborhood traversal, and graph-derived evidence.
4. `Arrow RecordBatch`, Arrow IPC, and eventually Flight SQL provide the canonical result boundary.

This RFC explicitly rejects three directions as the primary architecture:

1. `GraphQL` as the main query engine.
2. A fully custom Wendao-specific DSL.
3. A transport-first architecture centered on an external tool protocol.

The proposed design keeps the external query surface easy for humans, CLI workflows, and LLMs to generate while preserving a native execution path for the current Rust ecosystem.

## 2. Motivation

Wendao is no longer a pure keyword search service. It is evolving into a mixed retrieval and reasoning substrate with three hard requirements:

1. **Relational and analytical selection** over structured metadata and search outputs.
2. **Vector and hybrid retrieval** over Lance-based indexes.
3. **Graph expansion and path evidence** over `link_graph`.
4. **Low-latency point lookups and neighborhood jumps** without row-group inflation.

The current ecosystem decision is already clear:

1. Arrow IPC is a hard requirement.
2. DataFusion is the analytical engine.
3. LanceDB is the retrieval substrate.
4. Wendao domain logic is graph-aware through `link_graph`.

The unresolved architectural question is therefore not "which trendy API protocol should we adopt?" but:

**What query model best composes these three execution domains without degrading CLI usability, LLM usability, or execution efficiency?**

## 3. Problem Statement

The system currently lacks a single query architecture that can:

1. Express retrieval, filtering, ranking, and graph expansion in one execution plan.
2. Stay friendly to CLI workflows and direct operator use.
3. Stay easy for LLMs to generate with bounded prompting cost.
4. Avoid coupling user-facing syntax to internal execution details.
5. Reuse DataFusion instead of reimplementing a new planner from scratch.

If this remains unresolved, Wendao risks drifting into one of two bad outcomes:

1. Query logic fragments into separate REST endpoints, ad hoc search options, and graph-specific handler trees.
2. A new custom DSL emerges that is expensive to teach to LLMs and expensive to evolve safely.

## 4. Goals

This RFC has the following goals:

1. Define a query architecture native to the Arrow, DataFusion, and LanceDB ecosystem.
2. Keep the initial user-facing surface simple enough for SQL-oriented CLI use and LLM generation.
3. Make graph and retrieval behavior first-class at the operator level.
4. Standardize result materialization as Arrow-native batches.
5. Preserve room for future remote execution over Flight SQL.

## 5. Non-Goals

This RFC does not attempt to:

1. Implement a complete ISO GQL, SQL/PGQ, openCypher, or GraphQL engine.
2. Replace DataFusion with a separate graph database or document database.
3. Standardize the final Studio frontend API shape.
4. Define mutation semantics in the first iteration.
5. Commit the system to one external language forever.

## 6. Design Principles

### 6.1 SQL-first, not SQL-only

The engine should accept a SQL-first query surface because:

1. SQL is the most mature query form for CLI usage.
2. LLMs reliably generate SQL-like structures under constrained prompting.
3. DataFusion already provides parsing, logical planning, and optimization infrastructure.

However, Wendao must not pretend that graph traversal and hybrid retrieval are ordinary joins. The system is therefore **SQL-first, not SQL-only**.

### 6.2 Operator-first execution

The stable abstraction boundary is not the query string. The stable boundary is the **logical operator model**.

This RFC therefore defines Wendao around first-class operators such as:

1. `scan_structured`
2. `vector_search`
3. `fts_search`
4. `hybrid_search`
5. `rerank`
6. `graph_expand`
7. `graph_neighbors`
8. `evidence_project`

### 6.3 Arrow-native outputs

All query paths should converge on Arrow-native outputs:

1. `RecordBatch` in process
2. Arrow IPC for local and embedded consumers
3. Flight SQL for future remote clients and query shells

### 6.4 Lance-native storage for hot planes

Wendao should treat `Lance` as the preferred physical format for hot search planes and graph-adjacent retrieval planes.

This is a deliberate storage choice:

1. Wendao frequently performs point lookups by identifier.
2. Wendao frequently performs short-hop graph navigation against sparse neighborhoods.
3. Wendao frequently benefits from narrow-column filtering before wide-column payload fetch.

Parquet remains useful for cold exports, offline interchange, and archival workloads. It should not be the default physical format for the interactive search plane.

### 6.5 Execution by capability partition

Wendao should not force every capability through one physical engine.

Instead:

1. DataFusion owns relational planning, filtering, projection, aggregation, and relational composition.
2. LanceDB owns vector and hybrid retrieval primitives.
3. `link_graph` owns graph expansion, neighborhood extraction, and path evidence.

The Wendao planner composes these domains.

### 6.6 Late materialization by default

Wendao should adopt late materialization as a core execution rule for search-heavy paths.

The intended pattern is:

1. scan narrow columns such as `id`, `repo`, `path`, `score`, and compact filter fields
2. compute selection masks and candidate sets inside DataFusion
3. materialize wide payload columns such as `line_text`, `snippet`, `content`, or graph evidence only after the candidate set is stable

This principle should guide both schema layout and operator design.

## 7. Why Not GraphQL

GraphQL remains viable as a future presentation or aggregation layer, but it is not suitable as the primary query engine for Wendao.

### 7.1 GraphQL is a contract language, not an execution model

GraphQL is effective for frontend field selection and aggregation, but weak for:

1. ranking semantics
2. retrieval operators
3. graph traversal constraints
4. search planner composition
5. analytical query reuse

### 7.2 GraphQL does not align with the current execution substrate

The current stack is built around DataFusion, Arrow, LanceDB, and graph execution. GraphQL adds another query surface without reducing planning complexity.

### 7.3 GraphQL is not the best LLM surface here

For this project, LLMs benefit more from:

1. constrained SQL generation
2. typed parameters
3. explicit table functions
4. predictable tabular results

than from free-form GraphQL document generation.

## 8. Why Not a Custom DSL

A custom DSL is explicitly discouraged for the first design iteration.

### 8.1 LLM teaching cost is too high

Every custom grammar token increases prompt burden, repair burden, and schema drift risk.

### 8.2 CLI ergonomics get worse

Operators and developers already understand SQL-like filters, projections, and limits. They do not benefit from learning a new bespoke grammar unless there is strong semantic gain.

### 8.3 Internal evolution becomes harder

A custom DSL tends to freeze syntax before the operator model is stable. That is the wrong order for this system.

## 9. Proposed Architecture

## 9.1 Layered Model

The proposed architecture has four layers.

### Layer 1: Query Surface

Initial query entry is SQL-first with controlled Wendao extensions.

Examples of supported patterns:

```sql
SELECT path, score
FROM vector_search(
  index => 'repo_content',
  query => 'hybrid retrieval planner',
  limit => 20
)
WHERE repo = 'xiuxian-artisan-workshop'
ORDER BY score DESC;
```

```sql
SELECT *
FROM graph_neighbors(
  seed_path => 'docs/01_core/wendao/architecture/id-resolution-mechanism.md',
  direction => 'both',
  hops => 2,
  limit => 100
);
```

```sql
SELECT *
FROM rerank(
  hybrid_search(
    index => 'repo_content',
    query => 'planner rewrite strategy',
    limit => 50
  ),
  strategy => 'cross_encoder',
  top_k => 10
);
```

### Layer 2: Wendao Logical Operators

The query surface is lowered into Wendao-specific logical operators.

At minimum:

1. `StructuredScan`
2. `VectorSearch`
3. `FtsSearch`
4. `HybridSearch`
5. `Rerank`
6. `GraphExpand`
7. `GraphNeighbors`
8. `EvidenceProject`
9. `ResultUnion`

These operators are the real contract for system evolution.

### Layer 3: Physical Capability Backends

Logical operators map onto three backends:

1. `DataFusion`
2. `LanceDB`
3. `link_graph`

Mapping examples:

1. `StructuredScan` -> DataFusion `TableProvider`
2. `VectorSearch` -> LanceDB query builder or vector scan integration
3. `HybridSearch` -> LanceDB hybrid retrieval plus Wendao post-processing
4. `GraphNeighbors` -> `link_graph` traversal execution
5. `EvidenceProject` -> RecordBatch materialization and schema normalization

### Layer 3A: Storage policy

The storage policy is explicitly tiered:

1. `Lance` for hot search planes, adjacency-like graph planes, and random-access-heavy result sets
2. `Parquet` for cold archival, bulk interchange, and offline export surfaces
3. Arrow in-memory batches for execution intermediates

### Layer 4: Materialization Boundary

Every path emits normalized Arrow data with explicit schemas so downstream layers do not need to understand backend-specific response objects.

## 9.2 Query Surface Strategy

The first iteration should support **SQL plus Wendao table functions**.

This is preferred over introducing a new graph clause language because:

1. it lands faster on DataFusion
2. it stays easy to call from CLI
3. it stays prompt-friendly for LLMs
4. it keeps syntax local to specific high-value operators

Only after the operator model stabilizes should Wendao consider adding graph clauses such as `MATCH`-style sugar.

## 9.3 Planner Strategy

The planner should be implemented in two phases:

### Phase A: SQL lowering

Use DataFusion parsing and logical planning wherever possible.

### Phase B: Wendao rewrite

Add a Wendao-specific planner stage that:

1. recognizes retrieval and graph table functions
2. validates operator-specific arguments
3. rewrites supported expressions into Wendao logical operators
4. attaches execution capability metadata

This preserves DataFusion as the backbone while allowing Wendao-specific semantics to remain explicit.

## 9.4 Resource-aware planning

The planner should be allowed to degrade concurrency under system pressure.

The initial policy should make planning sensitive to:

1. file descriptor pressure
2. IO wait pressure
3. available memory budget
4. lane count and partition fan-out

This is specifically relevant for multi-repo scans, refresh storms, and large retrieval jobs where planner-level concurrency decisions can be the difference between degraded service and stable service.

## 9.5 Internal IR

Substrait should be evaluated as an **internal interchange boundary**, not as the user-facing language.

Potential uses:

1. caching executable plans
2. exporting relational subplans
3. future remote query planning
4. debugging planner outputs

Substrait should not replace the Wendao logical operator layer because retrieval and graph execution still require Wendao-native semantics.

## 10. Canonical Operators

The system should stabilize around a minimal initial operator set.

### 10.1 `vector_search`

Inputs:

1. index
2. query or embedding
3. filter
4. limit

Outputs:

1. record identifier
2. score
3. metadata projection
4. optional snippet

Backend:

1. LanceDB

### 10.2 `fts_search`

Inputs:

1. index
2. query text
3. filter
4. limit

Backend:

1. LanceDB full-text or text-search backend

### 10.3 `hybrid_search`

Inputs:

1. query text
2. candidate limit
3. weighting strategy
4. optional repo or namespace filters

Backend:

1. LanceDB retrieval
2. Wendao score fusion logic

### 10.4 `rerank`

Inputs:

1. candidate relation
2. reranker strategy
3. top-k

Backend:

1. Wendao reranker pipeline

### 10.5 `graph_neighbors`

Inputs:

1. seed identifier or path
2. direction
3. hops
4. limit

Backend:

1. `link_graph`

### 10.6 `graph_expand`

Inputs:

1. seed relation
2. expansion strategy
3. edge filters
4. hop constraints

Backend:

1. `link_graph`

### 10.7 `evidence_project`

Inputs:

1. relation from retrieval or graph expansion
2. projection requirements
3. optional snippet or provenance settings

Backend:

1. DataFusion projection
2. Wendao materialization helpers

### 10.8 `column_mask`

Inputs:

1. narrow relation
2. predicate set
3. candidate limit or threshold

Backend:

1. DataFusion filter and mask generation

Purpose:

1. make late materialization explicit
2. preserve narrow-first execution on wide retrieval planes

### 10.9 `payload_fetch`

Inputs:

1. candidate relation carrying row identifiers or stable ids
2. payload column set

Backend:

1. Lance-backed random access fetch
2. optional DataFusion projection over fetched payloads

Purpose:

1. delay wide column reads until after candidate pruning

## 11. Result Schema Contract

Wendao should standardize a small set of result schema families so different execution paths are interoperable.

### 11.1 Retrieval hit schema

Required fields:

1. `id`
2. `path`
3. `score`
4. `repo`
5. `source`
6. `snippet`

### 11.2 Graph node schema

Required fields:

1. `node_id`
2. `path`
3. `category`
4. `distance`
5. `edge_kind`

### 11.3 Evidence schema

Required fields:

1. `subject`
2. `predicate`
3. `object`
4. `provenance`
5. `confidence`

By keeping these schema families explicit, Wendao avoids leaking backend-specific objects into higher layers.

## 11.4 Graph edge relation

Required fields:

1. `src_id`
2. `dst_id`
3. `edge_kind`
4. `repo`
5. `weight`
6. `provenance`

This relation should be representable as an Arrow-native and optionally Lance-backed edge plane so graph traversal can operate over partitioned edge data rather than requiring monolithic in-memory graph loads.

## 12. CLI and LLM Fit

## 12.1 CLI fit

The proposed design is CLI-friendly because:

1. SQL remains the primary entry point.
2. Query fragments are composable and readable.
3. Table functions are easy to shell-quote and script.
4. Results are tabular by default.

## 12.2 LLM fit

The design is LLM-friendly because:

1. the grammar remains mostly SQL
2. only a small number of operator names must be learned
3. operator arguments can be strongly typed
4. the system can provide schema and function catalogs directly in prompts

This is materially easier than requiring an LLM to learn a completely custom language.

## 12.3 Execution locality and SIMD

Text and filter-heavy operators should run inside the Arrow and DataFusion kernel where possible.

This implies:

1. prefer Arrow-native string kernels and SIMD-friendly execution for `contains`, prefix, and token-sensitive filters
2. avoid bouncing filter execution back into per-row Rust callback paths
3. keep predicate evaluation vectorized whenever an Arrow kernel already exists

## 13. Transport Strategy

Transport should be treated as a separate concern from query semantics.

Recommended progression:

1. in-process Rust API
2. Arrow IPC for local and embedded boundaries
3. Flight SQL for remote query shells and external analytical clients

The project should avoid prematurely binding query semantics to GraphQL or another presentation-oriented transport.

## 14. Alternatives Considered

### 14.1 GraphQL-first architecture

Rejected as primary because it is better suited to frontend aggregation than query execution.

### 14.2 Full custom Wendao DSL

Rejected because it imposes high LLM teaching cost and unnecessary maintenance risk.

### 14.3 openCypher-first engine

Valuable as a design reference for graph semantics, but not selected as the first architecture because it does not map as directly onto DataFusion and Arrow.

### 14.4 Datalog-first engine

Valuable as a design reference for recursive and graph-aware query planning, but not selected as the first architecture because it is heavier for CLI and LLM workflows.

### 14.5 SQL/PGQ as the immediate target

Attractive in the long term, but the first implementation should focus on Wendao-specific operators on top of DataFusion rather than adopting a broader graph language standard prematurely.

## 15. Implementation Plan

### Phase 0: Operator contract

1. Define Rust request and schema types for the initial operator set.
2. Standardize canonical Arrow result schemas.
3. Isolate backend adapters for DataFusion, LanceDB, and `link_graph`.
4. Declare Lance as the default hot-plane storage format.

### Phase 1: SQL + table functions

1. Expose `vector_search`, `fts_search`, `hybrid_search`, `rerank`, and `graph_neighbors`.
2. Materialize outputs as Arrow relations.
3. Ship an internal query shell for operator testing.
4. Add `column_mask` and `payload_fetch` to make late materialization explicit.

### Phase 2: Planner rewrites

1. Introduce Wendao logical operators.
2. Add planner rewrites and validation passes.
3. Capture explain plans for debugging and observability.
4. Add runtime-aware partition and fan-out control hooks.

### Phase 3: Transport hardening

1. Add Arrow IPC interfaces.
2. Evaluate Flight SQL endpoint support.
3. Stabilize remote consumer contracts.

### Phase 3A: Graph columnarization

1. Materialize adjacency and edge relations as Arrow-native and Lance-backed graph planes where beneficial.
2. Allow graph traversal inputs to operate over partitioned edge relations rather than monolithic in-memory graph loads.
3. Evaluate recursive query support and SQL/PGQ-inspired traversal sugar only after operator semantics are stable.

### Phase 4: Syntax sugar

Only after operator semantics stabilize:

1. consider graph-oriented query sugar
2. consider SQL/PGQ-inspired clauses
3. consider a frontend-specific GraphQL facade if required

## 16. Risks

### 16.1 Planner complexity drift

If too much logic stays in handler code rather than the operator layer, the query engine will degenerate into endpoint orchestration.

### 16.2 Result schema instability

If retrieval and graph outputs do not converge on stable Arrow schemas, every consumer will fork its own normalization rules.

### 16.3 Premature syntax expansion

If the project introduces graph clauses before the operator layer is stable, the syntax will harden around incomplete semantics.

### 16.4 Backend leakage

If LanceDB or `link_graph` native objects leak above the materialization boundary, future planner evolution becomes expensive.

### 16.5 Resource storms under high parallelism

If the planner does not account for file descriptor pressure, IO wait, and partition fan-out, Wendao will continue to risk degraded throughput and `os error 24`-style incidents during multi-repo scans or refresh storms.

## 17. Open Questions

1. Should `hybrid_search` return a relation directly, or should it always be wrapped by `rerank` in the public surface?
2. Should `graph_expand` accept only seed relations, or also literal identifiers and document paths?
3. Should the engine expose embeddings directly as a typed Arrow column, or hide embedding vectors behind retrieval operators?
4. How much of the initial planner rewrite should live in DataFusion extension points versus Wendao-local planning code?
5. When should Flight SQL become an officially supported remote interface?

## 18. Decision

This RFC proposes the following architectural decision:

1. Wendao will use a **SQL-first** query surface.
2. Wendao will define a **native logical operator layer** for retrieval and graph execution.
3. DataFusion will remain the central planning and execution kernel.
4. LanceDB and `link_graph` will remain specialized backends, not peer query languages.
5. Lance will remain the preferred hot-plane physical format.
6. Arrow-native outputs will be the canonical result boundary.

## 19. Appendix: Example Query Shape

The following query illustrates the intended style of the first public surface:

```sql
WITH candidates AS (
  SELECT *
  FROM hybrid_search(
    index => 'repo_content',
    query => 'datafusion planner extension',
    limit => 40
  )
),
ranked AS (
  SELECT *
  FROM rerank(candidates, strategy => 'cross_encoder', top_k => 12)
)
SELECT g.path, g.category, r.score, r.snippet
FROM ranked r
JOIN graph_neighbors(
  seed_path => r.path,
  direction => 'both',
  hops => 1,
  limit => 8
) g
ON g.path = r.path
ORDER BY r.score DESC;
```

This shape keeps the system:

1. readable in the terminal
2. generatable by LLMs
3. composable over DataFusion
4. explicit about where retrieval and graph semantics enter the plan

## 20. Rust Operator Draft

This section sketches the minimal Rust-facing operator contract for the first implementation.

The goal is not to freeze an API immediately. The goal is to establish the ownership boundary between:

1. query parsing and lowering
2. logical operator construction
3. backend capability execution
4. Arrow-native materialization

### 20.1 Core traits

```rust
pub trait WendaoOperator: Send + Sync + std::fmt::Debug {
    fn kind(&self) -> WendaoOperatorKind;
    fn output_schema(&self) -> arrow_schema::SchemaRef;
}

pub trait WendaoExecutableOperator: WendaoOperator {
    fn backend(&self) -> WendaoBackendKind;
}

pub trait WendaoOperatorExecutor: Send + Sync {
    fn execute(
        &self,
        op: std::sync::Arc<dyn WendaoExecutableOperator>,
        ctx: &WendaoExecutionContext,
    ) -> xiuxian_types::Result<SendableRecordBatchStream>;
}
```

### 20.2 Operator kind enumeration

```rust
pub enum WendaoOperatorKind {
    StructuredScan,
    VectorSearch,
    FtsSearch,
    HybridSearch,
    Rerank,
    GraphNeighbors,
    GraphExpand,
    ColumnMask,
    PayloadFetch,
    EvidenceProject,
    ResultUnion,
}
```

### 20.3 Execution context

The execution context should carry:

1. DataFusion session state
2. LanceDB query handles
3. `link_graph` runtime handles
4. memory and concurrency budgets
5. tracing and explain-plan sinks

Example shape:

```rust
pub struct WendaoExecutionContext {
    pub session_state: std::sync::Arc<datafusion::execution::context::SessionState>,
    pub lance: std::sync::Arc<LanceRuntime>,
    pub link_graph: std::sync::Arc<LinkGraphRuntime>,
    pub resources: WendaoResourceBudget,
    pub explain: WendaoExplainSink,
}
```

### 20.4 Request structs

The public operator request structs should be strongly typed and decoupled from SQL token structure.

Examples:

```rust
pub struct VectorSearchOp {
    pub index: String,
    pub query_text: Option<String>,
    pub embedding: Option<Vec<f32>>,
    pub filter: Option<datafusion_expr::Expr>,
    pub limit: usize,
    pub projected_columns: Vec<String>,
}

pub struct GraphNeighborsOp {
    pub seed: WendaoSeed,
    pub direction: GraphDirection,
    pub hops: usize,
    pub limit: usize,
    pub edge_kinds: Vec<String>,
}

pub struct PayloadFetchOp {
    pub source: std::sync::Arc<dyn WendaoOperator>,
    pub payload_columns: Vec<String>,
    pub fetch_key: WendaoFetchKey,
}
```

### 20.5 Why this shape

This contract keeps SQL parsing out of the operator runtime and makes it possible to:

1. execute operators from SQL
2. execute operators from a future CLI shell API
3. test operators without invoking the SQL parser
4. materialize explain plans at the operator boundary

## 21. DataFusion Extension Mapping

This section describes where Wendao should lean on DataFusion directly and where it should extend it.

### 21.1 Use `TableProvider` for stable relations

Use `TableProvider` for:

1. repo metadata tables
2. materialized search result sets
3. graph edge relations exposed as Arrow-native relations
4. Lance-backed structured tables where projection and filter pushdown matter

### 21.2 Use table functions for retrieval entry points

Use SQL table functions for:

1. `vector_search(...)`
2. `fts_search(...)`
3. `hybrid_search(...)`
4. `graph_neighbors(...)`

These are the most natural user-facing primitives because they return relations directly.

### 21.3 Use custom logical nodes where semantics exceed SQL defaults

Use custom logical nodes for:

1. `rerank`
2. `column_mask`
3. `payload_fetch`
4. `graph_expand`
5. future recursive graph operators

These operations should not be modeled as opaque scalar UDFs because they affect:

1. schema
2. cost
3. materialization timing
4. resource planning

### 21.4 Use custom optimizer and analyzer rules

Wendao should add dedicated rules for:

1. late materialization rewrites
2. filter pushdown into Lance-backed scans
3. graph fan-out limiting
4. column pruning before payload fetch
5. runtime-aware partition reduction under pressure

### 21.5 Use session and runtime configuration explicitly

The Wendao integration layer should own DataFusion session and runtime tuning such as:

1. `target_partitions`
2. memory pool sizing
3. spill behavior
4. execution batch sizing
5. scan concurrency caps

This policy should live in Wendao runtime configuration, not in ad hoc per-query flags.

## 22. `link_graph` Edge Plane Schema

This section proposes the initial Arrow schema for columnar graph execution.

### 22.1 Base edge schema

```text
src_id: utf8
dst_id: utf8
edge_kind: utf8
repo: utf8
weight: float32
provenance: utf8
src_path: utf8
dst_path: utf8
src_category: utf8
dst_category: utf8
created_at: timestamp[us, utc]?
updated_at: timestamp[us, utc]?
```

### 22.2 Optional traversal acceleration columns

The following columns should remain optional until profiling proves value:

1. `src_hash: uint64`
2. `dst_hash: uint64`
3. `hop_cost: float32`
4. `namespace: utf8`
5. `partition_key: utf8`

### 22.3 Partitioning guidance

The initial partition strategy should prefer operational simplicity:

1. partition first by repo
2. optionally partition next by edge family or namespace
3. avoid over-partitioning by node id at the storage layer

The planner can still prune effectively using repo and edge-kind filters without exploding file counts.

### 22.4 Why an edge plane matters

An Arrow-native edge plane gives Wendao three benefits:

1. graph-adjacent operations become explainable in the same execution system as retrieval
2. multi-repo graph scans can run through DataFusion-style partition fan-out
3. graph evidence can be joined back to retrieval hits without bespoke object translation

## 23. Execution Sequence: `column_mask -> payload_fetch`

The hot-path search execution should follow an explicit narrow-first pipeline.

### 23.1 Intended sequence

```text
SQL/table function
  -> Wendao logical operator tree
  -> narrow scan over ids, scores, repo, path, compact filters
  -> DataFusion filter/projection
  -> column_mask candidate relation
  -> optional rerank / top-k trim
  -> payload_fetch on surviving ids only
  -> evidence_project / final projection
  -> Arrow RecordBatch output
```

### 23.2 Pseudocode

```rust
let candidates = vector_search(index, query, limit = 256)?;
let masked = column_mask(candidates, predicate_set)?;
let ranked = rerank(masked, top_k = 24)?;
let payload = payload_fetch(ranked, ["path", "snippet", "line_text"])?;
let output = evidence_project(payload, final_projection)?;
```

### 23.3 Operational rule

The system should avoid reading wide text payloads before at least one of the following is true:

1. final candidate set is below a configured threshold
2. reranking has completed
3. the client explicitly requests a payload-heavy plan

### 23.4 Observability requirements

This path should emit explain-plan and runtime counters for:

1. rows scanned in narrow phase
2. rows surviving mask generation
3. rows fetched in payload phase
4. bytes read in payload phase
5. planner reductions caused by resource pressure

These metrics are required to validate that late materialization is actually reducing IO amplification rather than merely moving it around.
