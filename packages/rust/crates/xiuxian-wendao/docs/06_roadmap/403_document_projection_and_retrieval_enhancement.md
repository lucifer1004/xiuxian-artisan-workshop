# Document Projection and Retrieval Enhancement

:PROPERTIES:
:ID: wendao-document-projection-retrieval
:PARENT: [[index]]
:TAGS: roadmap, docs, page-index, ppr, diataxis
:STATUS: PLANNED
:END:

## Purpose

This roadmap note defines the second-stage architecture that sits on top of Repo Intelligence MVP.

Stage 1 establishes repository records and stable repository queries:

- `repo.overview`
- `module.search`
- `symbol.search`
- `example.search`
- `doc.coverage`

Stage 2 projects those indexed records into a documentation layer and enhances retrieval using existing Wendao kernels such as page index, graph diffusion, weighted fusion, and agentic retrieval.

This roadmap note is scoped to the current target scenarios:

- Julia SciML repositories
- MSL (Modelica Standard Library)

## Position in the Architecture

The intended layering is:

```text
Stage 1: Repo Intelligence
  git mirror -> sync -> normalized records -> repo graph -> stable repo queries

Stage 2: Document Projection and Retrieval Enhancement
  repo records -> document pages -> page index -> graph-enhanced retrieval -> deep wiki surfaces
```

This stage does not replace Repo Intelligence. It consumes Repo Intelligence outputs and turns them into documentation objects and richer retrieval behavior.

## Core Principle

Wendao should not generate documentation directly from raw search results.

Instead, the system should:

1. index repository structure into stable records
2. project those records into documentation pages
3. index page structure with existing Wendao page kernels
4. run retrieval over both repository records and projected documentation pages

The key shift is from "search raw chunks and ask the LLM to improvise" to "project stable pages from indexed truth, then retrieve over those pages."

## Projection Model

### Target Page Types

The projection layer should build four first-class page families aligned with Diataxis:

- `ReferencePage`
- `HowToPage`
- `TutorialPage`
- `ExplanationPage`

### Projection Inputs

The projection engine consumes Stage 1 records:

- `RepositoryRecord`
- `ModuleRecord`
- `SymbolRecord`
- `ExampleRecord`
- `DocRecord`
- `RelationRecord`

The first landed contract for this boundary should stay minimal: a deterministic
`ProjectionInputBundle` emitted directly from `RepositoryAnalysisOutput`, with
stable `ProjectionPageSeed` rows grouped by `Reference`, `HowTo`, `Tutorial`,
and `Explanation`.

The next deterministic refinement should stay graph-native rather than
LLM-driven:

- module reference seeds should aggregate child-symbol docs and examples
- example how-to seeds should carry direct target-side docs and format hints
- doc seeds should carry direct target-side example anchors
- doc page-family classification should treat symbol-targeted docs and
  explicitly reference-like formats as `Reference` instead of collapsing all
  non-tutorial docs into `Explanation`

The next landed contract after seeds should stay equally deterministic:

- `ProjectedPageRecord`
- `ProjectedPageSection`

These records should preserve page family, source anchors, source paths, and a
small stable section set such as `Overview`, `Anchors`, `Sources`, and
`Examples`, without depending on prose generation or Markdown rendering.

The next parser-facing refinement should also stay deterministic:

- `ProjectedMarkdownDocument`
- `ProjectedPageIndexDocument`

These records should render projected pages into stable virtual markdown paths,
then reuse the existing markdown parser so Stage 2 can hand page-index-ready
sections downstream without persisting synthetic files first.

The next landed refinement after that should connect those parsed sections to the
real page-index builder:

- `ProjectedPageIndexTree`
- `ProjectedPageIndexNode`

These records should preserve the actual builder semantics, including heading
hierarchy, structural paths, token counts, and thinning state, while staying
serializable for snapshot and gateway-facing inspection.

The next external-consumption refinement should expose those deterministic
projection contracts through the existing Studio gateway surface:

- `GET /api/repo/projected-pages?repo=<id>`
- `GET /api/repo/projected-page?repo=<id>&page_id=<stable-id>`
- `GET /api/repo/projected-page-index-tree?repo=<id>&page_id=<stable-id>`
- `GET /api/repo/projected-page-index-trees?repo=<id>`

These routes should stay read-only and should return the same deterministic
Stage-2 payloads already pinned in the library snapshot lane instead of adding
gateway-only projection shapes.

The next retrieval refinement on top of those inspection routes should stay
deterministic as well:

- `GET /api/repo/projected-page-index-tree-search?repo=<id>&query=<text>&kind=<family>&limit=<n>`
- `GET /api/repo/projected-page-search?repo=<id>&query=<text>&kind=<family>&limit=<n>`
- `GET /api/repo/projected-retrieval?repo=<id>&query=<text>&kind=<family>&limit=<n>`

These routes should search existing deterministic projected pages and
builder-native projected page-index trees by title, path, format hint,
structural-path, and section-title evidence, with an optional page-family
filter, instead of introducing LLM-ranked retrieval at this stage. The mixed
retrieval route should merge page-level hits and builder-native section hits
under one deterministic contract so downstream Stage-2 consumers do not need to
manually fan out across the separate page and tree-search surfaces.

The next lookup refinement on top of those search and inspection routes should
also stay deterministic:

- `GET /api/repo/projected-retrieval-hit?repo=<id>&page_id=<stable-id>&node_id=<stable-node-id?>`
- `GET /api/repo/projected-retrieval-context?repo=<id>&page_id=<stable-id>&node_id=<stable-node-id?>&related_limit=<n>`
- `GET /api/repo/projected-page-family-context?repo=<id>&page_id=<stable-id>&per_kind_limit=<n>`
- `GET /api/repo/projected-page-family-cluster?repo=<id>&page_id=<stable-id>&kind=<family>&limit=<n>`
- `GET /api/repo/projected-page-index-node?repo=<id>&page_id=<stable-id>&node_id=<stable-node-id>`
- `GET /api/repo/projected-page-index-tree?repo=<id>&page_id=<stable-id>`

These routes should resolve one mixed retrieval hit, one builder-native
projected page-index node, one local mixed-hit context bundle, one grouped
page-family context bundle, one singular page-family cluster, or one full
projected page-index tree directly by stable identifiers instead of forcing
downstream consumers to fetch the full search result set or projected page-index
tree set before opening one specific page, tree, anchor, or family cluster.

The page-family context route should stay deterministic and graph-native:

- resolve one center `ProjectedPageRecord` by stable `page_id`
- rank related projected pages by shared module/symbol/example/doc anchors
- group those related pages by `ProjectionPageKind`
- preserve the shared-anchor score for each related page instead of flattening
  family context into an unannotated list

The next family-discovery refinement should stay equally deterministic:

- `GET /api/repo/projected-page-family-search?repo=<id>&query=<text>&kind=<family>&limit=<n>&per_kind_limit=<n>`

This route should search stable center projected pages first, then expand each
matched center page into grouped family clusters using the same shared-anchor
evidence as `projected-page-family-context`, so downstream Stage-2 consumers
can discover local family neighborhoods without already knowing a stable
`page_id`.

The next singular family-opening refinement should stay equally deterministic:

- `GET /api/repo/projected-page-family-cluster?repo=<id>&page_id=<stable-id>&kind=<family>&limit=<n>`

This route should resolve one stable center `ProjectedPageRecord`, require one
specific `ProjectionPageKind`, and return exactly one grouped family cluster
ranked by the same shared-anchor evidence as the broader family-context and
family-search lanes, so downstream Stage-2 consumers can reopen one family
cluster directly without fetching all families first.

The next page-centric navigation refinement should compose those singular
lookups into one deterministic bundle:

- `GET /api/repo/projected-page-navigation?repo=<id>&page_id=<stable-id>&node_id=<stable-node-id?>&family_kind=<family?>&related_limit=<n>&family_limit=<n>`

This route should return one stable center mixed hit, the full projected
page-index tree for the page, the optional local node neighborhood, related
projected pages ranked by shared anchors, and an optional singular family
cluster when a `family_kind` is requested, so downstream Stage-2 consumers can
open one projected page with its immediate deterministic navigation context in
one call instead of stitching together retrieval-context, tree, and
family-cluster routes manually.

The next navigation-discovery refinement should stay deterministic as well:

- `GET /api/repo/projected-page-navigation-search?repo=<id>&query=<text>&kind=<family>&family_kind=<family?>&limit=<n>&related_limit=<n>&family_limit=<n>`

This route should search stable center projected pages first, then expand each
matched page into the same page-centric navigation bundle used by
`projected-page-navigation`, optionally keeping only hits that can also resolve
the requested `family_kind`. That gives downstream Stage-2 consumers one search
surface that opens page-level context, page-index trees, related pages, and an
optional family cluster without a second round trip per hit.

### Projection Rules

The initial projection rules should stay deterministic:

- `ReferencePage`
  - sourced from exported modules, symbols, signatures, and direct documentation
- `HowToPage`
  - sourced from minimal runnable examples and task-oriented example clusters
- `TutorialPage`
  - sourced from ordered learning paths, long-form guides, and example sequences
- `ExplanationPage`
  - sourced from conceptual docs, architecture notes, UsersGuide sections, and semantic relation clusters

The first implementation should prefer explicit rules and graph evidence over LLM classification.

## Integration with Existing Wendao Kernels

This is the main architectural purpose of the stage: connect the existing Wendao kernels to the repository documentation layer.

### Page Index

`page index` should become the primary indexing kernel for projected documentation pages.

Responsibilities:

- build hierarchical page structure
- preserve section ancestry
- support section-level navigation and injection
- provide stable page and section identities for projected documentation

In this stage, `page index` is not the source of repository truth. It is the structural index over projected documentation truth.

### Link Graph and PPR

`link_graph` and `PPR` should operate over a richer mixed graph containing:

- repository records
- projected documentation pages
- page sections
- example-to-symbol links
- concept-to-module links

This allows PPR to surface not only structurally central code entities, but also structurally central documentation pages and explanation hubs.

### Weighted Fusion

Existing weighted fusion should become a shared retrieval layer across:

- repository entities
- projected pages
- page sections
- examples
- conceptual explanation nodes

The retrieval target is no longer "document chunks only." It becomes a multi-object retrieval surface.

### Agentic Retrieval

`agentic retrieval` should be positioned as an advanced retrieval mode for:

- cross-page learning path expansion
- architecture walkthroughs
- multi-hop explanation generation
- documentation gap discovery

It should not be required for the baseline projection pipeline.

## SciML and MSL Projection Strategy

### SciML

SciML repositories often split truth across:

- root package metadata
- exported Julia APIs
- external docs sites
- tutorial repositories
- benchmark/example repositories

For SciML, the projection layer should emphasize:

- API-centric `ReferencePage`
- example-driven `HowToPage`
- ecosystem-linked `ExplanationPage`

`TutorialPage` should remain optional until cross-repository tutorial aggregation is stable.

### MSL

MSL already exposes strong documentation conventions:

- `package.mo`
- `UsersGuide`
- `Examples`
- package hierarchy

For MSL, the projection layer should emphasize:

- `ReferencePage` from package and class definitions
- `TutorialPage` and `ExplanationPage` from `UsersGuide`
- `HowToPage` from `Examples`

MSL is therefore a strong early proving ground for deterministic projection.

## Retrieval Enhancement Modes

The stage-2 retrieval layer should expose at least four high-level modes:

- `reference`
- `tutorial`
- `howto`
- `explanation`

These are retrieval filters over projected pages, not ad-hoc prompt labels.

Example CLI shape:

```bash
wendao docs search --repo sciml-diffeq --mode reference --query "solve options"
wendao docs search --repo msl --mode tutorial --query "fluid heat exchanger"
wendao docs page --repo msl --page "Modelica.Fluid.UsersGuide"
```

## Gap Detection and Coverage

Stage 2 should also compute documentation gaps from the difference between Stage 1 truth and projected documentation.

Examples:

- module has exported symbols but no projected reference page
- symbol appears in examples but has no documentation link
- example exists but is not attached to any how-to path
- UsersGuide concept node has no explanation projection

This coverage signal should become a first-class input for future wiki expansion and auditing.

## Qianji Boundary

Qianji should remain outside the deterministic projection kernel.

Its role in Stage 2 is optional and bounded:

- page refinement
- page classification review
- prose cleanup
- audit and contradiction checks

Qianji must not become the only source of page structure or page truth.

## Execution Phases

1. Build projected page records from Repo Intelligence outputs.
2. Attach `page index` to projected pages and sections.
3. Extend mixed-graph retrieval to include pages and sections.
4. Add mode-aware docs retrieval (`reference`, `tutorial`, `howto`, `explanation`).
5. Add gap detection and coverage reporting.
6. Add optional Qianji-assisted refinement and audit.

## Expected Outcome

After this stage, Wendao should support:

- fast repository understanding through Stage 1 records
- MATLAB-like documentation surfaces through Stage 2 projection
- retrieval that understands both repository structure and page structure
- a clean path toward deep wiki generation without depending on raw LLM improvisation

## Why This Split Matters

Separating this stage from Repo Intelligence MVP prevents three common failures:

- confusing repository truth with projected documentation truth
- overloading the first-stage common core with page-generation concerns
- burying existing Wendao kernels such as page index and PPR under a repo-schema-only narrative

This split keeps Stage 1 focused on repository truth and Stage 2 focused on projected knowledge surfaces and retrieval enhancement.
