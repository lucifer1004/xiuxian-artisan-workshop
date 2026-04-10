# Search Queries Architecture

:PROPERTIES:
:ID: feat-search-queries-architecture
:PARENT: [[index]]
:TAGS: feature, search, flight, datafusion, graphql, sql
:STATUS: ACTIVE
:VERSION: 1.0
:END:

## Overview

The Studio search gateway must distinguish between two different concerns:

1. native Wendao business protocols
2. a shared query system

Native Flight routes remain the primary business protocol surface. They expose
Wendao-specific capabilities such as semantic search, graph navigation, VFS
resolution, and analysis routes. These routes are not just query translation.

The shared `queries/` system, by contrast, exists to translate a request
language into one local relation-execution plan over the Wendao search plane.
SQL is the first fully landed query adapter. FlightSQL, GraphQL, REST-style
query APIs, and a CLI `query` subcommand belong to the same architectural
family even when they are introduced later.

Today that shared family still contains substantial DataFusion residue, but it
should not be read as the long-term database execution owner. The current
DuckDB migration direction is narrower and more explicit:

- DuckDB is the intended database and query-execution lane for search-side
  Parquet publications, bounded FlightSQL statement reads, and bounded
  diagnostics SQL
- DataFusion retains value only where Rust still needs live Arrow compute over
  generated or request-scoped batches that have not become published Parquet
  or an explicitly registered DuckDB relation

## Target Layering

The intended layering is:

- native Flight business routes: Wendao capability surfaces
- shared queries system: one semantic query boundary inside `xiuxian-wendao`
- query adapters: SQL, FlightSQL, GraphQL, REST, CLI
- cross-language Arrow substrate:
  `WendaoArrow`, pyarrow, julia-arrow, and Flight move `RecordBatch`
  contracts between Rust and Julia analyzers
- DuckDB database execution:
  search-side SQL over published Parquet plus bounded request-scoped analytic
  relations
- residual DataFusion compute:
  Rust-side live Arrow compute, batch shaping, and migration-baseline support
- Arrow result encoding: returned through Flight or another adapter surface

This means Flight is not a replacement for DuckDB or DataFusion. Flight is a
transport or business-protocol surface. The cross-language Arrow substrate is
separate from the bounded database execution lane, and residual DataFusion use
should be read as Arrow-native compute support rather than as a second
long-term search database.

## Current Ownership Matrix

The current code-proven ownership split for Wendao search storage is:

- mutable runtime state: in-process coordinator state, not DuckDB and not
  Valkey by default
- shared cache and fast shared state: Valkey-backed when that role is enabled
- published read-mostly corpora: Parquet on disk
- current bounded SQL execution over Arrow or Parquet: DataFusion or DuckDB
  during the migration
- external business protocol: native Flight first, with FlightSQL as a bounded
  query-adapter surface

That means the current system does not collapse everything into DuckDB. The
boundary is narrower and more specific:

- `repo_index` runtime status, queue membership, active repo ordering, and
  pending work still live inside the process coordinator
- repo analysis cache and repo-search query cache still use in-memory plus
  Valkey-backed cache paths
- published local and repo corpora are persisted as Parquet, not as DuckDB
  files
- DuckDB is a bounded local execution lane over Arrow relations and Parquet
  publications
- native Flight and bounded FlightSQL remain protocol surfaces, not storage
  owners

### Cross-Language Arrow Substrate and Residual DataFusion Value

The current code also shows a separate boundary that should not be collapsed
into the DuckDB lane:

- `WendaoArrow`, pyarrow, julia-arrow, and Flight own the cross-language
  Arrow `RecordBatch` substrate between Rust and Julia analyzers
- Rust Julia integration paths such as parser-summary, graph-structural, and
  rerank exchange are Arrow-first request and response contracts before any
  database execution question appears
- DuckDB belongs downstream of that transport substrate when Wendao executes
  search-side SQL over published Parquet or bounded request-scoped analytic
  relations
- DataFusion's residual value therefore belongs only in Rust-side live Arrow
  compute, request and response shaping, or migration-baseline work where the
  data is still a generated Arrow workset rather than a Parquet publication or
  DuckDB-owned relation

So the intended long-term interpretation is:

- cross-language Arrow transport: `WendaoArrow` and Flight
- search and diagnostics database execution: DuckDB
- mutable state and shared cache: in-process plus Valkey-backed roles
- residual Arrow-native compute inside Rust: DataFusion only where DuckDB is
  not the right tool

This distinction matters because "search storage" in Wendao is now split
across different layers on purpose:

- Arrow: request-scoped relation and payload format
- Parquet: published persisted columnar corpus format
- DuckDB: search-side query and analytics execution over published Parquet or
  bounded request-scoped relations
- DataFusion: residual live Arrow compute inside Rust during the migration
- Valkey: cache and explicit fast-state roles
- in-process coordinator memory: mutable runtime state

### Repo Lane Ownership

For the repo lane specifically, the current code shows this narrower split:

- `repo_index` mutable state:
  in-process coordinator memory owns queue membership, active repo ordering,
  per-repo status maps, and the aggregate `status_snapshot`
- repo analysis cache:
  in-memory cache plus `ValkeyAnalysisCache`
- repo search query-result cache:
  in-memory query cache plus `ValkeyAnalysisCache`
- repo publication data:
  published `repo_entity` and `repo_content_chunk` corpora persisted as
  Parquet
- repo query execution:
  `ParquetQueryEngine` selects DataFusion or DuckDB over those Parquet
  publications
- repo diagnostics protocol surfaces:
  native Flight and JSON handlers may register request-scoped Arrow relations
  and run bounded diagnostics SQL, but those routes do not become storage
  owners

So the current repo lane should be read as:

- state: in-process
- shared cache: Valkey-backed
- publication: Parquet
- execution: DataFusion or DuckDB
- protocol: Flight, JSON, and bounded FlightSQL

### Local Corpus Lane Ownership

For the local corpus lane, the current code shows this narrower split:

- local publication ownership:
  `local_symbol`, `reference_occurrence`, `attachment`, and
  `knowledge_section` now rewrite published data directly to Parquet
- local epoch discovery and prewarm:
  Parquet-only for already-migrated local corpora
- local query execution:
  search and hydration paths read those Parquet publications through
  `ParquetQueryEngine`, which selects DataFusion or DuckDB
- local cache:
  no separate DuckDB-owned local corpus cache layer is introduced by these
  cuts
- local protocol surfaces:
  gateway search routes and bounded FlightSQL statements read from the local
  corpus Parquet publications, but they do not become storage owners

So the current local corpus lane should be read as:

- publication: Parquet-first
- execution: DataFusion or DuckDB over Parquet
- cache/state: not reassigned to DuckDB
- protocol: gateway routes and bounded FlightSQL over those publications

### Mutable State and Shared Cache Ownership

For mutable state and shared cache, the current code shows this split:

- `repo_index` mutable state:
  `RepoIndexCoordinator` still owns per-repo statuses, fingerprints, active
  ordering, the aggregate snapshot, and the pending queue in in-process
  `RwLock` and `Mutex` fields
- search-plane mutable runtime state:
  `SearchPlaneCoordinator` and `SearchPlaneService` still keep per-corpus
  runtime maps, maintenance state, dispatch runtime, repo runtime generation,
  and query telemetry in in-process synchronization primitives
- repository analysis cache:
  analysis outputs still land in an in-memory cache, with shared reuse going
  through `ValkeyAnalysisCache` where configured
- repository search query-result cache:
  query payload reuse still lands in an in-memory cache plus
  `ValkeyAnalysisCache`
- search-plane shared cache:
  `SearchPlaneCache` remains the Valkey-backed cache entrypoint for
  manifests, leases, and short-lived search-plane cache values where enabled
- DuckDB role:
  DuckDB remains an execution and bounded analytics lane over Arrow and
  Parquet relations; it does not own mutable coordinator state or the shared
  cache backplane

So the current mutable-state and shared-cache split should be read as:

- mutable state: in-process
- shared cache: Valkey-backed where enabled
- publication: Parquet
- execution: DataFusion or DuckDB
- protocol: Flight, JSON, and bounded FlightSQL over those owned surfaces

### Protocol Surface Ownership

For protocol surfaces, the current code shows this split:

- native Flight routes:
  `StudioSearchFlightRouteProvider` dispatches route requests to search
  handlers and returns `SearchFlightRouteResponse` batches plus metadata, but
  it does not own persisted publications or execution engines
- bounded FlightSQL:
  `StudioFlightSqlService` exposes discovery and statement-query surfaces over
  the shared query system and the published Parquet query-engine seam; it may
  route statements into DataFusion or DuckDB execution, but it does not become
  a storage owner
- JSON gateway handlers:
  Studio HTTP/JSON handlers call the underlying search-plane and repo/local
  search methods, then serialize response payloads; they are protocol adapters
  rather than persistence owners
- underlying ownership:
  protocol surfaces sit on top of Parquet publications, in-process state, and
  Valkey-backed caches without changing who owns those underlying layers

So the current protocol-surface split should be read as:

- protocol: native Flight, bounded FlightSQL, and JSON/HTTP
- publication: Parquet
- state: in-process
- cache: Valkey-backed where enabled
- execution: DataFusion or DuckDB selected underneath those protocol surfaces

The bounded DuckDB analytics proposal is tracked separately in
[RFC: DuckDB as a Bounded In-Process Analytic Lane for Wendao and Qianji](../../../../../../docs/rfcs/2026-04-08-wendao-qianji-duckdb-bounded-analytics-rfc.md).
That RFC keeps the external Flight contract unchanged and keeps the shared
query system's current DataFusion residue explicit; DuckDB is scoped to the
target search-side database execution lane and to internal request-scoped or
bounded-lived analytic execution.

The first bounded implementation slice under that RFC is now landed too.
`xiuxian-wendao` has a local `src/duckdb/` bridge that exposes one bounded
local relation-engine seam, and the bounded-work markdown lane now uses that
seam while remaining DataFusion-backed. DuckDB-specific execution is still
feature-gated scaffolding rather than the default shared query path.

The next bounded pilot under the same RFC is landed too. The bounded-work
markdown query owner now exposes an explicit
`query_bounded_work_markdown_payload_with_engine(...)` helper, and the new
`DuckDbLocalRelationEngine` can register Arrow batches through
`appender-arrow` and return Arrow-native query batches. The default
`query_bounded_work_markdown_payload(...)` path still instantiates the
DataFusion engine explicitly, so the residual shared-query DataFusion path
stays intact while the DuckDB pilot remains opt-in and request-scoped.

The next engine-policy slice under the same RFC is landed too.
`DuckDbLocalRelationEngine` now honors the published `search.duckdb`
registration policy instead of always materializing relations. Small bounded
worksets now register as request-scoped DuckDB temp views over a
Wendao-owned Arrow virtual table, while larger worksets or explicitly
non-virtual configurations fall back to appender-backed materialization. This
keeps the default shared query core unchanged while making the DuckDB pilot's
runtime policy real.

The next explain-facing slice under the same RFC is landed too. The bounded
markdown SQL payload now exposes additive local-engine metadata:
`localRelationEngine`, `duckdbRegistrationStrategy`, and
`registeredInputRowCount` are filled for the bounded local relation-engine
helper, while the shared SQL service continues to omit those fields by
default. This keeps the shared SQL surface stable while making the bounded
DuckDB pilot's execution choice visible.

The next bounded runtime-stats slice is landed too. The same bounded markdown
payload now also exposes `registeredInputBatchCount`, `registrationTimeMs`,
and `localQueryExecutionTimeMs` for the bounded local relation-engine helper.
This still keeps the shared SQL surface stable by default while making the
bounded pilot's registration and execution cost visible to explain consumers.

The next bounded byte-metadata slice is landed too. The same bounded markdown
payload now also exposes `registeredInputBytes` and `resultBytes` for the
bounded local relation-engine helper. This keeps the shared SQL surface stable
by default while making the bounded pilot's input and output memory footprint
visible to explain consumers.

The next bounded materialization-state slice is landed too. The same bounded
markdown payload now also exposes `localRelationMaterializationState`, so the
bounded local relation-engine helper reports whether it materialized the
relation or kept it virtual. This keeps the shared SQL surface stable by
default while making the bounded pilot's materialization behavior explicit
across both DataFusion and DuckDB paths.

The canonical DuckDB RFC is now synchronized with the same bounded rollout
status too. It records the landed RFC and boundary slice, the local
relation-engine seam, the bounded markdown pilot, the request-scoped
registration policy, and the additive runtime-stats, byte-metadata, and
materialization-state slices as code-backed Wendao work. Qianji stage-local
DuckDB pilots remain future work.

The next bounded temp-storage slice is landed too. The same bounded markdown
payload now also exposes `localTempStoragePeakBytes`, fed from DuckDB profiling
metric `SYSTEM_PEAK_TEMP_DIR_SIZE` on the bounded local engine path. This keeps
the shared SQL surface stable by default while making bounded DuckDB temp
storage usage visible without introducing a broader profiling system.

The next gateway-facing slice is landed too. Repo-backed gateway reads for
`repo_entity` and `repo_content_chunk` now route published Parquet scans
through a bounded `ParquetQueryEngine` seam under `src/duckdb/parquet.rs`.
When `search.duckdb.enabled` is true in a `duckdb` build, those repo-backed
gateway reads execute through DuckDB; otherwise they fall back to the current
DataFusion engine. This is the first gateway read cutover under the RFC, while
non-repo gateway handlers and local-corpus Lance writer removal remain future
work.

The next local-corpus gateway slice is landed too. The published `local_symbol`
read lane now reuses the same bounded `ParquetQueryEngine`, so local-symbol
search, autocomplete, and payload hydration no longer read directly from
`SearchEngineContext`. When `search.duckdb.enabled` is true in a `duckdb`
build, those published `local_symbol` parquet reads execute through DuckDB;
otherwise they fall back to DataFusion.

The next symbol-route gateway slice is landed too. `/search/symbols` now
reuses the published `local_symbol` read lane instead of reading from the
in-memory `UnifiedSymbolIndex`. The handler keeps the existing response
contract and pending/indexing behavior, filters the broader `local_symbol`
workset back down to code-symbol results, and focused handler plus Flight
provider tests now prove the route can return DuckDB-fed symbol hits without
warming the old in-memory symbol index.

The next local-symbol ownership slice is landed too. The `local_symbol` build
owner now rewrites published partition tables directly to Parquet through a
bounded local-publication helper instead of cloning and mutating Lance tables.
Local epoch discovery is now Parquet-only, and `local_symbol` no longer
participates in local Lance compaction scheduling because it no longer owns a
local Lance publication store. Focused build, query, and gateway tests now
prove the same published read contract without leaving behind fresh
`local_symbol` `.lance` tables.

The next local-corpus gateway slice is landed too. The published
`reference_occurrence` read lane behind `/search/references` now reuses the
same bounded `ParquetQueryEngine`, so the stage-one scan and payload
hydration path no longer reads directly from `SearchEngineContext`. The SQL
builder for this lane now quotes engine-facing identifiers such as `column`,
which keeps the published parquet read path valid in both DataFusion and
DuckDB. When `search.duckdb.enabled` is true in a `duckdb` build, those
published `reference_occurrence` parquet reads execute through DuckDB;
otherwise they fall back to DataFusion.

The next reference-occurrence ownership slice is landed too. The
`reference_occurrence` build owner now rewrites its published table directly
to Parquet through the bounded local-publication helper instead of cloning and
mutating a Lance table. The same published read contract stays in place, and
the corpus no longer participates in local Lance compaction scheduling because
it no longer owns a local Lance publication store.

The next local-corpus gateway slice is landed too. The published `attachment`
read lane behind `/search/attachments` now reuses the same bounded
`ParquetQueryEngine`, so the stage-one scan and payload hydration path no
longer reads directly from `SearchEngineContext`. The SQL builder for this
lane now quotes engine-facing identifiers and table names as well, keeping the
same published parquet read path valid in both DataFusion and DuckDB. When
`search.duckdb.enabled` is true in a `duckdb` build, those published
`attachment` parquet reads execute through DuckDB; otherwise they fall back to
DataFusion.

The next attachment ownership slice is landed too. The `attachment` build
owner now rewrites its published table directly to Parquet through the bounded
local-publication helper instead of cloning and mutating a Lance table, so the
same published read contract stays in place without a local Lance publication
store. The corpus also no longer participates in local Lance compaction
scheduling, and focused build plus query tests now prove the writer cut leaves
no fresh `attachment` `.lance` tables behind.

The next local-corpus gateway slice is landed too. The published
`knowledge_section` read lane behind the gateway knowledge search path now
reuses the same bounded `ParquetQueryEngine`, so the stage-one scan and
payload hydration path no longer read directly from `SearchEngineContext`.
The SQL builder for this lane now quotes engine-facing identifiers and table
names as well, keeping the same published parquet read path valid in both
DataFusion and DuckDB. When `search.duckdb.enabled` is true in a `duckdb`
build, those published `knowledge_section` parquet reads execute through
DuckDB; otherwise they fall back to DataFusion. Knowledge intent/source merge
orchestration remains a separate future migration question.

The next knowledge ownership slice is landed too. The `knowledge_section`
build owner now rewrites its published table directly to Parquet through the
bounded local-publication helper instead of cloning and mutating a Lance
table, so the same published read contract stays in place without a local
Lance publication store. The corpus also no longer participates in local
Lance compaction scheduling, and focused build plus query tests now prove the
writer cut leaves no fresh `knowledge_section` `.lance` tables behind.

The next gateway aggregation proof is landed too. `/search/intent` still does
not own a separate parquet read engine. Instead, it composes the already
migrated `knowledge_section`, `local_symbol`, and repo-intent lanes. The
bounded internal transport metadata for this route now records query-engine
labels for those source lanes, and focused handler plus Flight tests prove
that the public route can return DuckDB-fed intent hits without changing the
response contract or merge semantics.

The next bounded FlightSQL protocol slice is landed too. `CommandStatementQuery`
no longer unconditionally routes into the shared request-scoped SQL surface.
Single-table statements against published local `reference_occurrence`,
`attachment`, and `knowledge_section` corpora now reuse the bounded
`ParquetQueryEngine`, which means the same statement surface selects DuckDB
when `search.duckdb.enabled` is true and falls back to DataFusion otherwise.
All other FlightSQL statements still use the shared SQL path, and the routed
statement batches normalize top-level string columns back to the existing
`Utf8View` Arrow shape so the public FlightSQL payload contract stays stable.

The next bounded FlightSQL protocol slice is landed too. The same statement
router now also recognizes concrete repo publication source tables that are
already exposed by catalog discovery, such as hashed
`repo_content_chunk_repo_<hash>` names, and resolves them back to active
publication Parquet through repo snapshot metadata. Logical repo views still
stay on the shared SQL fallback, so FlightSQL still does not plan multi-source
repo unions, while `CommandGetTables` and `CommandStatementQuery` now agree on
the concrete repo-table surface that is eligible for DuckDB/DataFusion routing.

The next bounded FlightSQL protocol slice is landed too. The same statement
router now also recognizes concrete published `local_symbol` source tables,
including partitioned active-epoch names, and resolves them back to the active
local parquet files through the existing epoch table-name helpers. The logical
`local_symbol` view still stays on the shared SQL path, so this slice does not
teach FlightSQL to plan local-symbol views; it only makes `CommandStatementQuery`
agree with `CommandGetTables` on the already-exposed `local_symbol` source-table
family.

The next bounded diagnostics slice is landed too. The Studio search-index
status route now computes its top-level total, phase counts,
`compactionPending`, and aggregate maintenance summary through a bounded local
relation-engine helper instead of pure ad-hoc Rust traversal. The public
status payload stays unchanged, the route falls back to the existing Rust
summary path if local diagnostics execution fails, and focused unit plus
route-level tests now prove the same payload under both fallback and
DuckDB-enabled runtime policy.

The next diagnostics-expansion slice is landed too. The same bounded
search-index diagnostics helper now also rolls up
`query_telemetry_summary`, including per-scope telemetry buckets, through the
local relation-engine seam instead of the old ad-hoc Rust accumulator. The
public payload and fallback path remain unchanged, and focused telemetry-heavy
unit fixtures plus the existing route proof still match the Rust baseline.

The next diagnostics-expansion slice is landed too. The same bounded
search-index diagnostics helper now also selects the aggregate
`status_reason` through a request-scoped relation instead of leaving that
top-level priority rollup on ad-hoc Rust traversal. Severity and code
priority, plus affected, readable, and blocking corpus counts, remain
contract-identical to the Rust baseline, and the diagnostics path still falls
back cleanly if local execution fails.

The next diagnostics-expansion slice is landed too. The same bounded
search-index diagnostics helper now also maps top-level `repo_read_pressure`
through a request-scoped relation instead of leaving that field on direct Rust
snapshot mapping. The public payload and fallback path remain unchanged, and
all optional repo-read pressure fields continue to match the Rust baseline.

The next `appender-arrow` utilization slice is landed too. The same bounded
search-index diagnostics helper now marks `query_telemetry_rows` as a
repeated-use relation registration, which lets DuckDB prefer
`MaterializedAppender` even when the default row-count threshold would have
kept that relation virtual. DataFusion keeps its current in-memory
registration behavior, the public diagnostics payload stays unchanged, and
focused engine plus diagnostics tests now prove that the hint only narrows
request-scoped execution policy.

The next repo/runtime diagnostics slice is landed too. The Studio repo-index
analysis Flight route now rolls up its phase summary counts from the
per-repository `repos` relation through the same bounded local relation-engine
seam instead of trusting only the pre-aggregated counters on the response
struct. The JSON repo-index contract stays unchanged, the Flight batch and
metadata stay contract-identical, and the SQL rollup now casts all aggregate
columns to `BIGINT` so DataFusion and DuckDB agree on one stable `Int64`
Arrow shape instead of drifting by engine.

The next repo/runtime diagnostics follow-up slice is landed too. The same
repo-index analysis Flight diagnostics relation now also carries explicit
`active_order`, so `active_repo_ids` and `current_repo_id` are recomputed from
request-scoped rows instead of being copied directly from the incoming
response. The boundary stays narrow: runtime ordering is preserved, the JSON
and Flight contracts do not widen, and the same repeated-use registration hint
now justifies one bounded two-query workset over the same relation.

The next repo/runtime diagnostics HTTP follow-up slice is landed too. The
Studio `repo_index_status` JSON route now reuses the same bounded diagnostics
helper as the repo-index Flight route before serialization, so stale aggregate
counts plus active identity fields are recomputed consistently across both
surfaces while the JSON envelope, bootstrap telemetry, and fallback semantics
remain unchanged.

The next local-publication boundary slice is landed too. Local epoch discovery
for search-plane corpora now ignores legacy `.lance` artifacts and only
observes Parquet publications, while local prewarm now rejects missing Parquet
epochs instead of falling back to opening a local store. Focused construction
and maintenance proofs now keep stale local `.lance` directories from holding
search-plane read ownership open.

The next local-maintenance retirement slice is landed too. Wendao no longer
ships a local compaction queue or worker runtime for search-plane corpora:
`publish_ready_and_maintain(...)` now performs a pure publish for local
corpora, local maintenance runtime state is shutdown-only, and runtime status
annotation no longer fabricates local compaction backlog or running views.
Focused coordinator, maintenance, and status proofs now keep local compaction
metadata idle while preserving the repo-backed compaction status path.

The next bounded performance-gate slice is landed too. The Wendao performance
suite now compares the DataFusion and DuckDB `ParquetQueryEngine` lanes over
the same deterministic synthetic Parquet fixture, emits durable perf reports
through the shared `xiuxian-testing` harness, and enforces a configurable
DuckDB/DataFusion p95 ratio budget at the execution seam itself. This keeps
performance evidence attached to the bounded query-engine surface without
widening protocol or storage ownership.

The next bounded FlightSQL performance-gate slice is landed too. Wendao now
also benchmarks the routed single-table `CommandStatementQuery` surface over a
Julia parser-summary-aware gateway perf fixture, so the same published
repo-content source-table statement executes through both DataFusion and
DuckDB under the shared `xiuxian-testing` harness. The gate stays narrow: it
measures only the already-routed FlightSQL statement seam, emits durable perf
reports, and enforces a configurable DuckDB/DataFusion p95 ratio budget
without widening FlightSQL planning or storage ownership.

The next bounded FlightSQL latency-breakdown slice is landed too. The same
routed statement benchmark now also persists per-phase timing metadata into
its durable reports, including a direct-engine lower bound and bounded timings
for `get_flight_info`, `do_get` collection, decode, and validation. Current
local evidence shows that routed statement latency is dominated by
`get_flight_info` planning overhead rather than by DuckDB execution itself:
the direct-engine lower bound stays far below the routed statement p95, while
`do_get` collection and decode remain negligible on the bounded workload.

The next bounded live-harness correction slice is landed too. The routed
FlightSQL performance gate now leaves the required `gRPCServer` runtime
dependency under `WendaoSearch.jl`'s own `scripts/run_search_service.jl`
bootstrap instead of routing that ownership through Rust-side preflight or a
`WendaoArrow` support helper. The live script still honors an explicit
`WENDAO_FLIGHT_GRPCSERVER_PATH` override and reuses a vendored checkout when
present, but otherwise now bootstraps `gRPCServer.jl` from its official
`develop` branch into the live Julia environment before binding the Flight
listener.

## Native Flight

Native Flight should continue to own Wendao-specific capabilities that are not
well modeled as plain relational queries, including:

- semantic search routes
- definition and reference resolution
- graph-neighbor and topology routes
- VFS content and navigation
- analysis routes

These routes may internally depend on search-plane corpora, but they are still
business capabilities, not just query-language translation.

## Shared Queries System

The shared `queries/` system is the family boundary that should compile every
query language down to the same DataFusion execution core:

- SQL
- FlightSQL
- GraphQL
- REST-style query APIs
- CLI `query`

The contract for the shared system is:

1. validate the request language payload
2. open one request-scoped query core over the visible Wendao search-plane data
3. translate request shape into a DataFusion-readable query or plan
4. execute against that request-scoped query core
5. return Arrow-native batches plus adapter-specific metadata or rendering

## First Physical Slice

The first physical modularization slice is to stop treating SQL as a business
handler under `handlers/sql/` and instead make it explicit as a shared query
adapter under `queries/sql/`.

That first slice is intentionally bounded:

- it does not add FlightSQL execution yet
- it does not add GraphQL execution yet
- it does not add the CLI `query` command yet
- it does not change native Flight route behavior
- it only makes the architecture explicit in code layout

