# xiuxian-wendao-modelica

External Modelica Repo Intelligence plugin for `xiuxian-wendao`.

Current scope:

- expose a stable external crate boundary for `plugins = ["modelica"]`
- provide conservative repository, module, symbol, example, and documentation records for Modelica repositories
- validate that external plugins can use the same common-core query surface through registry-aware Repo Intelligence entry points
- register a `modelica` plugin that currently discovers:
  - root and nested `package.mo` modules
  - lightweight symbol declarations from `.mo` files
  - examples under `Examples/`
  - tutorial, documentation, and support-only models under `Examples/ExampleUtilities`, `Examples/.../Utilities`, `Internal`, and `UsersGuide/` kept out of the default `SymbolRecord` surface, with support-only package modules also excluded from the default `ModuleRecord` surface
  - docs from `README*`, `UsersGuide`, and conservative `annotation(Documentation(...))` extraction, including `UsersGuide` pages with inline annotations
  - repository files through a hidden-directory-aware walker that skips `.git` and other dot-prefixed paths
  - canonical module and example ordering from discovered `package.order` files
  - an explicit repository-surface classifier that separates default API, example, documentation, and support paths before record projection
  - `UsersGuide` file docs and `UsersGuide` annotation docs now link to the owning functional module and the visible `UsersGuide` module hierarchy, so module-scoped coverage can see nested guide pages such as `Tutorial/*`, `Overview/*`, and their inline annotation payloads
  - `UsersGuide` docs now also emit semantic `DocRecord.format` hints such as `modelica_users_guide_tutorial`, `modelica_users_guide_conventions`, `modelica_users_guide_connectors`, `modelica_users_guide_implementation`, `modelica_users_guide_revision_history`, `modelica_users_guide_version_management`, `modelica_users_guide_release_notes`, `modelica_users_guide_reference`, `modelica_users_guide_overview`, `modelica_users_guide_contact`, `modelica_users_guide_glossary`, `modelica_users_guide_concept`, and `modelica_users_guide_parameter`, with matching `_annotation` variants for inline `annotation(Documentation(...))` payloads
  - `Conventions.mo` now also projects synthetic subsection docs for stable nested guide topics (`Documentation`, `ModelicaCode`, and `Icons`) when the nested declaration block itself carries `annotation(Documentation(...))`
  - `ReleaseNotes.mo` now also projects synthetic subsection docs for stable nested release-note topics (`VersionManagement` plus annotated `Version_*` entries)
  - synthetic subsection docs keep stable section identities in `doc_id`/`path` while normalizing `DocRecord.title` to projection-ready labels such as `Modelica Code`, `Version Management`, and `Version 4.1.0`
  - `UsersGuide` doc inventory is now ordered with `package.order` semantics plus stable `package.mo`/annotation positioning, and non-doc control files such as `package.order` are excluded from `DocRecord` projection
  - file-backed docs now normalize `DocRecord.title` to page titles such as `ReleaseNotes`, `Concept`, or `Overview` instead of raw filenames like `ReleaseNotes.mo`
  - normalized `Contains`, `Declares`, `ExampleOf`, and `Documents` relations

Current parser strategy is intentionally conservative. It relies on filesystem layout plus simple declaration scanning for `.mo` files and avoids claiming full Modelica semantic completeness.

Current dependency boundary is now explicit:

- production contracts come from `xiuxian-wendao-core::repo_intelligence`
- `xiuxian-wendao` consumes this crate through a normal optional Cargo dependency instead of sibling-source inclusion
- registry-aware integration-query validation still uses `xiuxian-wendao` as a dev-dependency only

Package-local tracking docs now live under [docs/index.md](/Users/guangtao/projects/xiuxian-artisan-workshop/packages/rust/crates/xiuxian-wendao-modelica/docs/index.md), mirroring the higher-level `xiuxian-wendao/docs` structure so Modelica-specific architecture, feature notes, research notes, and roadmap progress can evolve independently. The first split is now explicit:

- `docs/01_core/` for extension-boundary and package architecture notes
  - includes the package-local document identity protocol for opaque hash-based `:ID:` values
- `docs/03_features/` for concrete Modelica/MSL behavior tracking
- `docs/05_research/` for repository-convention notes
- `docs/06_roadmap/` for package-local implementation progress

Current crate layout is intentionally modular:

- [lib.rs](/Users/guangtao/projects/xiuxian-artisan-workshop/packages/rust/crates/xiuxian-wendao-modelica/src/lib.rs) only re-exports the public plugin surface
- [plugin/entry.rs](/Users/guangtao/projects/xiuxian-artisan-workshop/packages/rust/crates/xiuxian-wendao-modelica/src/plugin/entry.rs) owns the plugin trait implementation
- [plugin/analysis.rs](/Users/guangtao/projects/xiuxian-artisan-workshop/packages/rust/crates/xiuxian-wendao-modelica/src/plugin/analysis.rs) orchestrates repository analysis
- [plugin/discovery.rs](/Users/guangtao/projects/xiuxian-artisan-workshop/packages/rust/crates/xiuxian-wendao-modelica/src/plugin/discovery.rs) owns repository walking, package-order discovery, and normalized record collection
- [plugin/relations.rs](/Users/guangtao/projects/xiuxian-artisan-workshop/packages/rust/crates/xiuxian-wendao-modelica/src/plugin/relations.rs) owns `Contains`, `Declares`, `ExampleOf`, and `Documents` relation construction
- [plugin/parsing.rs](/Users/guangtao/projects/xiuxian-artisan-workshop/packages/rust/crates/xiuxian-wendao-modelica/src/plugin/parsing.rs) owns conservative Modelica parsing helpers and parser snapshots
