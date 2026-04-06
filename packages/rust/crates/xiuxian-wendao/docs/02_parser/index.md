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

## Current Canonical Parser Families

- `src/parsers/markdown/`
- `src/parsers/link_graph/query/`
- `src/parsers/zhixing/tasks/`
- `src/parsers/cargo/dependencies/`
- `src/parsers/search/repo_code_query/`
- `src/parsers/graph/persistence/`

## What This Directory Governs

1. Which parser behavior belongs under `src/parsers/`
2. Which parse-like helpers stay adapter-local or subsystem-local
3. How parser modules should be split and tested
4. Why `[[...]]` links should establish graph topology before any semantic
   typing
5. Why explicit metadata or suffix signals should win over hardcoded link
   strings

:RELATIONS:
:LINKS: [[01_core/103_package_layering]], [[06_roadmap/405_large_rust_modularization]], [[03_features/210_search_queries_architecture]]
:END:

---

:FOOTER:
:LAST_SYNC: 2026-04-05
:END:
