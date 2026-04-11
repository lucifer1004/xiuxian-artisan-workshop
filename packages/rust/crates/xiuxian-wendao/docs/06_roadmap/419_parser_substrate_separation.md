# Parser Substrate Separation

:PROPERTIES:
:ID: wendao-parser-substrate-separation
:PARENT: [[index]]
:TAGS: roadmap, parser, packages, qianji, qianhuan, markdown, org
:STATUS: ACTIVE
:END:

## Purpose

This note answers one future-facing parser question:

> how should Wendao parser capability be separated so lightweight consumers can
> reuse document parsing without depending on the whole `xiuxian-wendao`
> business-domain crate?

The answer is not "leave parsers under Wendao forever," and it is also not
"move the current directory without changing contracts." The clean target is an
independent parser crate, tentatively `xiuxian-wendao-parsers`, achieved
through contract refactoring rather than a directory lift.

## One-Sentence Rule

- parser-owned parsing belongs in an independent crate tentatively named
  `xiuxian-wendao-parsers`
- Wendao-owned adapters that construct graph, retrieval, persistence, or other
  domain records stay in `xiuxian-wendao`

## Why This Split Exists

Three forces now meet in one place:

1. `xiuxian-qianji` already imports `xiuxian_wendao::parse_frontmatter`
2. future consumers such as `xiuxian-qianhuan` persona and template flows will
   benefit from shared document parsing
3. the parser lane is expected to grow beyond Markdown into Org document
   structure

Those forces justify a reusable parser substrate, but they do not justify
keeping parser ownership inside the main crate forever.

## Why Not A Raw Directory Move

The current `src/parsers/` tree mixes three different layers:

1. syntax-only parsing
2. parser-plus-intermediate-model assembly
3. Wendao domain projection

Examples:

1. Markdown note parsing currently builds `LinkGraphDocument`
2. link-graph query parsing currently merges into `LinkGraphSearchOptions`
3. graph persistence parsing currently constructs `Entity` and `Relation`

That means a clean direct extraction requires contract refactoring first. The
target is still direct parser independence, but the path is:

1. define parser-owned intermediate contracts
2. move parser execution to the independent crate
3. keep Wendao as a consumer and adapter over those contracts

## Current Audit Findings

### Direct parser-layer candidates

These families are good extraction candidates because their outputs can become
parser-owned and reusable:

1. Markdown frontmatter parsing
2. Markdown heading and section structure
3. Markdown references and wiki-link structure
4. shared source spans and format-agnostic document coordinates
5. future Org structural parsing

### Wendao-owned adapters

These families stay in `xiuxian-wendao` because they currently encode Wendao
semantics, not only syntax:

1. note parsing that builds `LinkGraphDocument`
2. link-graph query parsing that merges into `LinkGraphSearchOptions`
3. graph persistence parsing that constructs `Entity` and `Relation`
4. resource and graph adapters that enrich syntax into Wendao-specific records

### Local helpers that should not be promoted

These remain local until they prove reusable:

1. gateway request DTO parsing
2. adapter-local config parsing
3. subsystem-local payload decoders
4. one-off rendering or validation helpers

## Target Package Shape

| Package                       | Owns                                                                  | Must not own                                                                   |
| :---------------------------- | :-------------------------------------------------------------------- | :----------------------------------------------------------------------------- |
| `xiuxian-wendao-parsers`      | independent parser execution, parser-owned Markdown and Org contracts | Wendao graph/search/storage records, query DSLs, persistence semantics         |
| `xiuxian-wendao`              | document-to-domain adapters, graph semantics, retrieval DSLs, storage | parser ownership that non-Wendao consumers should import directly              |
| `xiuxian-qianji` / `qianhuan` | app-specific interpretation of syntax records                         | private copies of shared Markdown or Org grammar that should live in substrate |
| `xiuxian-ast`                 | code AST and language analysis                                        | shared Markdown or Org document grammar                                        |

## Extraction Rules

A parser surface should move to the independent parser layer only when all of
the following are true:

1. the input is a durable document-format grammar rather than a Wendao route or
   query language
2. the output can be represented as parser-owned contracts without
   `LinkGraphDocument`,
   `LinkGraphSearchOptions`, `Entity`, `Relation`, `WendaoResourceUri`, or
   other Wendao-owned types
3. at least one non-Wendao consumer can use the result directly
4. parser-owned tests can express the contract without booting Wendao domain
   services

If any of those conditions are false, keep the parser in `xiuxian-wendao`
temporarily and extract a smaller parser-owned contract first.

## First Three Slices

1. Planning and audit
   - classify current parser families by owner
   - record the direct parser-layer direction and non-goals
2. Frontmatter extraction
   - create `xiuxian-wendao-parsers`
   - move shared Markdown frontmatter parsing and its parser-owned record there
   - retarget `xiuxian-qianji` so it no longer depends on `xiuxian-wendao`
     only for frontmatter
3. Document-structure extraction
   - move shared Markdown structural parsing there
   - define parser-owned document models for future Org support
   - keep Wendao note parsing as an adapter over those parser-owned outputs

## Migration Bias

Do not optimize for file-count balance.

Optimize for these outcomes:

1. lightweight consumers stop pulling `xiuxian-wendao` for syntax-only parsing
2. Wendao keeps business semantics and document-to-domain projection ownership
3. Markdown and future Org support converge on one independent parser lane
4. no long-lived compatibility shim leaves the same parser API owned by two
   crates

:RELATIONS:
:LINKS: [[02_parser/architecture]], [[06_roadmap/417_wendao_package_boundary_matrix]], [[06_roadmap/405_large_rust_modularization]]
:END:

---

:FOOTER:
:LAST_SYNC: 2026-04-10
:END:
