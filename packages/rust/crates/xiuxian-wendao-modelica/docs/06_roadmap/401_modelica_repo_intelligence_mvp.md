# Modelica Repo Intelligence MVP

:PROPERTIES:
:ID: b233f9e3d413b7eb1748e734dd76f4eaf5da7167
:TYPE: ROADMAP
:STATUS: ACTIVE
:END:

This roadmap tracks the external Modelica plugin as a first-class package, separate from the Wendao common-core roadmap.

Current program position:

- the external Modelica package is now the completed `M6` additive proof path
  for Wendao
- that additive proof now spans repo-facing, docs-facing, and Studio-facing
  consumers, including the Stage-A repo service-state bundle
- the next governed program move is `Phase 7: Flight-First Runtime
Negotiation`, not more outward-surface expansion

## Landed

- external crate boundary validated through `plugins = ["modelica"]`
- modular plugin layout under `src/plugin/`
- conservative package and symbol discovery from `package.mo` and `.mo` files
- repository-surface classification for API, example, documentation, and support paths
- conservative support classification for `Internal` paths and `Examples/.../Utilities`
- support-only package modules excluded from the default module surface
- hidden/VCS path filtering during repository walking
- `Examples/` indexing with `package.order`-aware ordering
- `UsersGuide` file and annotation docs projected as `DocRecord` values
- `UsersGuide` docs linked to owning functional modules and visible guide hierarchy modules
- semantic `DocRecord.format` hints for common MSL guide topics
- `Conventions` projected as a first-class `UsersGuide` topic hint
- stable nested `Conventions` topics (`Documentation`, `ModelicaCode`, `Icons`) projected as synthetic subsection docs
- `Connectors` projected as a first-class `UsersGuide` topic hint
- `Implementation` and `RevisionHistory` projected as first-class `UsersGuide` topic hints
- `VersionManagement` projected as a first-class `UsersGuide` topic hint
- stable nested `ReleaseNotes` topics (`VersionManagement` and annotated `Version_*` entries) projected as synthetic subsection docs
- synthetic subsection docs normalized to projection-ready human titles while keeping stable section identities
- projection-ready `ProjectionInputBundle` consumption validated from the external Modelica integration path
- shared-target projection context validated from the external Modelica integration path, so module reference seeds can absorb child-symbol docs, example how-to seeds can carry direct guide docs, and symbol-targeted annotation docs classify as reference seeds
- deterministic `ProjectedPageRecord` output validated from the external Modelica integration path, including stable `Overview`, `Anchors`, `Sources/Documentation`, and `Examples` sections
- parser-facing `ProjectedMarkdownDocument` and `ProjectedPageIndexDocument` output validated from the external Modelica integration path, so Stage-2 projected pages can reuse the existing markdown parser before page-index ingestion
- real `ProjectedPageIndexTree` output validated from the external Modelica integration path, so projected Modelica guide pages now carry builder-native page-index hierarchy and thinning signals
- normalized page titles for file-backed docs
- production contract ownership now comes from
  `xiuxian-wendao-core::repo_intelligence`, while registry-aware integration
  query validation keeps `xiuxian-wendao` only as a dev-dependency
- the host now consumes `xiuxian-wendao-modelica` through a normal optional
  Cargo dependency instead of sibling-source inclusion
- Modelica integration fixtures now create deterministic repositories through
  CLI git helpers instead of `git2`
- the workspace root and `xiuxian-wendao-modelica` manifests no longer carry
  `git2`
- the snapshot-backed Modelica repo-intelligence contract now redacts
  concrete revision hashes, keeping backend cutovers backend-neutral
- the host `xiuxian-wendao` test gate now validates the external Modelica
  plugin through builtin-registry repo-overview, module-search,
  example-search, and symbol-search entry points
- the host `xiuxian-wendao` test gate now also validates the same external
  Modelica path through a relation-graph regression, so the additive proof
  covers structural/semantic relation output in addition to search consumers
- the host `xiuxian-wendao` test gate now also validates config-backed
  projected-page generation and deterministic projected-page lookup from the
  same external Modelica path
- the host `xiuxian-wendao` test gate now also validates projected page-index
  tree generation and deterministic tree lookup from the same external
  Modelica path
- the host `xiuxian-wendao` test gate now also validates projected page-index
  node lookup from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates page-centric
  projected navigation bundles from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates grouped projected
  page-family context from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates singular projected
  page-family cluster lookup from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates projected
  page-family search from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates projected
  page-navigation search from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  projected page-navigation search from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  projected page-family search from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  projected page-family context from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  projected page-navigation lookup from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  projected page-family cluster lookup from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  projected page lookup from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  projected page-index tree lookup from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  projected page-index node lookup from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  projected page-index tree search from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  projected page-index tree listing from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  projected page-index document generation from the same external Modelica
  path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  projected markdown document generation from the same external Modelica
  path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  projected page search from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing mixed
  projected retrieval from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing local
  projected retrieval context from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  deterministic projected retrieval-hit reopening from the same external
  Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  projected gap reporting from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  deterministic planner queue shaping from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  deterministic planner workset shaping from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  deterministic planner ranking from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  deterministic planner item reopening from the same external Modelica path
- the host `xiuxian-wendao` test gate now also validates docs-facing
  deterministic planner search from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/docs/planner-search` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/docs/planner-item` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/docs/planner-workset` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/docs/planner-rank` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/docs/planner-queue` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/docs/search` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/docs/retrieval` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/docs/retrieval-context` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/docs/retrieval-hit` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/docs/page` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/docs/family-context` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/docs/family-search` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/docs/family-cluster` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/docs/navigation` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/docs/navigation-search` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/docs/projected-gap-report` route from the same external Modelica
  path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/repo/overview` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/repo/module-search` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/repo/symbol-search` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/repo/example-search` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/repo/doc-coverage` route from the same external Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/repo/sync`, `/api/repo/projected-pages`, and
  `/api/repo/projected-gap-report` routes from the same external Modelica
  path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/repo/projected-page`, `/api/repo/projected-page-index-tree`,
  `/api/repo/projected-page-index-node`, `/api/repo/projected-retrieval-hit`,
  and `/api/repo/projected-retrieval-context` routes from the same external
  Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Studio
  `/api/repo/projected-page-index-tree-search`,
  `/api/repo/projected-page-search`, `/api/repo/projected-retrieval`,
  `/api/repo/projected-page-family-context`,
  `/api/repo/projected-page-family-search`,
  `/api/repo/projected-page-family-cluster`,
  `/api/repo/projected-page-navigation`,
  `/api/repo/projected-page-navigation-search`, and
  `/api/repo/projected-page-index-trees` routes from the same external
  Modelica path
- the host `xiuxian-wendao` lib-test lane now also validates the Stage-A
  Studio repo service-state bundle `/api/repo/index` and
  `/api/repo/index/status` from the same external Modelica path
- the shared host integration support now mounts repo fixtures once under
  `tests/integration/support/`, so the external Modelica proof no longer
  depends on per-file support copies or local dead-code suppressions
- the shared Wendao plus external Modelica slice is now green on the Tier-3 lane (`clippy` plus `nextest`), so follow-up work can move from contract repair back to feature hygiene and bounded semantic growth

## Active Tracking Focus

- keep package-level documentation in this crate instead of only under `xiuxian-wendao/docs`
- track Modelica-specific behavior changes independently from the Wendao common-core roadmap
- keep the production dependency boundary on `xiuxian-wendao-core` rather than
  reintroducing a monolithic host contract dependency
- preserve conservative semantics until a richer Modelica parsing layer exists

## Next Steps

1. Continue tightening MSL-aware topic inference for additional guide subtrees where the signal is stable.
2. Keep `BaseClasses` unresolved until there is stronger evidence than the safer `Internal`, `Examples/.../Utilities`, and support-module exclusion signals.
3. Feed Modelica doc `format` and hierarchy signals into the second-stage document-projection pipeline.
4. Add package-local feature notes here as the external crate grows beyond the MVP slice.

## Verification Lane

Current package-level verification for this roadmap slice:

- `direnv exec . cargo check -p xiuxian-wendao-modelica`
- `direnv exec . cargo test -p xiuxian-wendao-modelica`
- `direnv exec . cargo clippy -p xiuxian-wendao -p xiuxian-wendao-modelica --all-targets --all-features -- -D warnings`
- `direnv exec . cargo nextest run -p xiuxian-wendao -p xiuxian-wendao-modelica --no-fail-fast`
