# Parser Document Metadata

:PROPERTIES:
:ID: wendao-parser-document
:PARENT: [[02_parser/index]]
:TAGS: parser, document, markdown
:STATUS: ACTIVE
:END:

## Objective

Wendao now treats one cross-format document core plus one Markdown-specific
document wrapper as parser-owned shared surfaces in
`xiuxian-wendao-parsers`, while `xiuxian-wendao` keeps only the domain adapter
that assembles `LinkGraphDocument`.

## Contract

The canonical parser-owned document contracts now split into three layers:

1. `DocumentCore`
   - document format family
   - format-normalized body with top-level metadata stripped
   - best-effort title
   - best-effort tags
   - optional semantic document type
   - best-effort leading content snippet
   - best-effort body word count
2. `DocumentEnvelope<RawMetadata>`
   - one optional raw metadata payload owned by the source format
   - one embedded `DocumentCore`
3. `MarkdownDocument`
   - Markdown-local alias over `DocumentEnvelope<serde_yaml::Value>`
   - preserves raw YAML frontmatter when the document starts with a valid
     frontmatter block

`DocumentCore` is the reusable cross-format metadata and body contract.
`DocumentEnvelope<RawMetadata>` is the reusable cross-format top-level wrapper
shape for `raw metadata + document core`. `MarkdownDocument` is the
Markdown-local naming surface that keeps raw YAML metadata available for
current Wendao adapters. None of these contracts include path identity,
filesystem timestamps, saliency defaults, or graph records.

## Extraction Rules

The shared extractor follows these rules for Markdown:

1. frontmatter is split before metadata extraction
2. title prefers frontmatter `title`, then the first Markdown `# ` heading,
   then the caller-provided fallback
3. tags follow the historical note-parser contract and only read top-level
   frontmatter `tags`
4. `type` and `kind` are normalized into one optional `doc_type`
5. lead text skips blank lines, headings, and code-fence markers
6. `DocumentCore.format` is set to `markdown`

## Consumer Boundary

`xiuxian-wendao` now consumes these parser-owned document contracts:

1. `parse_note` consumes `MarkdownDocument.core` through the parser-owned
   `MarkdownNote` aggregate for title, tags, doc type, lead, body, and word
   count
2. Wendao still consumes `MarkdownDocument.raw_metadata` for saliency and
   timestamp adapters that are still Markdown-specific today
3. Wendao still owns `doc_id`, `path`, timestamps, saliency defaults, and
   `LinkGraphDocument` assembly
4. link extraction and section enrichment still happen in Wendao because they
   require workspace-aware and domain-aware adapters

## Regression Coverage

Coverage for this contract lives in:

1. `packages/rust/crates/xiuxian-wendao-parsers/tests/unit/document.rs`
2. `tests/unit/parsers/markdown/document.rs`
3. `tests/unit/parsers/markdown/namespace.rs`

:RELATIONS:
:LINKS: [[02_parser/index]], [[02_parser/architecture]], [[02_parser/note]], [[02_parser/sections]], [[06_roadmap/419_parser_substrate_separation]]
:END:

---

:FOOTER:
:LAST_SYNC: 2026-04-11
:END:
