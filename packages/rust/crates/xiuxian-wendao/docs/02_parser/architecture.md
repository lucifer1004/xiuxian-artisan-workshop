# Parser Architecture

:PROPERTIES:
:ID: wendao-parser-architecture
:PARENT: [[02_parser/index]]
:TAGS: parser, architecture, implementation
:STATUS: ACTIVE
:END:

## Objective

`xiuxian-wendao` keeps domain-core parser behavior under a single crate-root
namespace, `src/parsers/`, so parser ownership reflects durable input and
output contracts rather than whichever subsystem first consumed them.

## Canonical Parser Families

| Namespace                          | Input shape                     | Canonical output                                | Notes                                                        |
| ---------------------------------- | ------------------------------- | ----------------------------------------------- | ------------------------------------------------------------ |
| `parsers::markdown`                | Markdown notes                  | frontmatter, sections, links, code observations | Shared by indexing, search, enhancement, and semantic checks |
| `parsers::link_graph::query`       | link-graph search query strings | `ParsedLinkGraphQuery`                          | Shared query-language parsing                                |
| `parsers::zhixing::tasks`          | zhixing task lines              | task projections and normalized identities      | Shared by ingest and stats                                   |
| `parsers::cargo::dependencies`     | `Cargo.toml` dependency tables  | dependency projections                          | Shared by dependency indexing                                |
| `parsers::search::repo_code_query` | repo-code search query strings  | typed repo-code query                           | Shared by repo-search flows                                  |
| `parsers::graph::persistence`      | graph JSON dicts                | `Entity` and `Relation`                         | Shared by graph save/load persistence                        |

## Parser vs Local Helper Rule

Code belongs under `src/parsers/` when all of the following are true:

1. it parses a durable external or cross-subsystem input surface
2. it returns a canonical typed output reused by multiple consumers
3. the parsing semantics are domain-core, not tied to one adapter DTO
4. parser-owned unit coverage can live under `tests/unit/parsers/`

Code stays outside `src/parsers/` when it is one of these:

1. adapter-local request parsing, such as `search/queries/graphql/document.rs`
2. gateway-local validation and DTO decoding, such as
   `gateway/studio/router/handlers/repo/parse.rs`
3. subsystem-local config or payload decode helpers, such as
   `analyzers/config/parse.rs`,
   `search_plane/repo_entity/query/hydrate/parse.rs`, and
   `pybindings/link_graph_py/engine/refresh/parse.rs`
4. query models or execution modules, such as `entity/query.rs` and
   `storage/query.rs`

## Implementation Rules

1. `mod.rs` is interface-only and should re-export leaf modules.
2. Medium or complex parser work should use feature folders.
3. Direct migration is preferred over compatibility shims.
4. Parser-owned unit coverage should live under `tests/unit/parsers/<family>/`.
5. Consumer subsystems may import parser services, but they do not own
   duplicate parser namespaces.

## Parsing Strategy

Parser implementations should prefer structural signals over loose text
matching:

1. explicit fields, structured delimiters, and graph-visible links come first
2. ordinary wiki links create graph topology first; semantic upgrades come
   later and only from explicit metadata owners
3. file suffix or owned path conventions may classify resources such as
   attachments without introducing link-token string matches
4. heuristic or path-based fallbacks should stay bounded and local
5. keyword-only matching should not become the primary contract when a
   structural signal already exists

## Structural Relation Rule

When Wendao parses `[[...]]` links across the workspace, the first parser job
is to establish graph connectivity:

1. outbound wiki links define structural edges
2. reverse edges or backlinks are graph facts derived from the same link set
3. plain link text does not automatically become a semantic relation label

This means a link such as `[[notes/design]]` or `[[assets/logo.png]]` is first
handled as graph structure. If Wendao later needs to know that a target is an
attachment, that classification should come from an explicit structural signal
such as the file suffix, not from a special relation index note or a
hardcoded link label.

Typed relation semantics belong to explicit metadata surfaces, such as
property drawers or subsystem-owned metadata, not to hardcoded string matches
inside parser helpers.

## Persistence Rule

Graph persistence parsers may decode exact internal enum tokens written by
Wendao itself, but they must not reinterpret arbitrary wiki-link-shaped
strings as known semantic relation types. Unknown labels are preserved rather
than promoted.

:RELATIONS:
:LINKS: [[02_parser/index]], [[01_core/103_package_layering]], [[03_features/210_search_queries_architecture]], [[06_roadmap/405_large_rust_modularization]]
:END:

---

:FOOTER:
:LAST_SYNC: 2026-04-05
:END:
