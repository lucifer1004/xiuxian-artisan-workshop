# Parser Architecture

:PROPERTIES:
:ID: wendao-parser-architecture
:PARENT: [[02_parser/index]]
:TAGS: parser, architecture, implementation
:STATUS: ACTIVE
:END:

## Objective

`xiuxian-wendao` keeps Wendao-owned parser adapters under the single crate-root
namespace `src/parsers/`, while reusable parser-owned syntax, block, target,
and note-aggregate contracts may move to `xiuxian-wendao-parsers` once they
are cleanly separated from Wendao domain records.

## Canonical Parser Families

| Namespace                                             | Input shape                        | Canonical output                           | Notes                                                                                                                                                                                                                                                                                                             |
| ----------------------------------------------------- | ---------------------------------- | ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `parsers::markdown`                                   | Markdown notes                     | sections, code observations, note adapters | Shared by indexing, search, enhancement, and semantic checks; frontmatter, block extraction, document metadata, target occurrences, note aggregation, references, wikilinks, sourcepos, and parser-owned section structure now live in `xiuxian-wendao-parsers`, while Wendao keeps enrichments and note adapters |
| `parsers::link_graph::query`                          | link-graph search query strings    | `ParsedLinkGraphQuery`                     | Shared query-language parsing                                                                                                                                                                                                                                                                                     |
| `parsers::zhixing::tasks`                             | zhixing task lines                 | task projections and normalized identities | Shared by ingest and stats                                                                                                                                                                                                                                                                                        |
| `parsers::languages::rust::cargo::dependencies`       | `Cargo.toml` dependency tables     | dependency projections                     | Shared by dependency indexing                                                                                                                                                                                                                                                                                     |
| `parsers::languages::python::pyproject::dependencies` | `pyproject.toml` dependency tables | dependency projections                     | Shared by dependency indexing                                                                                                                                                                                                                                                                                     |
| `parsers::search::repo_code_query`                    | repo-code search query strings     | typed repo-code query                      | Shared by repo-search flows                                                                                                                                                                                                                                                                                       |
| `parsers::graph::persistence`                         | graph JSON dicts                   | `Entity` and `Relation`                    | Shared by graph save/load persistence                                                                                                                                                                                                                                                                             |

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

## Cross-Crate Reuse Rule

`xiuxian-wendao` is still the ownership home for Wendao domain parser
adapters, but not every parser family under `src/parsers/` should stay
Wendao-owned forever.

When a parser family becomes reusable across packages such as `xiuxian-qianji`
or future `xiuxian-qianhuan` document flows, the long-term extraction target is
an independent parser crate, tentatively `xiuxian-wendao-parsers`, rather than
another consumer-local helper tree.

That extraction is no longer theoretical: the parser-owned frontmatter
contract, raw frontmatter splitter, cross-format addressed-target core,
cross-format literal-addressed-target core, cross-format reference core,
cross-format document core, cross-format document-envelope core, cross-format
note core, cross-format note-aggregate core, shared target-occurrence core
with Markdown naming surface, shared block core with Markdown block naming
surface, Markdown reference grammar, Markdown wikilink grammar, shared
source-position helper, shared full section-core contract with its nested
section-scope core, and the parser-owned Markdown naming surfaces already live in
`xiuxian-wendao-parsers`, while `xiuxian-wendao` keeps only Wendao-owned
adapters and domain-side consumption.

A parser surface is a direct parser-crate candidate only when all of the
following are true:

1. the input is a durable document-format grammar such as Markdown or Org
2. the output can be expressed as parser-owned intermediate contracts without
   Wendao-owned domain records such as
   `LinkGraphDocument`, `LinkGraphSearchOptions`, `Entity`, `Relation`, or
   `WendaoResourceUri`
3. at least one non-Wendao package can consume the result directly

If the parser surface builds Wendao graph, retrieval, persistence, or other
business semantics, it stays in `xiuxian-wendao` and should be treated as a
domain adapter over any future independent parser crate.

Org remains a placeholder architecture target in this lane. The current parser
substrate is shaped so an Org slice can reuse the shared contracts later, but
no active Org parser implementation is implied until a direct consumer or
explicit slice reopens that work.

See [../06_roadmap/419_parser_substrate_separation.md](../06_roadmap/419_parser_substrate_separation.md)
for the package-split plan.

## Block Contract Boundary

Markdown block extraction is now split across five explicit contracts:

1. `xiuxian_wendao_parsers::blocks::BlockCore<Kind>` owns one reusable block
   payload shape for block identity, ranges, content hash, raw content,
   optional explicit ID, and structural path
2. `xiuxian_wendao_parsers::blocks::MarkdownBlockKind` owns the Markdown-local
   block variants
3. `xiuxian_wendao_parsers::blocks::MarkdownBlock` is the Markdown-local
   naming surface over `BlockCore<MarkdownBlockKind>`
4. `xiuxian_wendao_parsers::blocks::extract_blocks` is the shared parser-owned
   entry point for block extraction from one section body
5. `xiuxian_wendao::link_graph::BlockAddress` and
   `xiuxian_wendao::link_graph::BlockKindSpecifier` remain Wendao-owned
   because they encode semantic addressing grammar, not Markdown parsing
6. Wendao page-index building consumes parser-owned Markdown blocks directly
7. block-to-address matching stays Wendao-owned as a domain helper layered on
   top of parser-owned block payloads

## Section Contract Boundary

Markdown section extraction is now split across five explicit contracts:

1. `xiuxian_wendao_parsers::sections::SectionCore` owns shared normalized
   section text plus one nested `SectionScope` and one nested
   `SectionMetadata`
2. `xiuxian_wendao_parsers::sections::SectionMetadata` owns shared
   property-drawer attributes and logbook entries reusable across formats
3. `xiuxian_wendao_parsers::sections::SectionScope` stays the nested shared
   heading-ancestry and source-range contract inside `SectionCore`
4. `xiuxian_wendao_parsers::sections::MarkdownSection` is the Markdown-local
   naming surface over `SectionCore`
5. `xiuxian_wendao::parsers::markdown::ParsedSection` is an enriched adapter
   that adds Wendao-owned `entities` and `CodeObservation` rows
6. property-relation parsing can consume the parser-owned section contract
   because it only needs heading scope and parser-owned metadata attributes
7. note parsing that assembles `LinkGraphDocument` remains Wendao-owned

## Document Contract Boundary

Markdown document-content parsing is now split across four explicit contracts:

1. `xiuxian_wendao_parsers::document::DocumentCore` owns cross-format
   document format, normalized body, title, tags, doc type, lead, and word
   count
2. `xiuxian_wendao_parsers::document::DocumentEnvelope<RawMetadata>` owns one
   shared top-level `raw metadata + document core` contract reusable across
   formats
3. `xiuxian_wendao_parsers::document::MarkdownDocument` is the Markdown-local
   alias over `DocumentEnvelope<serde_yaml::Value>`
4. `xiuxian_wendao::parsers::markdown::parse_note` is the Wendao adapter that
   adds `doc_id`, path identity, timestamps, saliency defaults, links,
   sections, and `LinkGraphDocument` assembly
5. this keeps content-owned parsing reusable without moving graph or retrieval
   semantics into the parser crate

## Note Aggregate Boundary

Markdown note parsing is now split across four explicit contracts:

1. `xiuxian_wendao_parsers::note::NoteCore<Reference, Target, Section>` owns
   one reusable note-body aggregation shape for ordered references, targets,
   and sections
2. `xiuxian_wendao_parsers::note::NoteAggregate<Document, Reference, Target, Section>`
   owns one reusable top-level `document + note-core` aggregate shape
3. `xiuxian_wendao_parsers::note::MarkdownNote` is the Markdown-local alias
   over `NoteAggregate<MarkdownDocument, MarkdownReference, MarkdownTargetOccurrence, MarkdownSection>`
4. `xiuxian_wendao_parsers::note::parse_markdown_note` is the shared
   parser-owned entry point for Markdown note aggregation
5. `xiuxian_wendao::parsers::markdown::parse_note` is the Wendao adapter that
   consumes `MarkdownDocument.core` for reusable document metadata, consumes
   `MarkdownDocument.raw_metadata` for current Markdown-specific adapters,
   consumes `MarkdownNote.core` for reusable note-body aggregation, and adds
   workspace-aware link normalization, attachment classification, enriched
   sections, and final `LinkGraphDocument` assembly
6. this keeps parser orchestration reusable without moving filesystem or graph
   semantics into the parser crate

## Addressed Target and Reference Boundary

Markdown ordinary body links now split across five explicit contracts:

1. `xiuxian_wendao_parsers::AddressedTarget` owns one reusable parser-owned
   `target + target_address` contract for cross-format structural link
   coordinates
2. `xiuxian_wendao_parsers::LiteralAddressedTarget` owns one reusable
   parser-owned `AddressedTarget + original literal` contract for
   source-preserved link items
3. `xiuxian_wendao_parsers::ReferenceCore<Kind>` owns one reusable
   parser-owned `kind + LiteralAddressedTarget` contract for
   source-preserved reference items that still carry one format-local kind tag
4. `xiuxian_wendao_parsers::references::MarkdownReference` is the
   Markdown-local alias over `ReferenceCore<MarkdownReferenceKind>`
5. `xiuxian_wendao_parsers::wikilinks::MarkdownWikiLink` is the
   Markdown-local naming surface over `LiteralAddressedTarget`
6. Wendao consumers such as `link_graph_refs` and `skill_vfs` reduce this
   parser-owned core into their own domain-specific adapters
