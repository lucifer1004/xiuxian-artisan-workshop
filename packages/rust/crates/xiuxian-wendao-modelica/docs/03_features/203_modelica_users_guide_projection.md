# Modelica UsersGuide Projection

:PROPERTIES:
:ID: c4feb9a4bedc81e9d7dab78e779d5fbb3d5e439b
:TYPE: FEATURE
:STATUS: ACTIVE
:END:

The Modelica plugin treats `UsersGuide` as documentation-first structure and projects it into normalized doc inventory plus explicit `Documents` relations.

## Current Projection Rules

- file-backed `UsersGuide` pages become `DocRecord` values
- inline `annotation(Documentation(...))` payloads become separate `DocRecord` values
- `UsersGuide` pages stay out of the default `SymbolRecord` surface
- file-backed docs normalize titles to page names rather than raw filenames

## Link Targets

`UsersGuide` docs currently link to:

- the owning functional module
- the visible `UsersGuide` module hierarchy

That means nested pages such as `Tutorial/*` can appear in module-scoped `doc.coverage` without path-guessing in the common core.

## Semantic Format Hints

The plugin currently emits specialized `DocRecord.format` values for common guide topics:

- tutorial
- conventions
- connectors
- implementation
- revision history
- version management
- release notes
- reference and literature
- overview
- contact
- glossary
- concept
- parameter

Each topic can also appear as an `_annotation` variant when the content comes from inline documentation instead of a file-backed page.

For real `Conventions.mo`-style guide files, the plugin now also synthesizes subsection docs for stable nested topics:

- `Documentation`
- `ModelicaCode`
- `Icons`

These subsection docs are only emitted when the nested declaration block exists and carries its own `annotation(Documentation(...))`.

Synthetic subsection docs keep stable section identities in `doc_id` and `path`, but normalize `DocRecord.title` to projection-ready labels such as `Modelica Code`, `Version Management`, and `Version 4.1.0`.

These `format`, hierarchy, and normalized-title signals are now also smoke-tested through Wendao's deterministic `ProjectionInputBundle` contract instead of staying local to raw repo inventory snapshots.

That projection contract now also preserves shared-target context:

- module reference seeds can absorb child-symbol docs when the symbol belongs to
  the same module
- example how-to seeds can carry direct module-side guide docs and format hints
- guide doc seeds can carry direct module-side examples
- symbol-targeted annotation docs project as `Reference` seeds instead of
  falling back to generic explanation seeds

The same integration path now also validates deterministic `ProjectedPageRecord`
output:

- `Overview` sections preserve page family, source paths, and format hints
- `Anchors` sections preserve module/symbol lineage
- `Sources` or `Documentation` sections preserve linked guide pages
- `Examples` sections preserve direct example associations

The same integration path now also validates the parser-facing handoff:

- projected pages render into stable virtual markdown paths
- projected markdown reuses the existing Wendao markdown parser
- parsed section summaries preserve heading order, heading depth, and section
  property-drawer attributes for downstream page-index construction

The same path now also validates real page-index tree construction:

- projected markdown sections pass through Wendao's actual `page_index` builder
- projected tree summaries preserve structural paths, token counts, and thinning
  state
- the Modelica integration snapshots now pin those tree summaries for selected
  guide and symbol pages

For real `ReleaseNotes.mo`-style guide files, the plugin now also synthesizes subsection docs for stable nested release-note topics:

- `VersionManagement`
- annotated `Version_*` entries

These subsection docs are only emitted when the nested declaration block exists and carries its own `annotation(Documentation(...))`.

## Open Follow-Up

- extend topic recognition only when the subtree naming is stable across real MSL samples
- feed these format hints directly into second-stage document projection rather than keeping them repo-intelligence-only