That first slice is now complete. The next bounded slice is also landed:
`wendao query sql --query ...` now reuses the same `queries/sql/` execution
seam instead of creating a second planner path under `src/bin/wendao/`.

## Current Shared SQL Boundary

`queries/sql/` now has two responsibilities:

1. adapter-specific request decoding or response metadata for SQL-over-Flight
2. a transport-neutral request-scoped SQL execution path that other adapters
   can reuse

The shared execution rule is now enforced in code:

- Flight provider code may wrap SQL execution results
- CLI `query` code may render SQL execution results
- neither adapter may own a private copy of the request-scoped DataFusion
  assembly or execution flow

## Planned Namespace Shape

The intended long-term search tree is:

- `src/search/mod.rs`: canonical shared-search namespace
- `search/handlers/flight/`: native Flight business capabilities
- `search/queries/mod.rs`: shared query system seam
- `search/queries/sql/`: SQL adapter
- `search/queries/flightsql/`: FlightSQL adapter
- `search/queries/graphql/`: GraphQL adapter
- `search/queries/rest/`: REST-style query adapter when needed
- `src/bin/wendao/.../query.rs`: CLI adapter into the same query system

The currently landed shared-query adapters are:

- `search/queries/sql/`: request-scoped SQL execution plus Flight wrapping
- `search/queries/flightsql/`: FlightSQL statement-query plus `sql_info`
  adapter over the same request-scoped SQL surface
- `search/queries/graphql/`: GraphQL table-query adapter over the same
  request-scoped SQL surface
- `search/queries/rest/`: REST-style request/response adapter over the same
  shared query service
- `src/bin/wendao/execute/query/`: CLI adapters over the same shared query
  system

The current ownership rule is explicit:

- `src/search/queries/` is the canonical implementation tree
- adapter-local tests should live with the canonical adapter under
  `src/search/queries/*/tests/` unless a gateway-facing namespace is itself
  the behavior under test
- adapter-local SQL, GraphQL, and FlightSQL tests now follow that rule under
  `src/search/queries/{sql,graphql,flightsql}/tests/`
- the old `gateway/studio/search/queries/` tree is retired entirely; native
  Flight and CLI callers import the canonical adapters directly

The first landed FlightSQL cut is intentionally narrow:

- one dedicated FlightSQL server builder and binary
- `CommandStatementQuery` now first checks a bounded local published-Parquet
  route for single-table `reference_occurrence`, `attachment`, and
  `knowledge_section` statements, and otherwise falls back to the shared
  request-scoped SQL surface
- minimal `CommandGetSqlInfo` coverage
- no prepared statements, ingest/update, or broad JDBC/XDBC metadata yet
- no merger with the native Wendao business Flight router

The next bounded FlightSQL discovery slice should add:

- `CommandGetCatalogs`
- `CommandGetDbSchemas`
- `CommandGetTables`
- one stable logical catalog over the shared request-scoped SQL surface
- schema names derived from registered SQL scope instead of a second planner
  layer or sidecar registry

The first landed GraphQL cut is intentionally narrow:

- one table-query frontend over the request-scoped SQL surface
- ROAPI-style query operators such as `filter`, `sort`, `limit`, and `page`
- no full HTTP GraphQL server yet
- no custom GraphQL business root fields
- no attempt to flatten all Wendao business semantics into one GraphQL release

Today the same adapter is reachable through:

- native shared-query internals under `search/queries/graphql/`
- `wendao query graphql --document ...` on the CLI

Within `search/queries/`, adapter-neutral execution now lives above the
protocol-specific wrapper modules so CLI, FlightSQL, GraphQL, and REST
adapters can all reuse the same request-scoped query surface.

That reuse is now physical in code too: `search/queries/core/` assembles the
shared request-scoped query core once, and SQL, GraphQL, and FlightSQL consume
that seam instead of calling the low-level surface-registration function
directly.

The next tightening above that core is landed too: canonical adapters and CLI
entrypoints now share one `SearchQueryService` seam over the landed query core
instead of each holding raw `SearchPlaneService` ownership independently.

The naming-convergence cleanup is now landed too. Query-owned names under
`src/search/queries/` now use neutral shared-query naming instead of reading
like legacy Studio gateway wrappers. That cleanup does not apply to
explicitly Studio-owned gateway transport/provider names.

The first landed REST cut is intentionally narrow:

- one thin request/response adapter under `search/queries/rest/`
- request variants limited to SQL and GraphQL delegation
- one CLI proof through `wendao query rest --payload ...`
- no native HTTP route rollout yet
- no REST-owned planner or request-scoped surface assembly