7. Wendao-owned relation targets still use `Address` and are not part of this
   parser-owned addressed-target and reference contract

## Target Occurrence Boundary

Markdown note-level target capture is now split across two explicit contracts:

1. `xiuxian_wendao_parsers::targets::TargetOccurrenceCore<Kind>` owns the
   shared parser-visible `kind + target + source ranges` occurrence payload
   reusable across formats
2. `xiuxian_wendao_parsers::targets::MarkdownTargetOccurrence` is the
   Markdown-local naming surface over that shared core
3. `xiuxian_wendao_parsers::targets::extract_targets` is the shared
   parser-owned entry point for note-level target capture
4. `xiuxian_wendao::parsers::markdown::extract_link_targets_from_occurrences`
   is the Wendao adapter that applies workspace-aware normalization and
   attachment classification
5. Wendao section enrichment now filters note-level parser occurrences by
   section byte range before normalization, instead of re-running a second
   Markdown syntax pass per section
6. embedded wikilinks remain ignored on the current comrak-backed target path,
   matching the existing Wendao note-level behavior

## Parsing Strategy

Parser implementations should prefer structural signals over loose text
matching:

1. explicit fields, structured delimiters, and graph-visible links come first
2. ordinary wiki links create graph topology first; semantic upgrades come
   later and only from explicit metadata owners
3. Obsidian-style wiki-link fragments such as `#Heading` or `#^block-id`
   should be treated as real target addresses, not semantic type suffixes
4. file suffix or owned path conventions may classify resources such as
   attachments without introducing link-token string matches
5. heuristic or path-based fallbacks should stay bounded and local
6. keyword-only matching should not become the primary contract when a
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

For ordinary body links, Wendao follows one parser-owned Markdown reference
contract:

1. `[label](note/path.md)` means a Markdown reference target
2. `[label](note/path.md#Heading)` means a Markdown reference plus structural
   address
3. `[label](#Local Heading)` means a local same-note structural address
4. `[[note]]` means a wiki-link note target
5. `[[note#Heading]]` means a wiki-link note plus heading target
6. `[[note#^block-id]]` means a wiki-link note plus block target
7. `[[#Local Heading]]` means a local same-note structural address

These address fragments are structural coordinates, not semantic type tags.

The canonical implementation for ordinary Markdown references now lives in
`xiuxian-wendao-parsers` and uses comrak AST parsing plus source-span
reconstruction, so ordinary Markdown reference parsing is not owned by
consumer-local scanners or by the Wendao domain crate itself.

The narrower wikilink-only subset is also exposed from
`xiuxian-wendao-parsers` for consumers that only care about ordinary
Obsidian-style topology links, while `xiuxian-wendao` keeps compatibility
re-exports for existing internal consumers.

Typed relation semantics belong to explicit metadata surfaces, such as
property drawers or subsystem-owned metadata, not to hardcoded string matches
inside parser helpers.

## Property Drawer Scope Rule

Property drawers are the explicit metadata surface for section-scoped relation
semantics.

This means Wendao distinguishes three different parser contracts:

1. ordinary global `[[...]]` links in note content:
   topology, backlinks, and structural adjacency
2. property-drawer relation values:
   explicit typed relations scoped to the owning heading or section
3. property-drawer scalar values:
   local metadata such as limits, weights, policy tags, or scope markers that
   do not create graph edges by default

Inside a property drawer, Wendao uses an explicit target grammar so a value
such as `[[file-b#section-2]]` means a scoped relation target rather than the
ordinary body-link interpretation of `#...`.

Stable cross-document section relations should prefer explicit `:ID:` anchors.
Path- and hash-scoped targets are still preserved by the parser, but the
current graph adapter only resolves the safe subset that can be mapped without
guessing.

The shared property-drawer and logbook extraction now live in
`xiuxian-wendao-parsers`, so Wendao relation and indexing flows consume one
parser-owned section metadata contract before adding domain semantics.

See [relation_semantics.md](relation_semantics.md) for the detailed contract.

## Persistence Rule

Graph persistence parsers may decode exact internal enum tokens written by
Wendao itself, but they must not reinterpret arbitrary wiki-link-shaped
strings as known semantic relation types. Unknown labels are preserved rather
than promoted.

:RELATIONS:
:LINKS: [[02_parser/index]], [[02_parser/addressed_target]], [[02_parser/document]], [[02_parser/note]], [[02_parser/targets]], [[02_parser/sections]], [[02_parser/references]], [[02_parser/wikilinks]], [[02_parser/relation_semantics]], [[01_core/103_package_layering]], [[03_features/210_search_queries_architecture]], [[06_roadmap/405_large_rust_modularization]], [[06_roadmap/419_parser_substrate_separation]]
:END:

---

:FOOTER:
:LAST_SYNC: 2026-04-12
:END:
