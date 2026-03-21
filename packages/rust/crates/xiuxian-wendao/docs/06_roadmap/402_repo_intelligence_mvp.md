# Repo Intelligence MVP

:PROPERTIES:
:ID: wendao-repo-intelligence-mvp
:PARENT: [[index]]
:TAGS: roadmap, repo-intelligence, plugins, git
:STATUS: ACTIVE
:END:

## Goal

Land a Wendao-native Repo Intelligence MVP that lets agents answer repository questions from pre-indexed structure instead of repeating `grep`, `ls`, and ad-hoc exploration on every request.

## Scope

The MVP surface is limited to five query families:

- `repo.overview`
- `module.search`
- `symbol.search`
- `example.search`
- `doc.coverage`

The common core owns repository mirroring, incremental discovery, normalized record storage, graph persistence, and shared query contracts. Language-specific or ecosystem-specific semantics are delegated to Rust plugins selected in `wendao.toml`, for example `plugins = ["julia"]` or `plugins = ["modelica"]`.

## Repository Findings

### DifferentialEquations.jl

- Root shape is compact: `Project.toml`, `README.md`, `src/`, `test/`, and assets.
- The entry module is thin and primarily reexports upstream packages:
  - `SciMLBase`
  - `OrdinaryDiffEq`
- Effective intelligence for this repository depends on understanding package metadata, `@reexport` surfaces, and ecosystem links to external docs/tutorial packages.

### Modelica Standard Library

- Root shape is library-first: `Modelica/`, `ModelicaReference/`, `ModelicaServices/`, `ModelicaTest/`, plus top-level package files.
- `Modelica/package.mo` exposes rich structured metadata through `annotation(Documentation(...))`.
- `Examples` and `UsersGuide` subtrees are widespread and regular, making them strong candidates for first-class `ExampleRecord` and `DocRecord` extraction.

## Common-Core Boundary

The Wendao common core should absorb everything that is expensive, repeated, or storage-sensitive:

- git mirror management and refresh policies
- repository registry from `wendao.toml`
- incremental file discovery and invalidation
- file classification and normalized record ingestion
- graph persistence and shared retrieval APIs
- plugin registry, scheduling, and diagnostics

Plugins should only provide semantic enrichment, not take over the runtime.

## Plugin API Boundary

The first plugin API should stay narrow:

1. Detect whether the plugin applies to a repository or file set.
2. Analyze files into normalized records.
3. Enrich cross-file or cross-module relations after base ingestion.
4. Optionally expand or rerank query results at query time.

Plugins should return normalized records and relations, not mutate Wendao storage internals directly.

## Immediate Next Steps

1. Extend the explicit `wendao repo sync --repo <id>` control surface beyond the current `ensure`/`refresh`/`status` modes with richer sync policies and remote lifecycle diagnostics instead of keeping all source preparation implicit behind analysis queries.
2. Replace the current conservative Julia-only doc linker with richer repository-graph linking for docstrings and structured docs.
3. Deepen the external `xiuxian-wendao-modelica` implementation from conservative package-layout indexing toward richer MSL-aware semantics.
4. Consolidate fuzzy retrieval into shared Wendao search primitives so lexical edit-distance scoring, fuzzy option contracts, and Tantivy-backed fuzzy querying stop drifting across isolated search call sites.

## Current Status

- The Repo Intelligence implementation namespace now lives under `xiuxian-wendao::analyzers`; this roadmap keeps "Repo Intelligence" as the product concept, but code references should now point at `src/analyzers/` rather than the retired `src/repo_intelligence/` path.
- Initial contracts now exist for:
  - repository registration metadata
  - normalized records
  - MVP query request/response types
  - plugin trait boundaries
  - plugin registry behavior
- All five Repo Intelligence query slices are now wired end to end:
  - `wendao.toml` now derives repo-intelligence registrations from `link_graph.projects.<id>` instead of maintaining a parallel `[[repo_intelligence.repos]]` registry
  - legacy `[[repo_intelligence.repos]]` entries are now ignored by the runtime loader instead of being merged with project-derived registrations
  - project-scoped repo sources use `root = "..."` for local checkouts and `url = "..."` with optional `ref = "..."` for managed git materialization, while `plugins = ["julia" | "modelica"]` acts as the repo-intelligence opt-in on that same project entry
  - relative project roots resolve against the active `wendao.toml` directory
  - the common core now validates that configured local paths point at git checkout roots instead of arbitrary directories
  - repository records now derive `revision` and fallback `url` metadata from the local git checkout when configuration does not provide them
  - managed checkout refresh behavior is now explicit through `refresh = "fetch" | "manual"` instead of being hardcoded in the service layer
  - managed checkouts now clone from cache-local mirrors instead of cloning directly from upstream URLs every time
  - `repo.overview` now again merges plugin-provided repository metadata, post-analysis relation enrichment, checkout metadata hydration, and skeptic verification-state backfill before snapshotting or returning analysis results
  - `wendao repo sync --repo <id>` now exposes the common-core source lifecycle directly and returns the resolved source kind, requested sync mode, refresh policy, mirror/check-out lifecycle states, observation time (`checked_at`), last local mirror fetch time (`last_fetched_at`), mirror revision, tracking revision, drift state, high-level `health_state`, freshness-oriented `staleness_state`, a grouped `status_summary`, checkout path, optional mirror path, upstream URL, and active revision without requiring a full analysis pass
  - `wendao repo sync --repo <id> --mode status` now inspects the current managed-source cache state without creating mirrors, creating working checkouts, or triggering network refresh
  - managed source `status` mode is now again read-only for existing checkouts, while `ensure`/`refresh` correctly advance bare-mirror branch heads before materializing or fast-forwarding managed working copies
  - `repo sync` now also exposes a compact health summary so callers can distinguish `healthy`, `missing_assets`, `needs_refresh`, `has_local_commits`, `diverged`, and `unknown` without reinterpreting the lower-level lifecycle fields themselves
  - `repo sync` now also classifies mirror freshness into `fresh`, `aging`, and `stale` buckets, with `not_applicable` for local checkouts and `unknown` when managed metadata is missing
  - `repo sync` now also groups lifecycle, freshness, and revision state into a nested `status_summary` so agent-side consumers can read one structured object instead of reconstructing those relationships from flat fields
  - the same `repo sync` payload is now exposed through the studio gateway at `GET /api/repo/sync?repo=<id>&mode=<ensure|refresh|status>`, and the bundled OpenAPI artifact now documents that route for downstream consumers
  - `repo overview` is now also exposed through the studio gateway at `GET /api/repo/overview?repo=<id>`, so external agent callers can consume the normalized overview counts without shelling out to the CLI
  - `repo module-search` is now also exposed through the studio gateway at `GET /api/repo/module-search?repo=<id>&query=<text>&limit=<n>`, returning normalized module rows from the existing Repo Intelligence service path
  - `repo symbol-search` is now also exposed through the studio gateway at `GET /api/repo/symbol-search?repo=<id>&query=<text>&limit=<n>`, returning normalized symbol rows from the existing Repo Intelligence service path
  - `repo example-search` is now also exposed through the studio gateway at `GET /api/repo/example-search?repo=<id>&query=<text>&limit=<n>`, returning normalized example rows from the existing Repo Intelligence service path
  - `repo doc-coverage` is now also exposed through the studio gateway at `GET /api/repo/doc-coverage?repo=<id>&module=<qualified-name>`, returning normalized doc rows plus covered and uncovered symbol counts from the existing Repo Intelligence service path
  - the common core now also exposes registry-aware library entry points for `repo.overview`, `module.search`, `symbol.search`, `example.search`, and `doc.coverage`, so external crates can reuse the same configured query surface with custom plugin registries
  - `xiuxian-wendao` bootstraps the built-in `julia` plugin automatically for this slice
  - Julia syntax extraction now lives in `xiuxian-ast` behind its `julia` dependency feature, and the built-in Julia analyzer now registers through `xiuxian-wendao::analyzers::languages::julia` while the query/runtime orchestration lives under `xiuxian-wendao::analyzers::service`
  - the Julia AST layer now extracts conservative symbol docstrings and literal `include("...")` edges, and the Wendao Julia bridge now walks the root-file include graph before normalizing `DocRecord` inventory plus explicit `RelationKind::Documents` edges
  - the analyzer implementation is now split across `analyzers/` feature folders with interface-only `mod.rs` boundaries instead of keeping the old `repo_intelligence/` path as the live implementation root
  - `wendao repo overview --repo <id>` returns a real `RepoOverviewResult` through the existing `--output json|pretty` surface
  - `wendao repo module-search --repo <id> --query <text>` returns a real `ModuleSearchResult` through the same output surface
  - `wendao repo symbol-search --repo <id> --query <text>` returns a real `SymbolSearchResult` through the same output surface
  - `wendao repo example-search --repo <id> --query <text>` returns a real `ExampleSearchResult` through the same output surface and now uses explicit `RelationKind::ExampleOf` edges instead of relying only on example file names
  - `wendao repo doc-coverage --repo <id> [--module <module>]` now aggregates explicit `RelationKind::Documents` edges emitted during the Julia link phase instead of performing query-time path/title guessing
  - structural graph edges now exist for `Contains`, `Declares`, `Uses`, `Documents`, and `ExampleOf` in the Julia MVP slice
  - the first external extension validation slice is now landed as workspace crate `xiuxian-wendao-modelica`, which registers `plugins = ["modelica"]` and conservatively indexes `package.mo`, lightweight `.mo` declarations, `Examples`, `UsersGuide`, and inline `annotation(Documentation(...))` docs through the same common-core query surface
  - the external Modelica walker now skips hidden/VCS paths such as `.git`, so documentation inventory no longer picks up repository internals as false-positive docs
  - `xiuxian-wendao-modelica` is now realigned to the live `xiuxian-wendao::analyzers` record and import contracts again, so `cargo check -p xiuxian-wendao -p xiuxian-wendao-modelica` and `cargo test -p xiuxian-wendao-modelica` are both green instead of drifting behind stale common-core schemas
  - `cargo test -p xiuxian-wendao --lib` is now green again after prewarming the repo-analysis cache inside the gateway repo test fixture and splitting the brittle `StudioState::new()` bootstrap threshold from the stricter cached-router latency threshold
  - the external Modelica crate now follows a feature-folder module split, with `lib.rs` reduced to public re-exports and internal responsibilities separated across `plugin/entry.rs`, `plugin/analysis.rs`, `plugin/discovery.rs`, `plugin/relations.rs`, and `plugin/parsing.rs`
  - `module.search` now preserves analyzer order for equal-score matches, allowing language plugins such as `xiuxian-wendao-modelica` to project canonical `package.order` semantics into query results instead of having common-core alphabetical tiebreaks overwrite them
  - `example.search` now also preserves analyzer order for equal-score matches, allowing `xiuxian-wendao-modelica` to project canonical example ordering from `package.order` instead of falling back to title/path alphabetical ordering
  - the external Modelica bridge now classifies repository paths into API, example, documentation, and support surfaces before record projection, keeping runnable `Examples/` models in the example surface while treating `Examples/ExampleUtilities` as support-only and `UsersGuide/` as documentation so `symbol.search`, `example.search`, and repository counts stay focused on library/API entities
  - the external Modelica relation layer now links both `UsersGuide` file docs and `UsersGuide` annotation docs to the owning functional module as well as the visible `UsersGuide` module hierarchy, so module-scoped `doc.coverage` can surface nested guide pages and their inline annotation payloads without falling back to root-only linkage
  - the external Modelica discovery layer now also projects semantic `DocRecord.format` hints for `UsersGuide` assets, distinguishing generic guide pages from `Tutorial`, `ReleaseNotes`, `References/Literature`, `Overview`, `Contact`, `Glossar/Glossary`, `Concept/*Concept`, and `Parameters/Parameterization` content while preserving separate `_annotation` variants for inline documentation payloads
  - the external Modelica discovery layer now also orders `UsersGuide` docs with `package.order` semantics plus stable `package.mo`/annotation positioning, while excluding non-doc control files such as `package.order` from `DocRecord` inventory so `doc.coverage` stays focused on actual documentation assets
  - the external Modelica discovery layer now also normalizes file-backed doc titles to page titles instead of raw filenames, so projected docs read `ReleaseNotes`, `Concept`, or `Overview` rather than `ReleaseNotes.mo`, `Concept.mo`, or `Overview.mo`
  - Repo Intelligence now also exposes a deterministic Stage-2 handoff contract through `build_projection_inputs(...)`, emitting `ProjectionInputBundle` seeds so external analyzers such as `xiuxian-wendao-modelica` can verify that `format`, hierarchy, and attached relations survive into projection-ready page families without going through LLM classification
  - the external Modelica package now also maintains its own `docs/` tree with the same section layout as `xiuxian-wendao/docs`, so Modelica-specific architecture, feature notes, research notes, and roadmap progress can be tracked locally instead of only inside Wendao-wide roadmap files
- Focused verification passed:
  - `cargo check -p xiuxian-wendao -p xiuxian-wendao-modelica`
  - `cargo test -p xiuxian-wendao --test repo_example_search`
  - `cargo test -p xiuxian-wendao --test repo_doc_coverage`
  - `cargo test -p xiuxian-wendao --test repo_module_search`
  - `cargo test -p xiuxian-wendao --test repo_symbol_search`
  - `cargo test -p xiuxian-wendao --test repo_overview`
  - `cargo test -p xiuxian-wendao --test repo_sync`
  - `cargo test -p xiuxian-wendao --test repo_relations`
  - `cargo test -p xiuxian-wendao --test repo_intelligence_registry`
  - `cargo test -p xiuxian-wendao-modelica`
  - `cargo test -p xiuxian-ast --features julia --lib`

## Search Primitive Follow-Up

The next bounded refactor slice should establish a crate-local common fuzzy-search layer inside `xiuxian-wendao` before any wider workspace rollout:

- extract reusable lexical distance and normalized fuzzy-scoring helpers into a shared search module
- define shared fuzzy option contracts for edit distance, prefix length, and transposition behavior
- adapt existing Tantivy-backed lookup paths so fuzzy querying is exposed through a reusable matcher boundary instead of feature-local query construction
- migrate touched callers in Wendao to the common primitive layer first, leaving cross-crate search unification as a later decision

Initial bounded progress for that slice is now landed:

- `xiuxian-wendao` now exposes a shared `search` module with reusable lexical and Tantivy-backed fuzzy matchers
- `crate::search::tantivy` now also owns a shared `SearchDocumentIndex` and shared search-document schema so new search backends stop rebuilding Tantivy field layouts ad hoc
- the shared fuzzy hot path now uses a three-row scratch-buffer edit-distance implementation and avoids repeated query lowercasing inside the lexical/Tantivy matcher loops, reducing per-candidate allocation churn
- the shared fuzzy hot path now also reuses query/candidate char buffers and distance scratch space across lexical/Tantivy matcher loops, while public scoring helpers (`edit_distance`, `normalized_score`, `score_candidate`) now borrow thread-local buffers instead of allocating fresh `Vec<char>` / `Vec<usize>` scratch state on every call
- Tantivy best-fragment rescoring now also walks borrowed fragment slices directly instead of first materializing a temporary `Vec<String>` candidate list for each stored title, and only allocates the final `matched_text` once the winning fragment is known
- Tantivy identifier-boundary discovery now also uses a single-pass `Peekable<char_indices()>` state machine instead of collecting every `(byte_idx, char)` pair into a temporary `Vec`, and the splitter semantics are now pinned by direct unit tests for CamelCase, acronym-to-word, and alpha-digit transitions
- `LexicalMatcher::search` now also uses the shared thread-local fuzzy buffers directly instead of constructing fresh `query_chars`, `candidate_chars`, and `scratch` vectors on every search call, and focused tests now verify consecutive lexical searches clear that reused state correctly
- prefix gating now also runs inside the shared candidate-lowercasing pass instead of scanning each candidate once for prefix validation and a second time for lowercase collection, and the prefix comparison now treats non-ASCII case pairs through `char::to_lowercase()` equality rather than ASCII-only case folding
- `FuzzySearchOptions` now also exposes a preparatory `camel_case_symbol()` profile so future Julia/Modelica symbol callers can opt into relaxed prefix gating for CamelCase-style abbreviations without changing the default symbol profile
- `UnifiedSymbolIndex` now supports option-aware fuzzy lookup through the shared matcher layer and now reuses the shared search-document schema instead of maintaining a feature-local Tantivy schema
- Repo Intelligence `module.search`, `symbol.search`, and `example.search` now also build shared `SearchDocumentIndex` instances and use an `exact Tantivy -> fuzzy Tantivy -> legacy lexical fallback` ranking pipeline, so the studio repo handlers inherit the shared search primitives without re-scanning every module/symbol/example row first
- deterministic projected-page search now also uses `build_projected_pages(...)` plus the shared search-document index for exact and fuzzy retrieval before falling back to the existing keyword/path heuristics
- projected-page family search, navigation search, retrieval context, and projected page-index tree lookup now also resolve against the shared projected-page and projected page-index tree builders instead of re-deriving partial views from raw docs
- `build_projected_retrieval_hit(...)` now also resolves stable projected `page_id` values through the shared projected-page lookup instead of misreading them as raw stage-one `doc_id` values
- Tantivy best-fragment rescoring now also expands CamelCase and alpha-digit identifier spans, so symbol-like names get higher-quality secondary matches even before a full custom tokenizer lands
- LinkGraph topology discovery now has a typo-tolerant lexical title fallback backed by the same shared fuzzy primitives
- Studio definition resolution, semantic-auditor fuzzy scoring, and graph dedup edit-distance scoring now reuse the shared primitives instead of carrying isolated edit-distance implementations
- dedicated projection integration targets now validate the shared projected-page search/navigation/retrieval slice through a stable in-memory analysis fixture, avoiding the currently broken built-in Julia plugin bootstrap path while keeping the search-contract assertions in place
- the `repo_projected_` slice of `xiuxian-testing-gate` is now back to green after updating the stale projection fixtures to the current contracts and accepting the deterministic snapshot drift
- the `repo_example_search` slice of `xiuxian-testing-gate` now also passes with shared Tantivy-backed typo handling for example-title queries, and the stale CLI JSON snapshot baseline has been refreshed to the current payload shape
- the filtered `repo_overview` and `repo_sync` slices of `xiuxian-testing-gate` are now green again after restoring overview aggregation semantics and managed-source drift/freshness classification, and the affected overview snapshots have been refreshed to the current symbol/diagnostic payload shape
- focused lib tests now validate typo-tolerant Repo Intelligence module/symbol retrieval through `analyzers::service::search::tests::*`, which stays runnable even while the broader `xiuxian-testing-gate` target is blocked by unrelated compile failures
- projected doc-kind inference now also honors the shared doc-format contract for standalone `reference` docs while still upgrading symbol-anchored explanation docs to `Reference`, which unblocked the shared projected-page lib tests and removed one source of repo-sync payload drift
- the bundled Wendao gateway OpenAPI artifact now also covers `/api/analysis/code-ast`, keeping the route inventory test aligned with the runtime gateway surface
- `cargo test -p xiuxian-wendao --lib` is now green again after refreshing the affected studio Markdown-analysis and repo-sync snapshot baselines to the current response contracts

## Open Constraint

The repository-level AGENTS reference points at `[[.data/blueprints/project_anchor_semantic_addressing.md]]`, but that file is not currently present in the workspace. The Repo Intelligence MVP should therefore treat this roadmap note plus the paired ExecPlan as the immediate execution guide until the canonical semantic-addressing blueprint is restored or replaced.
