# Parser Note Aggregate

:PROPERTIES:
:ID: wendao-parser-note
:PARENT: [[02_parser/index]]
:TAGS: parser, note, markdown
:STATUS: ACTIVE
:END:

## Objective

Wendao now treats one cross-format note core plus one cross-format top-level
note aggregate as parser-owned shared surfaces in
`xiuxian-wendao-parsers`, while `xiuxian-wendao` keeps the workspace-aware
adapter that resolves links and assembles `LinkGraphDocument`.

## Contract

The canonical parser-owned note contracts now split into three layers:

1. `NoteCore<Reference, Target, Section>`
   - format-owned references in document order
   - format-owned note-level target occurrences in document order
   - format-owned section structure in body order
2. `NoteAggregate<Document, Reference, Target, Section>`
   - one parser-owned format document wrapper
   - one embedded `NoteCore<Reference, Target, Section>`
3. `MarkdownNote`
   - Markdown-local alias over
     `NoteAggregate<MarkdownDocument, MarkdownReference, MarkdownTargetOccurrence, MarkdownSection>`
   - `MarkdownTargetOccurrence` remains the Markdown-local naming surface over
     `TargetOccurrenceCore<MarkdownTargetOccurrenceKind>`

`NoteCore` is the reusable cross-format note-body aggregation shape.
`NoteAggregate<Document, Reference, Target, Section>` is the reusable
cross-format top-level note aggregate shape.
`MarkdownNote` is the Markdown-specific naming surface that keeps
`DocumentEnvelope<serde_yaml::Value>` plus Markdown-owned item contracts
available. None of these contracts include path identity,
attachment classification,
workspace-aware link normalization,
timestamps, or graph records.

## Extraction Rules

The shared aggregate follows these rules for Markdown:

1. `parse_markdown_note` first parses `MarkdownDocument`
2. reference and section extraction run against `MarkdownDocument.core.body`
3. target-occurrence extraction runs against that same parser-owned body
4. the resulting `NoteAggregate<...>` preserves parser-owned raw targets
   plus parser-visible occurrence ranges without filesystem context
5. sections preserve parser-owned heading scope plus shared `SectionMetadata`
   without Wendao enrichments

## Consumer Boundary

`xiuxian-wendao` now consumes this parser-owned note contract:

1. `parse_note` uses `MarkdownNote` as the parser-owned aggregate entry point
2. Wendao consumes `MarkdownDocument.core` for reusable document metadata and
   `MarkdownDocument.raw_metadata` for the current Markdown-specific saliency
   and timestamp adapters
3. Wendao consumes `MarkdownNote.core` for reusable note-body aggregation
   shape while keeping Markdown-specific item types intact
4. Wendao still owns `doc_id`, `path`, timestamps, saliency defaults, and
   `LinkGraphDocument` assembly
5. workspace-aware note-link and attachment resolution still happen in Wendao
6. Wendao still enriches parser-owned sections into `ParsedSection` by adding
   note-link entities and `CodeObservation` rows
7. section entity enrichment now partitions parser-owned note-level target
   occurrences by section byte range before normalization

## Regression Coverage

Coverage for this contract lives in:

1. `packages/rust/crates/xiuxian-wendao-parsers/tests/unit/note.rs`
2. `tests/unit/parsers/markdown/document.rs`
3. `tests/unit/parsers/markdown/namespace.rs`
4. `tests/unit/workflow_demo.rs`

:RELATIONS:
:LINKS: [[02_parser/index]], [[02_parser/architecture]], [[02_parser/document]], [[02_parser/targets]], [[02_parser/sections]], [[06_roadmap/419_parser_substrate_separation]]
:END:

---

:FOOTER:
:LAST_SYNC: 2026-04-12
:END:
