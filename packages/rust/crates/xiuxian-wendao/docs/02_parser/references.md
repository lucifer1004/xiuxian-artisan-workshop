# Parser References

:PROPERTIES:
:ID: wendao-parser-references
:PARENT: [[02_parser/index]]
:TAGS: parser, references, markdown
:STATUS: ACTIVE
:END:

## Objective

Wendao keeps ordinary Markdown reference parsing under
`src/parsers/markdown/references/` so `SKILL.md` and ordinary Markdown
documents do not diverge into different link grammars.

## Syntax Contract

The canonical parser preserves ordinary Markdown references in these shapes:

1. `[label](note/path.md)`
2. `[label](note/path.md#Heading)`
3. `[label](#Local Heading)`
4. `[[note]]`
5. `[[note#Heading]]`
6. `[[#Local Heading]]`

The parser separates an optional cross-document target from an optional
structural address for both syntaxes.

## Extraction Rules

The implementation is comrak-backed and parser-owned:

1. comrak parses ordinary Markdown links and wikilinks in one traversal
2. source spans are converted back into exact byte slices so the parser keeps
   the original literal
3. references are returned in document order
4. images such as `![label](asset.png)` are excluded
5. embedded wikilinks such as `![[note]]` are excluded

## Consumer Boundary

`skill_vfs::internal_manifest::authority` consumes this parser surface:

1. `SKILL.md` manifest intents now use the same parser-owned Markdown
   reference contract as ordinary Markdown documents
2. the authority consumer reduces parser output into local manifest-path
   normalization and URI authority checks
3. `SKILL.md` no longer owns a local split between Markdown-link scanning and
   wikilink parsing

The narrower `wikilinks.md` surface documents the Obsidian-only subset used by
consumers that only care about ordinary `[[...]]` topology links.

## Regression Coverage

Coverage for this contract lives in:

1. `tests/unit/parsers/markdown/references.rs`
2. `tests/snapshots/parser/markdown/references.json`
3. `src/skill_vfs/internal_manifest/tests.rs`

:RELATIONS:
:LINKS: [[02_parser/index]], [[02_parser/architecture]], [[02_parser/wikilinks]], [[01_core/103_package_layering]]
:END:

---

:FOOTER:
:LAST_SYNC: 2026-04-05
:END:
