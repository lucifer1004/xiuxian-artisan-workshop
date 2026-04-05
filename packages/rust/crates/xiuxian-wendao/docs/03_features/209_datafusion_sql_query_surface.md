# DataFusion SQL Query Surface

:PROPERTIES:
:ID: feat-datafusion-sql-query-surface
:PARENT: [[index]]
:TAGS: feature, sql, datafusion, flight, snapshot
:STATUS: ACTIVE
:VERSION: 1.0
:END:

## Overview

`xiuxian-wendao` exposes a request-scoped SQL surface on top of the
search plane. Each SQL request builds a fresh DataFusion session, registers the
currently readable search-plane corpora, then executes a read-only SQL query
through the shared query core and SQL execution seam used by the SQL Flight
provider, FlightSQL, GraphQL, and the CLI `query` adapters through one shared
`SearchQueryService` ownership seam.

The surface is intentionally request-scoped:

- queryable tables reflect only the corpora readable for that request
- discovery catalogs are rebuilt per request
- `information_schema` is enabled without mutating a shared global session

## SQL Surface Layers

The SQL lane lives under `src/search/queries/`:

- `core/`: shared request-scoped `SearchEngineContext + SqlQuerySurface`
  assembly
- `execution/`: transport-neutral SQL execution results and payload rendering
- `provider/`: SQL Flight entrypoint, route execution, and app metadata
- `registration/`: request-scoped DataFusion session setup, table/view
  registration, and stable SQL naming
- `tests/`: focused provider, catalog, information-schema, logical-view, and
  snapshot coverage

The core registration flow is:

1. open the shared request-scoped query core
2. create a request-scoped `SearchEngineContext`
3. register readable local and repo-backed corpora
4. register stable logical views
5. register Wendao discovery catalogs
6. execute the client SQL query
7. return Arrow batches plus structured SQL app metadata

## Stable SQL Objects

The SQL surface distinguishes between base tables, logical views, and
request-scoped system catalogs.

### Base Tables

- stable local aliases such as `reference_occurrence`
- stable repo aliases such as
  `SearchPlaneService::repo_content_chunk_table_name(repo_id)` and
  `SearchPlaneService::repo_entity_table_name(repo_id)`

### Logical Views

- `local_symbol`: unions currently readable local-symbol tables
- `repo_content_chunk`: unions readable repo-content aliases and injects
  `repo_id`
- `repo_entity`: unions readable repo-entity aliases and injects `repo_id`

### System Catalogs

- `wendao_sql_tables`: queryable inventory of tables, views, and system
  catalogs
- `wendao_sql_columns`: column inventory with origin semantics
- `wendao_sql_view_sources`: logical-view membership and source ordering

### Standard SQL Discovery

The same request-scoped session enables DataFusion `information_schema`, so
clients can query:

- `information_schema.tables`
- `information_schema.columns`

The returned app metadata advertises this through
`supportsInformationSchema = true`.

## Catalog Semantics

`wendao_sql_tables` exposes:

- `sql_table_name`
- `corpus`
- `scope`
- `sql_object_kind`
- `source_count`
- `repo_id`

`wendao_sql_columns` exposes:

- `sql_table_name`
- `column_name`
- `source_column_name`
- `data_type`
- `sql_object_kind`
- `column_origin_kind`

`column_origin_kind` uses:

- `stored`: base-table column persisted in the underlying corpus
- `projected`: logical-view column projected from a source column
- `synthetic`: logical-view column injected by the SQL surface, such as
  `repo_id`, `title`, `doc_type`, `code_tag`, `file_tag`, `kind_tag`, and
  `language_tag` on `repo_content_chunk`

`wendao_sql_view_sources` exposes:

- `sql_view_name`
- `source_sql_table_name`
- `corpus`
- `repo_id`
- `source_ordinal`

## Query Expressions

The SQL feature is only usable if clients can write concrete expressions
without reverse-engineering the provider. The following queries are the
intended starting surface for both human users and LLM-generated SQL.

### Discovery Queries

List all SQL-visible objects for the current request:

```sql
SELECT sql_table_name, corpus, scope, sql_object_kind, source_count, repo_id
FROM wendao_sql_tables
ORDER BY sql_table_name, COALESCE(repo_id, '');
```

Inspect the columns exposed by one SQL object:

```sql
SELECT column_name, source_column_name, data_type, sql_object_kind, column_origin_kind
FROM wendao_sql_columns
WHERE sql_table_name = 'repo_entity'
ORDER BY ordinal_position;
```

Inspect the physical sources behind one logical view:

```sql
SELECT sql_view_name, source_sql_table_name, corpus, repo_id, source_ordinal
FROM wendao_sql_view_sources
WHERE sql_view_name = 'repo_content_chunk'
ORDER BY source_ordinal, COALESCE(repo_id, '');
```

Use standard SQL discovery through DataFusion:

```sql
SELECT table_name, table_type
FROM information_schema.tables
WHERE table_name IN (
  'reference_occurrence',
  'local_symbol',
  'repo_content_chunk',
  'repo_entity'
)
ORDER BY table_name;
```

Run the same request through the CLI adapter:

```bash
direnv exec . cargo run -p xiuxian-wendao --bin wendao -- query sql --query \
  "SELECT sql_table_name, sql_object_kind FROM wendao_sql_tables ORDER BY sql_table_name"
```

### Local Corpus Queries

Reference-occurrence lookup:

```sql
SELECT name, path, line
FROM reference_occurrence
WHERE name = 'AlphaService'
ORDER BY path, line;
```

Local-symbol lookup through the stable logical view:

```sql
SELECT name, path, line_start
FROM local_symbol
WHERE name = 'AlphaSymbol'
ORDER BY path, line_start;
```

### Repo Logical-View Queries

Cross-repo content query:

```sql
SELECT repo_id, title, path, doc_type, language_tag, kind_tag, line_number, line_text
FROM repo_content_chunk
WHERE path = 'src/lib.rs'
ORDER BY repo_id, line_number;
```

Repo-content logical-column query:

```sql
SELECT repo_id, title, path, language_tag, code_tag, file_tag, kind_tag
FROM repo_content_chunk
WHERE title LIKE '%parser%'
  AND language_tag = 'lang:rust'
ORDER BY repo_id, path;
```

Cross-repo entity query:

```sql
SELECT repo_id, entity_kind, name, path
FROM repo_entity
WHERE entity_kind = 'symbol'
ORDER BY repo_id, name;
```

### Query-Writing Rules

- Treat the SQL surface as read-only. The provider validates read-only query
  text and is not intended for DDL or mutation statements.
- Prefer stable SQL names such as `reference_occurrence`, `local_symbol`,
  `repo_content_chunk`, and `repo_entity` instead of internal engine table
  names.
- Use the Wendao catalogs first when you need stable contract semantics such as
  `column_origin_kind` or logical-view source membership.
- Use `information_schema` when you need portable SQL metadata queries.
- Expect the surface to be request-scoped. Visible tables and views depend on
  which corpora are currently readable for that request.

### LLM Prompting Guidance

When an LLM needs to author SQL for this surface, the prompt should include at
least:

- the target SQL object name
- the expected output columns
- whether the query should stay inside one corpus or span multiple repos
- whether the model should consult `wendao_sql_tables`,
  `wendao_sql_columns`, or `wendao_sql_view_sources` first

For example:

```text
Write a read-only SQL query for Wendao's request-scoped DataFusion surface.
Use the stable logical view `repo_entity`. Return `repo_id`, `name`, and
`path` for symbol rows only, ordered by `repo_id` and `name`.
```

## Gateway Convergence Targets

The SQL surface is now mature enough to absorb some gateway query lanes, but
not all of them should be folded into SQL.

Good convergence targets:

- repo-content Flight filtering that already behaves like table selection and
  projection
- request-scoped discovery and introspection routes
- local or repo-backed search-plane corpora that already expose stable SQL
  names and do not require extra semantic enrichment

Out of scope for direct SQL convergence:

- graph-native traversal and topology queries that are better modeled through
  `query_core` and future GraphQL adapters than through one flat relational
  surface

- definition and intent routes with semantic post-processing
- knowledge and graph routes that are not simple table projections
- symbol-index routes that still depend on symbol-index-only behavior rather
  than search-plane SQL-visible corpora

The practical rule is simple: if the route is mostly query planning, row
filtering, projection, and ordering over SQL-visible search-plane tables, it is
a good SQL convergence candidate. If the route adds thick semantic logic, it
should stay outside the SQL lane.

### Repo-Content Flight Status

The first three repo-content gateway-thinning slices are now live.

- `path_prefixes` and `filename_filters` are query-native: they are planned
  into the repo-content SQL/DataFusion scan before the Flight batch is
  materialized.
- `language_filters` were already query-native and remain in the same SQL
  planning lane.
- `title_filters` are now query-native too: they are planned into the
  repo-content SQL/DataFusion scan through folded path/title semantics.
- `tag_filters` no longer stay in the Flight gateway adapter. They execute in
  the repo-content query lane after hit materialization, which keeps the
  gateway at protocol-adapter scope while preserving the current tag
  semantics.
- The stable `repo_content_chunk` logical view now exposes SQL-facing derived
  columns for repo-content semantics:
  `title`, `doc_type`, `code_tag`, `file_tag`, `kind_tag`, and
  `language_tag`.

For prompt and client writing, this means repo-content path, filename,
language, and title constraints should be treated as SQL-visible table filters.
Tag constraints still stay in the repo-content query lane because some tag
semantics, especially exact-match tags, remain query-dependent rather than
stable logical-view columns.

The next bounded convergence step is to split repo-content tag semantics into:

- SQL-safe stable tags such as language or fixed file-kind tags
- query-dependent tags such as exact-match markers

Only the SQL-safe subset should be planned into DataFusion SQL. The
query-dependent subset should stay in the query lane.

## Snapshot Testing Contract

The SQL lane keeps snapshot-level regression coverage in
`src/search/queries/sql/tests/snapshots.rs`, with baselines
written under `tests/snapshots/search/queries/`.

The snapshot suite locks two contracts:

- query surface snapshots for stable local and logical-view SQL queries
- discovery surface snapshots for Wendao catalogs and `information_schema`

The snapshot payload is normalized into JSON:

- decoded SQL app metadata
- batch schema metadata
- row-oriented batch values

This avoids brittle Arrow-internal formatting while still pinning the SQL
contract seen by clients.

## Validation Commands

- `direnv exec . cargo nextest run -p xiuxian-wendao --lib --features julia gateway::studio::search::queries::sql::tests`
- `direnv exec . cargo clippy -p xiuxian-wendao --all-targets --all-features --message-format=short -- -D warnings`
- `direnv exec . git diff --check`

## Contributor Notes

When adding a new SQL-visible corpus or logical view:

1. register it through `registration/`
2. extend the Wendao discovery catalogs if the SQL-visible contract changes
3. update the SQL snapshot suite if the client-facing surface changes
4. sync this feature doc, the package README, GTD, and the active ExecPlan
