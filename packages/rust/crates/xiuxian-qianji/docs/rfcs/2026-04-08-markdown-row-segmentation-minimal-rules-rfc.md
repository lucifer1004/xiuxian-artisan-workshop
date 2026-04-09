---
type: knowledge
title: "RFC 0006: Markdown Row Segmentation Minimal Rules for Bounded Plan Work"
category: "rfc"
status: "draft"
authors:
  - codex
created: 2026-04-08
tags:
  - rfc
  - qianji
  - wendao
  - markdown
  - retrieval
  - segmentation
---

# RFC 0006: Markdown Row Segmentation Minimal Rules for Bounded Plan Work

## 1. Summary

This RFC defines the minimal row-segmentation rules for the `markdown`
retrieval surface used by bounded plan work.

Version one needs only three things:

1. a stable answer to what one `markdown` row represents
2. a stable answer to when a row should be split
3. a small rejection set that prevents over-fragmentation or unstable slicing

The goal is to keep retrieval rows stable enough for Codex to reason about
them across repeated `show -> sql -> check` loops.

## 2. Background

RFC 0003 defines the minimal SQL retrieval surface and the `markdown` table.

RFC 0005 defines what `skeleton` must preserve and how Codex should prefer
`skeleton` before `body`.

What still needs to be fixed is the row model itself:

> when Wendao returns one row from `markdown`, what exact structural unit does
> that row represent?

Without a stable answer, retrieval can drift into either:

1. rows that are too large and lose boundedness
2. rows that are too small and destroy structural meaning

## 3. Non-Goals

This RFC does not define:

1. AST-internal storage design
2. indexing or persistence layout
3. ranking or reranking strategy
4. SQL optimizer behavior
5. code-block-specific segmentation rules

Version one only defines the external row-segmentation contract.

## 4. Row Model

Version one defines one `markdown` row as:

> one retrievable markdown structural unit with a stable heading context and a
> stable `skeleton` / `body` rendering.

In practice, a row may correspond to:

1. a document root
2. a heading subtree
3. a section boundary that is stable enough to retrieve independently

The external rule is stability, not one hardcoded parser-internal shape.

## 5. Minimum Stable Split Rules

Version one should follow four rules.

### 5.1 Document Root May Exist

A document root row is allowed.

This gives Codex a stable top-level entry when a file needs to be addressed as
a whole.

### 5.2 Heading Boundaries Are Primary Split Points

Headings are the default and preferred split boundaries.

If a markdown file has stable heading structure, segmentation should follow
that structure before inventing finer-grained boundaries.

### 5.3 Child Sections Must Keep Parent Context

When a row represents a subsection, it must still preserve enough parent
context for `heading_path` and `skeleton` to remain meaningful.

That is why path-like context such as:

```text
Blueprint/Boundary
Plan/Rust
```

is part of the retrieval surface.

### 5.4 Independent Rows Must Still Be Structurally Legible

A row must not be split so finely that its `skeleton` loses structural meaning.

If a fragment cannot stand on its own as a recognizable structural unit, it is
too small to become its own version-one row.

## 6. What Version One Should Prefer

Version one should prefer:

1. one root row per markdown file when useful
2. one row per stable heading subtree
3. splits that preserve heading path and checklist shape
4. predictable repeated retrieval over maximal compression

This keeps the retrieval surface simple and inspectable.

## 7. What Version One Should Avoid

Version one should avoid:

1. splitting every bullet into its own row
2. splitting every paragraph into its own row
3. splitting in ways that discard heading ancestry
4. splitting differently on every re-index when the source shape is unchanged

These behaviors make Codex retrieval noisy and unstable.

## 8. Relation to `heading_path`

`heading_path` is the main external signal that row segmentation remained
stable.

If a row has:

```text
Blueprint/Boundary
```

then Codex should be able to infer:

1. this unit belongs under `Blueprint`
2. this unit is specifically the `Boundary` branch
3. retrieving this row preserves useful local context without requiring the
   whole file

## 9. Relation to `skeleton`

`skeleton` depends on row segmentation staying structurally meaningful.

If rows are too coarse:

1. `skeleton` becomes too large
2. token cost rises

If rows are too fine:

1. `skeleton` loses plan shape
2. Codex has to reconstruct context manually

That is why row segmentation and `skeleton` rules must stay aligned.

## 10. Minimal Examples

### 10.1 Document With Headings

Source:

```markdown
# Blueprint

## Boundary

- [ ] check ownership

## Interfaces

Long prose...
```

A version-one segmentation may expose:

1. a root row for `Blueprint`
2. a row for `Blueprint/Boundary`
3. a row for `Blueprint/Interfaces`

### 10.2 Over-Segmentation To Avoid

This is not a preferred version-one segmentation:

1. one row for `- [ ] check ownership`
2. one row for `Long prose...`

Those fragments are too small and unstable on their own.

## 11. Codex Retrieval Guidance

Codex should keep using the same bounded order:

```text
1. qianji show --dir <plan-workdir>
2. inspect filenames and directory shape
3. use tree only if deeper structure inspection is needed
4. use wendao sql to retrieve stable markdown rows
5. read skeleton before body
6. qianji check --dir <plan-workdir>
```

The row model exists to make step 4 stable and predictable.

## 12. Rejections

This RFC explicitly rejects:

1. paragraph-by-paragraph default segmentation
2. bullet-by-bullet default segmentation
3. segmentation that drops heading ancestry
4. unstable segmentation for unchanged source

## 13. Conclusion

Version one row segmentation should stay simple:

1. prefer heading-based structural units
2. preserve parent context through `heading_path`
3. allow root rows where useful
4. avoid over-fragmentation

That is enough to support stable SQL retrieval for bounded Codex work.
