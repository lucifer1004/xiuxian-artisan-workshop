# Wendao Parser Docs

:PROPERTIES:
:ID: wendao-parser-docs
:PARENT: [[index]]
:TAGS: parser, architecture, core
:STATUS: ACTIVE
:END:

This directory is the stable documentation home for parser ownership,
canonical parser families, and parser implementation rules in
`xiuxian-wendao`.

## Documents

- [architecture.md](architecture.md): Canonical parser namespace, parser-family
  matrix, parser-vs-helper decision rule, and structural parsing principles.
- [references.md](references.md): Unified ordinary Markdown reference grammar
  for `[...](...)` and `[[...]]`, plus the parser-owned consumer boundary.
- [wikilinks.md](wikilinks.md): Obsidian-aligned ordinary body wikilink
  grammar, comrak-backed extraction, and the `link_graph_refs` consumer
  boundary.
- [relation_semantics.md](relation_semantics.md): The difference between
  global `[[...]]` topology links, section-scoped property-drawer relations,
  local property scalar metadata, and the snapshot-backed regression contract.
- [../06_roadmap/419_parser_substrate_separation.md](../06_roadmap/419_parser_substrate_separation.md):
  Future-facing package split for shared syntax-only document parsing versus
  Wendao-owned domain adapters.

## Current Canonical Parser Families

- `src/parsers/markdown/`
- `src/parsers/link_graph/query/`
- `src/parsers/zhixing/tasks/`
- `src/parsers/languages/rust/cargo/`
- `src/parsers/languages/python/pyproject/`
- `src/parsers/search/repo_code_query/`
- `src/parsers/graph/persistence/`

## What This Directory Governs

1. Which parser behavior belongs under `src/parsers/`
2. Which parse-like helpers stay adapter-local or subsystem-local
3. How parser modules should be split and tested
4. Why `[[...]]` links should establish graph topology before any semantic
   typing
5. Why explicit metadata and real structural signals should win over hardcoded
   link strings
6. How `PROPERTIES` scoped relations differ from global wiki links and scalar
   metadata
7. How ordinary Markdown reference parsing stays parser-owned and comrak-backed
8. How the narrower ordinary wikilink subset relates to the shared reference
   parser surface
9. How shared syntax-only document parsing should be separated from
   Wendao-owned domain adapters for cross-crate consumers

:RELATIONS:
:LINKS: [[01_core/103_package_layering]], [[06_roadmap/405_large_rust_modularization]], [[06_roadmap/419_parser_substrate_separation]], [[03_features/210_search_queries_architecture]], [[02_parser/references]], [[02_parser/wikilinks]], [[02_parser/relation_semantics]]
:END:

---

:FOOTER:
:LAST_SYNC: 2026-04-10
:END:
