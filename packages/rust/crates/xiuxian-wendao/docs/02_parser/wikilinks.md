# Parser Wikilinks

:PROPERTIES:
:ID: wendao-parser-wikilinks
:PARENT: [[02_parser/index]]
:TAGS: parser, wikilinks, markdown
:STATUS: ACTIVE
:END:

## Objective

Wendao now treats ordinary body wikilink parsing as the narrower
Obsidian-style subset of the shared parser-owned reference surface in
`xiuxian-wendao-parsers`, backed by the same parser-owned addressed-target
core that ordinary Markdown references use.

## Syntax Contract

The canonical parser preserves ordinary body wikilinks in these shapes:

1. `[[note]]`
2. `[[note#Heading]]`
3. `[[note#^block-id]]`
4. `[[#Local Heading]]`
5. `[[note|Alias]]`

The parser treats `#...` as a structural address, never as a semantic type
suffix.

## Extraction Rules

The implementation is comrak-backed and derived from the shared reference
parser, not regex-driven:

1. the shared reference parser walks Markdown links and wikilinks in one
   parser-owned traversal
2. the wikilink surface filters that shared output down to ordinary
   `[[...]]` references only
3. source spans are converted back into exact byte slices so the parser keeps
   the original literal, including aliases
4. ordinary body wikilinks are returned in document order
5. embedded forms such as `![[note]]` are excluded from this ordinary
   body-link surface

This gives Wendao one parser-owned structural interpretation for body links
before any consumer-specific reduction happens.

`MarkdownWikiLink` is now the Markdown-local naming surface for
`LiteralAddressedTarget`. That means the note target plus optional scoped
address come from the shared `AddressedTarget` contract, while the original
literal comes from the shared source-preserved literal wrapper.

## Consumer Boundary

`link_graph_refs` is now a consumer over this parser surface:

1. it filters out local-only body addresses because `LinkGraphEntityRef`
   requires a cross-note target name
2. it keeps its historical deduplication behavior for `LinkGraph` consumers
3. it no longer owns its own regex-based wikilink grammar

`docs_governance` also consumes this parser surface for ordinary `:LINKS:`
and index-body wikilink collection:

1. relation-line and index-body checks now reduce canonical parser output
   instead of re-owning a local wikilink scanner
2. hidden-path governance still keeps its own line/offset helper because that
   adapter-local contract needs byte ranges rather than just wikilink targets

`skill_vfs::internal_manifest::authority` no longer consumes this narrower
surface directly. It now consumes the shared `references` parser so `SKILL.md`
ordinary Markdown links and ordinary wikilinks follow one parser-owned
contract.

## Semantic Boundary

Ordinary body wikilinks only establish structure:

1. note-to-note topology
2. note-to-heading or note-to-block addressing
3. local address visibility

Typed semantics still belong to explicit metadata owners such as property
drawers, section tags, or other subsystem-owned metadata.

## Regression Coverage

Coverage for this contract lives in:

1. `packages/rust/crates/xiuxian-wendao-parsers/tests/unit/wikilinks.rs`
2. `tests/unit/parsers/markdown/wikilinks.rs`
3. `tests/snapshots/parser/markdown/wikilinks.json`
4. `tests/unit/link_graph_refs.rs`
5. `src/zhenfa_router/native/semantic_check/docs_governance/tests/index_links/relations.rs`

:RELATIONS:
:LINKS: [[02_parser/index]], [[02_parser/addressed_target]], [[02_parser/references]], [[02_parser/architecture]], [[02_parser/relation_semantics]], [[01_core/103_package_layering]]
:END:

---

:FOOTER:
:LAST_SYNC: 2026-04-11
:END:
