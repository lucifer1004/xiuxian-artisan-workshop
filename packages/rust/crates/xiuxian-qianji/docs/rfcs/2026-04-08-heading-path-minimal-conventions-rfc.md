---
type: knowledge
title: "RFC 0008: Heading Path Minimal Conventions for Bounded Plan Work"
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
  - heading-path
---

# RFC 0008: Heading Path Minimal Conventions for Bounded Plan Work

## 1. Summary

This RFC defines the minimum external conventions for `markdown.heading_path`
in bounded plan work.

Version one only needs four things:

1. a stable separator
2. a stable rule for root rows
3. a stable relation between `heading_path` and `title`
4. a small rejection set that prevents ambiguous or noisy paths

The goal is to keep `heading_path` human-readable, SQL-friendly, and stable
across repeated `show -> sql -> check` loops.

## 2. Background

RFC 0003 defines `heading_path` as part of the minimal `markdown` SQL surface.

RFC 0006 defines one `markdown` row as a stable structural unit and makes
heading boundaries the preferred split points.

What still needs to be fixed is the external convention itself:

> when Wendao exposes `heading_path`, what exact string shape should Codex
> expect?

Without that answer, retrieval can drift into ad hoc path rendering.

## 3. Non-Goals

This RFC does not define:

1. internal parser AST shape
2. markdown parsing algorithms
3. file-level indexing layout
4. slug-generation rules
5. automatic disambiguation for every pathological heading shape

Version one defines only the smallest stable external convention.

## 4. Core Convention

`heading_path` is the human-readable heading ancestry for one retrievable
markdown structural unit.

It is:

1. heading-based
2. ordered from ancestor to descendant
3. separated by `/`
4. independent from the filesystem `path` column

It is not:

1. a file path
2. a slug
3. a summary
4. a parser-internal identifier

## 5. Minimum Formatting Rules

### 5.1 Separator

Version one uses `/` as the only path separator.

Examples:

```text
Blueprint
Blueprint/Boundary
Plan
Plan/Rust
```

### 5.2 Segment Source

Each segment must come from the visible heading text of one markdown heading
in the ancestor chain.

Version one should use the trimmed heading text itself instead of inventing a
slug or opaque identifier.

### 5.3 Ordering

Segments must appear from outermost ancestor to innermost heading.

So if a row belongs to:

1. `# Blueprint`
2. `## Boundary`

then the path is:

```text
Blueprint/Boundary
```

not:

```text
Boundary/Blueprint
```

### 5.4 Root Rows

Document-root rows are allowed.

For version one, a root row should use:

```text
heading_path = ""
```

This keeps root rows distinguishable from heading-scoped rows without adding a
special keyword.

### 5.5 No Leading or Trailing Separator

Version one must not emit:

```text
/Blueprint
Blueprint/
/Blueprint/Boundary/
```

The separator belongs only between segments.

## 6. Relation to `title`

`title` is the local title of the current structural unit.

`heading_path` is the full visible ancestry of that unit.

For a row with:

```text
heading_path = "Blueprint/Boundary"
title = "Boundary"
```

Codex should be able to infer:

1. the local heading is `Boundary`
2. its parent context is `Blueprint`

For root rows, `title` may be empty or use another stable root marker, but
`heading_path` should remain the empty string.

## 7. Relation to `path`

`path` and `heading_path` serve different purposes.

1. `path` identifies the markdown file
2. `heading_path` identifies the structural location inside that file

Example:

```text
path = "blueprint/blueprint.md"
heading_path = "Blueprint/Boundary"
```

Version one must not merge these two surfaces into one combined string.

## 8. Relation to Row Segmentation

`heading_path` is meaningful only if row segmentation remains stable.

That is why version one assumes:

1. root rows may exist
2. heading boundaries are the preferred split points
3. each retrievable row preserves enough ancestry to expose a stable
   `heading_path`

If segmentation discards heading ancestry, `heading_path` stops being a useful
external contract.

## 9. Recommended Authoring Constraints

To keep `heading_path` simple and stable, version one should prefer:

1. unique heading chains within one markdown file
2. headings whose visible text is already readable without slug conversion
3. avoiding `/` inside retrievable heading text

These are authoring preferences for bounded plan work, not a requirement to
invent a heavier escaping system in version one.

## 10. Minimal Accepted Examples

### 10.1 Top-Level Heading

```text
path = "blueprint/blueprint.md"
heading_path = "Blueprint"
title = "Blueprint"
```

### 10.2 Nested Heading

```text
path = "blueprint/blueprint.md"
heading_path = "Blueprint/Boundary"
title = "Boundary"
```

### 10.3 Plan Subsection

```text
path = "plan/plan.md"
heading_path = "Plan/Rust"
title = "Rust"
```

### 10.4 Root Row

```text
path = "plan/plan.md"
heading_path = ""
```

## 11. Rejected Shapes

Version one rejects these patterns as the default external convention:

1. filesystem-like mixes such as `blueprint/blueprint.md#Boundary`
2. reversed ancestry such as `Boundary/Blueprint`
3. slug-only forms such as `blueprint/boundary` when the visible headings are
   `Blueprint` and `Boundary`
4. leading or trailing separators

## 12. Design Rationale

The bounded loop already has:

1. `path` for file identity
2. `heading_path` for structural locality
3. `skeleton` for cheap structural reading
4. `body` for expanded content

The smallest stable convention is therefore:

1. keep `heading_path` human-readable
2. keep it heading-based
3. keep it separate from filesystem paths
4. use one separator and one root-row rule

Anything heavier would turn version one into an escaping and slugging RFC
instead of a minimal retrieval contract.

## 13. Conclusion

Version one `heading_path` should stay simple:

1. `/` is the separator
2. segments come from visible heading text
3. ancestry runs from parent to child
4. root rows use the empty string
5. `heading_path` stays separate from `path` and `title`

That is enough to make bounded markdown retrieval stable without growing the
surface into another parser-specific DSL.