Snapshot-level regression coverage is now mandatory for every canonical
adapter under `src/search/queries/*`. SQL, GraphQL, FlightSQL, and REST all
keep adapter-local snapshot suites under their canonical `tests/` folders, and
the source-tree enforcement test under `search/queries/tests/` keeps that
contract from drifting back into a convention.

These baselines now live under a canonical `tests/snapshots/search/queries/`
tree rather than the legacy `gateway/studio` snapshot namespace, because
gateway and CLI consume one canonical query system under `src/search/queries/`.

The next ownership tightening for tests is landed too: repeated shared-query
fixture support now lives under `search/queries/tests/`, while
transport-specific decode helpers stay inside the adapter-local test folders.

The next bounded gateway-downpressure slice is now landed too:
repo-content business-search execution now lives under
`src/search/repo_search/`, and both native Flight repo-search and code-search
consume that shared seam. The Flight provider remains a transport adapter, but
it is no longer the canonical execution owner for repo-content business search.

The next bounded slice in the same lane is now landed too: repo-entity
execution and relation-to-`SearchHit` shaping now live under
`src/search/repo_search/`. Code-search delegates to that shared seam
directly, while the knowledge-intent path reuses the same execution owner
through the thin gateway wrapper instead of keeping a gateway-only repo-entity
execution core alive.

The next bounded slice in the same lane is landed too: shared repo-search
target partitioning and parallelism selection now live under
`src/search/repo_search/dispatch.rs`, so gateway `code_search/query.rs` no
longer owns dispatch planning for the same repo-search workstream.

The next bounded slice in the same lane is now landed too: buffered
repo-search queue-draining, spawn policy, and repo-level query execution now
live under `src/search/repo_search/`. Knowledge-intent merge and code-search
callers consume the same shared buffered seam, and the old
gateway-local `code_search/search/{buffered,task}.rs` owners are retired.

The next bounded slice in the same lane is now landed too: shared
repo-search publication-state lookup, dispatch telemetry, and
pending/skipped/partial state assembly now live under
`src/search/repo_search/orchestration.rs`. Knowledge-intent merge and
code-search response callers now adapt that same shared owner instead of
recomputing dispatch state locally.

The remaining gateway-local boundary in this lane is now explicit:
Studio-config repo resolution, cache policy, and final response DTO shaping
still remain in the Studio gateway layer by design. Any later move past this
point would be a DTO-boundary review, not another pure execution downpressure
slice.

The production `intent = "code_search"` path now also uses that source-owned
shape end to end. `src/gateway/studio/search/handlers/code_search/search/`
is the real gateway owner, the old test-mounted implementation files are
retired, and the linked host proofs now execute Studio code-search against
plain Julia and plain Modelica plugin repositories backed by the Julia-owned
native parser-summary routes.

Within `search/queries/graphql/`, document parsing should stay adapter-local,
while execution should delegate into the existing shared SQL/DataFusion
surface:

- table and view lookup through request-scoped SQL registration
- DataFusion dataframe operators compiled from the GraphQL query shape
- no direct graph-native business traversal unless the graph data is first
  materialized as SQL-visible tables or views

Future query adapters should follow the same feature-folder rule:

- one folder per adapter
- one interface seam in `mod.rs`
- request parsing, translation, metadata, and tests split by responsibility
- shared query semantics should stay above adapter-specific request parsing

## Contributor Rules

- Do not place new query-language translation logic inside business handlers.
- Do not widen the native Flight gateway with query-adapter planning logic.
- Keep shared query semantics in `xiuxian-wendao`, because DataFusion query
  semantics belong there rather than in `runtime`.
- Keep request-scoped surface assembly behind one shared query-core seam rather
  than letting SQL, GraphQL, and FlightSQL each call the low-level assembly
  helper directly.
- Keep the canonical adapter implementation under `src/search/queries/`.
- Do not reintroduce a `gateway/studio/search/queries/` shadow implementation
  tree.
- Keep GraphQL and FlightSQL adapter tests in the canonical
  `src/search/queries/*/tests/` tree.
- Keep SQL adapter tests under `src/search/queries/sql/tests/`; do not
  reintroduce a gateway-owned SQL test tree.
- Keep snapshot-level regression coverage mandatory for canonical adapters
  under `src/search/queries/*`.
- Make CLI, backend, and future frontend adapters consume the same shared
  query system rather than each owning their own planner path.
- Do not make the CLI depend on Flight-specific provider traits when a shared
  query execution seam is the correct owner.
- Keep execution request-scoped so visible corpora and catalogs reflect the
  current request, not a shared global SQL session.
