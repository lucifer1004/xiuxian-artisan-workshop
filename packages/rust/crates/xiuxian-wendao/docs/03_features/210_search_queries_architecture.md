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
language into a DataFusion plan over the Wendao search plane. SQL is the first
fully landed query adapter. FlightSQL, GraphQL, REST-style query APIs, and a
CLI `query` subcommand belong to the same architectural family even when they
are introduced later.

## Target Layering

The intended layering is:

- native Flight business routes: Wendao capability surfaces
- shared queries system: one semantic query boundary inside `xiuxian-wendao`
- query adapters: SQL, FlightSQL, GraphQL, REST, CLI
- DataFusion planning and execution: one execution core inside
  `xiuxian-wendao`
- Arrow result encoding: returned through Flight or another adapter surface

This means Flight is not a replacement for DataFusion. Flight is a transport
or business-protocol surface. DataFusion remains the query planning and
execution engine for the shared queries system.

The bounded DuckDB analytics proposal is tracked separately in
[RFC: DuckDB as a Bounded In-Process Analytic Lane for Wendao and Qianji](../../../../../../docs/rfcs/2026-04-08-wendao-qianji-duckdb-bounded-analytics-rfc.md).
That RFC keeps the external Flight contract unchanged and keeps the shared
query system DataFusion-led; DuckDB is explicitly scoped only to internal
request-scoped or bounded-lived analytic execution.

The first bounded implementation slice under that RFC is now landed too.
`xiuxian-wendao` has a local `src/duckdb/` bridge that exposes one bounded
local relation-engine seam, and the bounded-work markdown lane now uses that
seam while remaining DataFusion-backed. DuckDB-specific execution is still
feature-gated scaffolding rather than the default shared query core.

The next bounded pilot under the same RFC is landed too. The bounded-work
markdown query owner now exposes an explicit
`query_bounded_work_markdown_payload_with_engine(...)` helper, and the new
`DuckDbLocalRelationEngine` can register Arrow batches through
`appender-arrow` and return Arrow-native query batches. The default
`query_bounded_work_markdown_payload(...)` path still instantiates the
DataFusion engine explicitly, so the shared query system remains DataFusion-led
while the DuckDB pilot stays opt-in and request-scoped.

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

The next bounded diagnostics slice is landed too. The Studio search-index
status route now computes its top-level total, phase counts,
`compactionPending`, and aggregate maintenance summary through a bounded local
relation-engine helper instead of pure ad-hoc Rust traversal. The public
status payload stays unchanged, the route falls back to the existing Rust
summary path if local diagnostics execution fails, and focused unit plus
route-level tests now prove the same payload under both fallback and
DuckDB-enabled runtime policy.

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
- `CommandStatementQuery` routed into the shared request-scoped SQL surface
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
