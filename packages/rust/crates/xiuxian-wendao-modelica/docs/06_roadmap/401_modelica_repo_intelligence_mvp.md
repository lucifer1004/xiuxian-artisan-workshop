# Modelica Repo Intelligence MVP

:PROPERTIES:
:ID: b233f9e3d413b7eb1748e734dd76f4eaf5da7167
:TYPE: ROADMAP
:STATUS: ACTIVE
:END:

This roadmap tracks the external Modelica plugin as a first-class package, separate from the Wendao common-core roadmap.

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
- external crate contracts are now realigned to the live `xiuxian-wendao::analyzers` record/import schemas again, restoring a green package test lane against the current common core
- the shared Wendao plus external Modelica slice is now green on the Tier-3 lane (`clippy` plus `nextest`), so follow-up work can move from contract repair back to feature hygiene and bounded semantic growth

## Active Tracking Focus

- keep package-level documentation in this crate instead of only under `xiuxian-wendao/docs`
- track Modelica-specific behavior changes independently from the Wendao common-core roadmap
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
- `cargo clippy -p xiuxian-wendao -p xiuxian-wendao-modelica --all-targets --all-features -- -D warnings`
- `cargo nextest run -p xiuxian-wendao -p xiuxian-wendao-modelica --no-fail-fast`
